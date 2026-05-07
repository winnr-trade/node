use crate::error::{IntoMarketError, IntoMarketErrorFlat};
use crate::{Event, MarketError, MarketModule};
use shared_types::MarketId;
use sov_bank::TokenId;
use sov_modules_api::{Context, EventEmitter, Spec, TxState};
use tracing::info;

impl<S: Spec> MarketModule<S> {
    /// Support collateral token for markets
    pub(crate) fn set_supported_collateral_token(
        &mut self,
        token_id: TokenId,
        support: bool,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let admin = self.admin.get_or_err(state).into_market_err_flat()?;
        if *context.sender() != admin {
            return Err(MarketError::Unauthorized {
                action: "add supported collateral token".to_string(),
            });
        }
        // Check token exists in bank
        let token_exists = self
            .bank
            .get_token(&token_id, state)
            .is_ok_and(|v| v.is_some());
        if !token_exists {
            return Err(MarketError::UnsupportedCollateralToken { token_id });
        }

        if support {
            self.supported_collateral_token
                .set(&token_id, &(), state)
                .into_market_err()?;
        } else {
            self.supported_collateral_token
                .remove(&token_id, state)
                .into_market_err()?;
        }

        info!(
            token_id = format!("{:?}", token_id),
            admin = %context.sender(),
            "Supported collateral token added"
        );

        Ok(())
    }

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

        self.emit_event(
            state,
            Event::MarketStatusChanged {
                market_id,
                old_status,
                new_status: market.status(),
            },
        );

        Ok(())
    }
}
