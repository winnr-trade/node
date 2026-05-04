use crate::error::{IntoMarketError, IntoMarketErrorFlat};
use crate::{Event, Market, MarketError, MarketModule};
use shared_types::{MarketId, MarketStatus};
use sov_bank::{Amount, Coins, IntoPayable, Payable};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};
use tracing::info;

impl<S: Spec> MarketModule<S> {
    /// Mint YES and NO shares by depositing collateral from the same payable holder.
    pub fn mint_shares_to(
        &mut self,
        market_id: MarketId,
        amount: u64,
        to: impl Payable<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if amount == 0 {
            return Err(MarketError::ZeroAmount);
        }

        let owner = to.as_token_holder().to_owned();
        let mut market = self.get_active_market(market_id, state)?;

        // Transfer collateral from the holder into market module custody.
        self.bank
            .transfer_from(
                to,
                self.id.to_payable(),
                Coins {
                    amount: Amount(amount as u128),
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        // Update market totals.
        market.total_shares = market
            .total_shares
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow in total_shares"))?;
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        // Update collateral tracking.
        let current_collateral = self
            .market_collateral
            .get(&market_id, state)
            .into_market_err()?
            .unwrap_or(0);
        self.market_collateral
            .set(&market_id, &(current_collateral + amount), state)
            .into_market_err()?;

        // Update owner position and accessory index for users.
        self.add_position_shares_for_owner(market_id, &owner, amount, amount, state)?;

        info!(
            market_id = %market_id,
            user = %owner,
            amount = amount,
            "Shares minted"
        );

        self.emit_event(
            state,
            Event::SharesMinted {
                market_id,
                user: owner.to_string(),
                amount,
            },
        );

        Ok(())
    }

    /// Mint YES and NO shares by depositing collateral
    pub(crate) fn mint_shares(
        &mut self,
        market_id: MarketId,
        amount: u64,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        self.mint_shares_to(market_id, amount, ctx.sender(), state)
    }

    /// Burn YES and NO share pairs and redeem collateral to the same payable holder.
    pub fn burn_shares_from(
        &mut self,
        market_id: MarketId,
        amount: u64,
        from: impl Payable<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if amount == 0 {
            return Err(MarketError::ZeroAmount);
        }

        let owner = from.as_token_holder().to_owned();

        let mut market = self
            .markets
            .get_or_err(&market_id, state)
            .into_market_err_flat()?;

        // Can redeem from active or halted markets, not resolved
        if market.outcome.is_some() {
            return Err(MarketError::MarketAlreadyResolved { market_id });
        }

        // Burn shares and keep accessory index in sync.
        self.sub_position_shares_for_owner(market_id, &owner, amount, amount, state)?;

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

        // Transfer collateral back to holder.
        self.bank
            .transfer_from(
                self.id.to_payable(),
                from,
                Coins {
                    amount: Amount(amount as u128),
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        info!(
            market_id = %market_id,
            user = %owner,
            amount = amount,
            "Shares redeemed"
        );

        self.emit_event(
            state,
            Event::SharesRedeemed {
                market_id,
                user: owner.to_string(),
                amount,
            },
        );

        Ok(())
    }

    /// Burn YES and NO share pairs and redeem collateral to the tx sender.
    pub(crate) fn burn_shares(
        &mut self,
        market_id: MarketId,
        amount: u64,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        self.burn_shares_from(market_id, amount, ctx.sender(), state)
    }

    /// Transfer shares from an explicit owner to another owner.
    pub fn transfer_shares_from(
        &mut self,
        market_id: MarketId,
        from: impl Payable<S>,
        to: impl Payable<S>,
        yes_amount: u64,
        no_amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if yes_amount == 0 && no_amount == 0 {
            return Err(MarketError::ZeroAmount);
        }

        let market = self
            .markets
            .get(&market_id, state)
            .into_market_err()?
            .ok_or(MarketError::MarketNotFound { market_id })?;

        if market.outcome.is_some() {
            return Err(MarketError::MarketAlreadyResolved { market_id });
        }

        let from_owner = from.as_token_holder().to_owned();
        let to_owner = to.as_token_holder().to_owned();

        if from_owner == to_owner {
            return Ok(());
        }

        self.sub_position_shares_for_owner(market_id, &from_owner, yes_amount, no_amount, state)?;
        self.add_position_shares_for_owner(market_id, &to_owner, yes_amount, no_amount, state)?;

        self.emit_event(
            state,
            Event::SharesTransferred {
                market_id,
                from: from_owner.to_string(),
                to: to_owner.to_string(),
                yes_amount,
                no_amount,
            },
        );

        Ok(())
    }

    /// Transfer shares from the tx sender to another holder.
    pub(crate) fn transfer_shares(
        &mut self,
        market_id: MarketId,
        to: S::Address,
        yes_amount: u64,
        no_amount: u64,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        self.transfer_shares_from(market_id, ctx.sender(), &to, yes_amount, no_amount, state)
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
