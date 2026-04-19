use crate::call::ResolutionData;
use crate::error::IntoMarketError;
use crate::{Event, MarketError, MarketModule, Resolver};
use shared_types::{MarketId, Outcome};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};
use tracing::info;

impl<S: Spec> MarketModule<S> {
    /// Resolve a market with a final outcome
    pub(crate) fn resolve_market(
        &mut self,
        market_id: MarketId,
        data: ResolutionData,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let mut market = self
            .markets
            .get(&market_id, state)
            .into_market_err()?
            .ok_or(MarketError::MarketNotFound { market_id })?;

        // Check resolution time has passed
        let current_time = self
            .chain_state
            .get_time(state)
            .into_market_err()?
            .as_millis() as u64;
        if current_time < market.resolution_time {
            return Err(MarketError::ResolutionTimeTooEarly {
                market_id,
                resolution_time: market.resolution_time,
                current_time,
            });
        }

        // Market must not already be resolved
        if market.outcome.is_some() {
            return Err(MarketError::MarketAlreadyResolved { market_id });
        }

        // Dispatch based on resolver type
        let outcome = match (&market.resolver, &data) {
            (Resolver::Address(resolver_addr), ResolutionData::Address { outcome }) => {
                self.resolve_by_address(market_id, resolver_addr, *outcome, context)?
            }
            (
                Resolver::Pyth {
                    feed_id,
                    lower_bound,
                    upper_bound,
                },
                ResolutionData::Pyth { publish_time },
            ) => {
                self.resolve_by_pyth(*feed_id, *lower_bound, *upper_bound, *publish_time, state)?
            }
            (Resolver::Optimistic {}, _) => {
                return Err(MarketError::OptimisticResolutionNotImplemented);
            }
            // Mismatched resolver type and resolution data
            (resolver, _) => {
                return Err(MarketError::InvalidResolverType {
                    market_id,
                    expected: format!("{:?}", resolver),
                    actual: format!("{:?}", data),
                });
            }
        };

        // Update market
        market.outcome = Some(outcome);
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        info!(
            market_id = %market_id,
            outcome = ?outcome,
            resolver = %context.sender(),
            "Market resolved"
        );

        self.emit_event(
            state,
            Event::MarketResolved {
                market_id,
                outcome,
                resolver: context.sender().to_string(),
            },
        );

        Ok(())
    }

    /// Resolve by designated address
    fn resolve_by_address(
        &self,
        market_id: MarketId,
        resolver_addr: &S::Address,
        outcome: Outcome,
        context: &Context<S>,
    ) -> Result<Outcome, MarketError> {
        if *context.sender() != *resolver_addr {
            return Err(MarketError::UnauthorizedResolver {
                market_id,
                expected: resolver_addr.to_string(),
                actual: context.sender().to_string(),
            });
        }
        Ok(outcome)
    }

    /// Resolve by Pyth oracle price feed
    fn resolve_by_pyth(
        &self,
        feed_id: sov_modules_api::HexHash,
        lower_bound: Option<u64>,
        upper_bound: Option<u64>,
        publish_time: u64,
        state: &mut impl TxState<S>,
    ) -> Result<Outcome, MarketError> {
        let key = pyth::PriceFeedKey {
            feed_id,
            publish_time,
        };

        let price_update = self
            .pyth
            .price_updates
            .get(&key, state)
            .into_market_err()?
            .ok_or_else(|| MarketError::PythFeedNotFound {
                feed_id: feed_id.to_string(),
                publish_time,
            })?;

        let price = price_update.price;

        // Negative price is always out of u64 bounds
        if price < 0 {
            return Ok(Outcome::No);
        }

        let price_u64 = price as u64;
        let above_lower = lower_bound.map_or(true, |lb| price_u64 >= lb);
        let below_upper = upper_bound.map_or(true, |ub| price_u64 <= ub);
        if above_lower && below_upper {
            Ok(Outcome::Yes)
        } else {
            Ok(Outcome::No)
        }
    }
}
