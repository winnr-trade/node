use crate::error::IntoOrderbookError;
use crate::order::canonical::CanonicalOrder;
use crate::{MarketSideKey, OrderRequest, OrderbookError, OrderbookModule};
use market::{MarketId, MarketStatus};
use shared_types::{OrderId, OrderType, Price, Side};
use sov_modules_api::{Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    pub(crate) fn validate_order(
        &self,
        order_request: &OrderRequest,
        canonical_order: &CanonicalOrder,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let OrderRequest {
            market_id,
            outcome,
            side,
            price,
            quantity,
            order_type,
        } = order_request;

        if *order_type != OrderType::Market && !canonical_order.price.is_valid() {
            return Err(OrderbookError::InvalidPrice {
                price: canonical_order.price.0,
            });
        }

        if *quantity == 0 {
            return Err(OrderbookError::ZeroQuantity);
        }

        let config = self
            .config
            .get(state)
            .into_orderbook_err()?
            .unwrap_or_default();

        if *quantity < config.min_order_size {
            return Err(OrderbookError::OrderTooSmall {
                size: *quantity,
                minimum: config.min_order_size,
            });
        }

        // Verify market exists and is active
        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound {
                market_id: *market_id,
            })?;

        if market.status() != MarketStatus::Active {
            return Err(OrderbookError::MarketNotActive {
                market_id: *market_id,
                status: format!("{:?}", market.status()),
            });
        }

        Ok(())
    }

    /// Determine whether remaining quantity should rest on the book after matching.
    pub(crate) fn should_post(
        order_type: &OrderType,
        total_filled: u64,
        remaining: u64,
        quantity: u64,
    ) -> Result<bool, OrderbookError> {
        match order_type {
            OrderType::Limit => Ok(true),
            OrderType::PostOnly => Ok(total_filled == 0),
            OrderType::ImmediateOrCancel | OrderType::Market => Ok(false),
            OrderType::FillOrKill if remaining > 0 => Err(OrderbookError::FillOrKillNotFilled {
                requested: quantity,
                available: total_filled,
            }),
            OrderType::FillOrKill => Ok(false),
        }
    }

    /// Check if a canonical order would match immediately.
    pub(crate) fn would_match(
        &self,
        market_id: MarketId,
        canonical_side: Side,
        canonical_price: Price,
        state: &mut impl TxState<S>,
    ) -> Result<bool, OrderbookError> {
        match canonical_side {
            Side::Bid => {
                if let Some(best_ask) = self.best_ask.get(&market_id, state).into_orderbook_err()? {
                    return Ok(canonical_price >= best_ask);
                }
            }
            Side::Ask => {
                if let Some(best_bid) = self.best_bid.get(&market_id, state).into_orderbook_err()? {
                    return Ok(canonical_price <= best_bid);
                }
            }
        }

        Ok(false)
    }

    pub(crate) fn next_order_id(
        &mut self,
        state: &mut impl TxState<S>,
    ) -> Result<OrderId, OrderbookError> {
        let id = self
            .next_order_id
            .get(state)
            .into_orderbook_err()?
            .ok_or_else(|| anyhow::anyhow!("Module not initialized"))?;
        self.next_order_id
            .set(&(id + 1), state)
            .into_orderbook_err()?;
        Ok(OrderId(id))
    }
}
