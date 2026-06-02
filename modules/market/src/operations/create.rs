use crate::error::{IntoMarketError, IntoMarketErrorFlat};
use crate::{Event, Market, MarketError, MarketModule, Resolver};
use shared_types::{MarketId, Size};
use sov_bank::Amount;
use sov_modules_api::{Context, EventEmitter, SafeString, Spec, TxState};
use tracing::info;

impl<S: Spec> MarketModule<S> {
    /// Create a new prediction market
    pub(crate) fn create_market(
        &mut self,
        question: SafeString,
        resolution_time: u64,
        resolver: Resolver<S>,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let config = self.config.get_or_err(state).into_market_err_flat()?;
        let collateral_token = self
            .collateral_token_id
            .get_or_err(state)
            .into_market_err_flat()?;

        // Validate question length
        if question.len() > config.max_question_length {
            return Err(MarketError::QuestionTooLong {
                length: question.len(),
                max_length: config.max_question_length,
            });
        }

        // Validate resolution time
        let current_time = self
            .chain_state
            .get_time(state)
            .into_market_err()?
            .as_millis() as u64;

        if resolution_time <= current_time + config.min_market_duration {
            return Err(MarketError::ResolutionTimeTooSoon {
                resolution_time,
                earliest_allowed: current_time + config.min_market_duration,
            });
        }

        // Generate market ID
        let market_id = MarketId(
            self.next_market_id
                .get_or_err(state)
                .into_market_err_flat()?,
        );
        self.next_market_id
            .set(&(market_id.0 + 1), state)
            .into_market_err()?;

        // Create market
        let market = Market {
            id: market_id,
            question: question.clone(),
            creator: ctx.sender().clone(),
            collateral_token,
            resolution_time,
            halted: false,
            outcome: None,
            resolver: resolver.clone(),
            total_shares: Size::ZERO,
            created_at: current_time,
        };

        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;
        self.market_collateral
            .set(&market_id, &Amount::ZERO, state)
            .into_market_err()?;

        info!(
            market_id = %market_id,
            creator = %ctx.sender(),
            resolver = ?resolver,
            resolution_time = resolution_time,
            "Market created"
        );

        self.emit_event(
            state,
            Event::MarketCreated {
                market_id,
                question,
                creator: ctx.sender().to_string(),
                collateral_token,
                resolution_time,
                resolver: resolver.to_string(),
                timestamp: current_time,
            },
        );

        Ok(())
    }
}
