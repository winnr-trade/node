use crate::error::{IntoMarketError, IntoMarketErrorFlat};
use crate::{Event, Market, MarketError, MarketModule, PositionUpdateSource};
use shared_types::{MarketId, MarketStatus, Price, Size};
use sov_bank::{Amount, Coins, IntoPayable, Payable};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};
use tracing::info;

impl<S: Spec> MarketModule<S> {
    /// Mint outcome shares by transferring collateral from user to market module
    /// and updating user share position accounting. Only allowed for active markets.
    pub fn mint_shares_to(
        &mut self,
        market_id: MarketId,
        quantity: Size,
        to: impl Payable<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if quantity.is_zero() {
            return Err(MarketError::ZeroAmount);
        }

        let owner = to.as_token_holder().to_owned();
        let mut market = self.get_active_market(market_id, state)?;

        // Transfer collateral from the holder into market module custody.
        let collateral_amount = Price::ONE.cost(quantity, &market.collateral_token);
        self.bank
            .transfer_from(
                to,
                self.id.to_payable(),
                Coins {
                    amount: collateral_amount,
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        // Update market totals.
        market.total_shares = market
            .total_shares
            .checked_add(quantity)
            .ok_or_else(|| anyhow::anyhow!("Overflow in total_shares"))?;
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        // Update collateral tracking.
        let current_collateral = self
            .market_collateral
            .get(&market_id, state)
            .into_market_err()?
            .unwrap_or(Amount::ZERO);
        let new_collateral = current_collateral
            .checked_add(collateral_amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow in market_collateral"))?;
        self.market_collateral
            .set(&market_id, &new_collateral, state)
            .into_market_err()?;

        let cost_per_side = Amount(collateral_amount.0 / 2);

        // Update owner position and accessory index for users.
        self.add_position_shares_to(
            market_id,
            &owner,
            quantity,
            quantity,
            cost_per_side,
            cost_per_side,
            PositionUpdateSource::Mint,
            state,
        )?;

        info!(
            market_id = %market_id,
            user = %owner,
            amount = %quantity,
            "Shares minted"
        );

        self.emit_event(
            state,
            Event::SharesMinted {
                market_id,
                user: owner.to_string(),
                amount: quantity,
            },
        );

        Ok(())
    }

    /// Burn YES and NO share pairs and redeem collateral to the same payable holder.
    pub fn burn_shares_from(
        &mut self,
        market_id: MarketId,
        amount: Size,
        from: impl Payable<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if amount.is_zero() {
            return Err(MarketError::ZeroAmount);
        }

        let owner = from.as_token_holder().to_owned();

        let mut market = self
            .markets
            .get_or_err(&market_id, state)
            .into_market_err_flat()?;
        let collateral_amount = Price::ONE.cost(amount, &market.collateral_token);

        // Can redeem from active or halted markets, not resolved
        if market.outcome.is_some() {
            return Err(MarketError::MarketAlreadyResolved { market_id });
        }

        // Burn shares and keep accessory index in sync.
        self.sub_position_shares_from(
            market_id,
            &owner,
            amount,
            amount,
            PositionUpdateSource::Burn,
            state,
        )?;

        // Update market totals
        market.total_shares = market.total_shares.saturating_sub(amount);
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        // Update collateral tracking
        let current_collateral = self
            .market_collateral
            .get(&market_id, state)
            .into_market_err()?
            .unwrap_or(Amount::ZERO);
        let new_collateral = current_collateral
            .checked_sub(collateral_amount)
            .ok_or_else(|| anyhow::anyhow!("Underflow in market_collateral"))?;
        self.market_collateral
            .set(&market_id, &new_collateral, state)
            .into_market_err()?;

        // Transfer collateral back to holder.
        self.bank
            .transfer_from(
                self.id.to_payable(),
                from,
                Coins {
                    amount: collateral_amount,
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        info!(
            market_id = %market_id,
            user = %owner,
            amount = %amount,
            "Shares redeemed"
        );

        self.emit_event(
            state,
            Event::SharesBurned {
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
        amount: Size,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        self.mint_shares_to(market_id, amount, ctx.sender(), state)
    }

    /// Burn YES and NO share pairs and redeem collateral to the tx sender.
    pub(crate) fn burn_shares(
        &mut self,
        market_id: MarketId,
        amount: Size,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        self.burn_shares_from(market_id, amount, ctx.sender(), state)
    }

    /// Transfer shares from an explicit owner to another owner.
    ///
    /// `to_cost_yes` / `to_cost_no` are the collateral amounts paid by `to` for the acquired
    /// shares (e.g. `fill.price.cost(qty, token)` for a YES acquisition). Pass `Amount::ZERO`
    /// when `to` is a module address — `PositionUpdated` is not emitted for modules.
    pub fn transfer_shares_from(
        &mut self,
        market_id: MarketId,
        from: impl Payable<S>,
        to: impl Payable<S>,
        quantity_yes: Size,
        quantity_no: Size,
        cost_yes: Amount,
        cost_no: Amount,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if quantity_yes.is_zero() && quantity_no.is_zero() {
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

        self.sub_position_shares_from(
            market_id,
            &from_owner,
            quantity_yes,
            quantity_no,
            PositionUpdateSource::Trade,
            state,
        )?;
        self.add_position_shares_to(
            market_id,
            &to_owner,
            quantity_yes,
            quantity_no,
            cost_yes,
            cost_no,
            PositionUpdateSource::Trade,
            state,
        )?;

        self.emit_event(
            state,
            Event::SharesTransferred {
                market_id,
                from: from_owner.to_string(),
                to: to_owner.to_string(),
                yes_amount: quantity_yes,
                no_amount: quantity_no,
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
