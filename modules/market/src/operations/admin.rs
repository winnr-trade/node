use crate::error::{IntoMarketError, IntoMarketErrorFlat};
use crate::{Event, MarketError, MarketModule};
use shared_types::MarketId;
use sov_modules_api::{Context, EventEmitter, Spec, TxState};

impl<S: Spec> MarketModule<S> {
    /// Set market halted state (halt/resume)
    pub(crate) fn set_market_status(
        &mut self,
        market_id: MarketId,
        halted: bool,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let admin = self.admin.get_or_err(state).into_market_err_flat()?;

        let mut market = self
            .markets
            .get(&market_id, state)
            .into_market_err()?
            .ok_or(MarketError::MarketNotFound { market_id })?;

        // Only admin can change status
        if *context.sender() != admin {
            return Err(MarketError::Unauthorized {
                action: "change market status".to_string(),
            });
        }

        // Cannot change status of resolved markets
        if market.outcome.is_some() {
            return Err(MarketError::MarketAlreadyResolved { market_id });
        }

        let old_status = market.status();
        market.halted = halted;
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        let timestamp = self.current_time_ms(state)?;
        self.emit_event(
            state,
            Event::MarketStatusChanged {
                market_id,
                old_status,
                new_status: market.status(),
                timestamp,
            },
        );

        Ok(())
    }
}
