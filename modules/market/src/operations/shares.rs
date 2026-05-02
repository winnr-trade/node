use crate::error::{IntoMarketError, IntoMarketErrorFlat};
use crate::{Event, Market, MarketError, MarketModule, PositionKey};
use shared_types::{MarketId, MarketStatus};
use sov_bank::{Amount, Coins, IntoPayable};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};
use tracing::info;

impl<S: Spec> MarketModule<S> {
    /// Mint YES and NO shares by depositing collateral
    pub(crate) fn mint_shares(
        &mut self,
        market_id: MarketId,
        amount: u64,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if amount == 0 {
            return Err(MarketError::ZeroAmount);
        }

        let mut market = self.get_active_market(market_id, state)?;

        // Transfer collateral from user
        self.bank
            .transfer_from(
                ctx.sender(),
                self.id.to_payable(),
                Coins {
                    amount: Amount(amount as u128),
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        // Update market totals
        market.total_shares = market
            .total_shares
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow in total_shares"))?;
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        // Update collateral tracking
        let current_collateral = self
            .market_collateral
            .get(&market_id, state)
            .into_market_err()?
            .unwrap_or(0);
        self.market_collateral
            .set(&market_id, &(current_collateral + amount), state)
            .into_market_err()?;

        // Update user position
        let position_key: PositionKey<S> = PositionKey {
            market_id,
            address: ctx.sender().clone(),
        };
        let mut position = self
            .positions
            .get(&position_key, state)
            .into_market_err()?
            .unwrap_or_default();
        position.yes_shares = position
            .yes_shares
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow in yes_shares"))?;
        position.no_shares = position
            .no_shares
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow in no_shares"))?;
        self.positions
            .set(&position_key, &position, state)
            .into_market_err()?;

        info!(
            market_id = %market_id,
            user = %ctx.sender(),
            amount = amount,
            "Shares minted"
        );

        self.emit_event(
            state,
            Event::SharesMinted {
                market_id,
                user: ctx.sender().to_string(),
                collateral_amount: amount,
                yes_shares: amount,
                no_shares: amount,
            },
        );

        Ok(())
    }

    /// Redeem pairs of YES and NO shares for collateral
    pub(crate) fn redeem_shares(
        &mut self,
        market_id: MarketId,
        amount: u64,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if amount == 0 {
            return Err(MarketError::ZeroAmount);
        }

        let mut market = self
            .markets
            .get_or_err(&market_id, state)
            .into_market_err_flat()?;

        // Can redeem from active or halted markets, not resolved
        if market.outcome.is_some() {
            return Err(MarketError::MarketAlreadyResolved { market_id });
        }

        // Get and validate user position
        let position_key: PositionKey<S> = PositionKey {
            market_id,
            address: ctx.sender().clone(),
        };
        let mut position = self
            .positions
            .get_or_err(&position_key, state)
            .into_market_err_flat()?;

        if position.yes_shares < amount || position.no_shares < amount {
            return Err(MarketError::InsufficientShares {
                required: amount,
                available_yes: position.yes_shares,
                available_no: position.no_shares,
            });
        }

        // Burn shares
        position.yes_shares -= amount;
        position.no_shares -= amount;

        if position.is_empty() {
            self.positions
                .remove(&position_key, state)
                .into_market_err()?;
        } else {
            self.positions
                .set(&position_key, &position, state)
                .into_market_err()?;
        }

        // Update market totals
        market.total_shares -= amount;
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        // Update collateral tracking
        let current_collateral = self
            .market_collateral
            .get(&market_id, state)
            .into_market_err()?
            .unwrap_or(0);
        self.market_collateral
            .set(
                &market_id,
                &current_collateral.saturating_sub(amount),
                state,
            )
            .into_market_err()?;

        // Transfer collateral back to user
        self.bank
            .transfer_from(
                self.id.to_payable(),
                ctx.sender(),
                Coins {
                    amount: Amount(amount as u128),
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        info!(
            market_id = %market_id,
            user = %ctx.sender(),
            amount = amount,
            "Shares redeemed"
        );

        self.emit_event(
            state,
            Event::SharesRedeemed {
                market_id,
                user: ctx.sender().to_string(),
                yes_shares_burned: amount,
                no_shares_burned: amount,
                collateral_returned: amount,
            },
        );

        Ok(())
    }

    /// Get a market and verify it's active
    fn get_active_market(
        &self,
        market_id: MarketId,
        state: &mut impl TxState<S>,
    ) -> Result<Market<S>, MarketError> {
        let market = self
            .markets
            .get(&market_id, state)
            .into_market_err()?
            .ok_or(MarketError::MarketNotFound { market_id })?;

        if market.status() != MarketStatus::Active {
            return Err(MarketError::MarketNotActive {
                market_id,
                status: market.status(),
            });
        }

        Ok(market)
    }
}
