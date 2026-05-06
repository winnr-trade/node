use crate::error::IntoOrderbookError;
use crate::{OrderbookError, OrderbookModule, UserMarketKey};
use market::{MarketId, PositionKey};
use shared_types::{OutcomeSide, Size};
use sov_bank::utils::TokenHolder;
use sov_bank::{Amount, Coins, IntoPayable, Payable};
use sov_modules_api::{Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    /// Transfer collateral from a user to the module and lock it in accounting
    /// for the given market.
    pub(crate) fn lock_collateral_from(
        &mut self,
        owner: &S::Address,
        market_id: MarketId,
        amount: Amount,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        if amount == Amount::ZERO {
            return Ok(());
        }

        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        self.bank
            .transfer_from(
                owner,
                self.id.to_payable(),
                Coins {
                    amount,
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_orderbook_err()?;

        let key = UserMarketKey {
            address: owner.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        let new_val = current
            .checked_add(amount)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in locked_collateral")))?;

        self.locked_collateral
            .set(&key, &new_val, state)
            .into_orderbook_err()?;

        Ok(())
    }

    /// Unlock collateral from a user's locked bucket and optionally transfer it to a recipient.
    ///
    /// If `recipient` is `None`, collateral is only unlocked from accounting.
    pub(crate) fn unlock_collateral_to(
        &mut self,
        owner: &S::Address,
        recipient: Option<impl Payable<S>>,
        market_id: MarketId,
        amount: Amount,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        if amount == Amount::ZERO {
            return Ok(());
        }

        let key = UserMarketKey {
            address: owner.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let new_val =
            current
                .checked_sub(amount)
                .ok_or(OrderbookError::InsufficientCollateral {
                    required: amount,
                    available: current,
                })?;

        if new_val == Amount::ZERO {
            self.locked_collateral
                .remove(&key, state)
                .into_orderbook_err()?;
        } else {
            self.locked_collateral
                .set(&key, &new_val, state)
                .into_orderbook_err()?;
        }

        if let Some(recipient) = recipient {
            let market = self
                .market
                .markets
                .get(&market_id, state)
                .into_orderbook_err()?
                .ok_or(OrderbookError::MarketNotFound { market_id })?;

            self.bank
                .transfer_from(
                    self.id.to_payable(),
                    recipient,
                    Coins {
                        amount,
                        token_id: market.collateral_token,
                    },
                    state,
                )
                .into_orderbook_err()?;
        }

        Ok(())
    }

    // ========================================================================
    // SHARE RESERVATION (for SELL orders)
    // ========================================================================

    /// Reserve shares for a SELL order of the given outcome.
    pub(crate) fn lock_shares_from(
        &mut self,
        owner: &S::Address,
        market_id: MarketId,
        outcome: OutcomeSide,
        amount: Size,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let position_key = PositionKey {
            market_id,
            owner: TokenHolder::User(owner.clone()),
        };
        let position = self
            .market
            .positions
            .get(&position_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let key = UserMarketKey {
            address: owner.clone(),
            market_id,
        };

        let mut locked_shares = self
            .locked_shares
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let (delta_yes, delta_no): (Size, Size) = match outcome {
            OutcomeSide::Yes => (amount, Size::ZERO),
            OutcomeSide::No => (Size::ZERO, amount),
        };

        let available_shares = match outcome {
            OutcomeSide::Yes => position.yes_shares.saturating_sub(locked_shares.yes),
            OutcomeSide::No => position.no_shares.saturating_sub(locked_shares.no),
        };

        if available_shares < amount {
            return Err(OrderbookError::InsufficientShares {
                required: amount,
                available: available_shares,
            });
        }

        let new_locked_shares_yes = locked_shares
            .yes
            .checked_add(delta_yes)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in locked_shares")))?;

        let new_locked_shares_no = locked_shares
            .no
            .checked_add(delta_no)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in locked_shares")))?;

        locked_shares.yes = new_locked_shares_yes;
        locked_shares.no = new_locked_shares_no;

        self.locked_shares
            .set(&key, &locked_shares, state)
            .into_orderbook_err()?;

        Ok(())
    }

    /// Unreserve shares for the given outcome and optionally transfer them.
    ///
    /// If `recipient` is `None`, shares are only unlocked from accounting.
    /// `to_cost_yes` / `to_cost_no` are forwarded to `transfer_shares_from` as the acquisition
    /// cost for the recipient. Ignored when `recipient` is `None`.
    pub(crate) fn unlock_shares_to(
        &mut self,
        owner: &S::Address,
        recipient: Option<&S::Address>,
        market_id: MarketId,
        outcome: OutcomeSide,
        amount: Size,
        cost_yes: Amount,
        cost_no: Amount,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        if amount.is_zero() {
            return Ok(());
        }

        let key = UserMarketKey {
            address: owner.clone(),
            market_id,
        };

        let mut locked_shares = self
            .locked_shares
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let (delta_yes, delta_no): (Size, Size) = match outcome {
            OutcomeSide::Yes => (amount, Size::ZERO),
            OutcomeSide::No => (Size::ZERO, amount),
        };

        let new_locked_shares_yes =
            locked_shares
                .yes
                .checked_sub(delta_yes)
                .ok_or(OrderbookError::InsufficientShares {
                    required: amount,
                    available: locked_shares.yes,
                })?;
        let new_locked_shares_no =
            locked_shares
                .no
                .checked_sub(delta_no)
                .ok_or(OrderbookError::InsufficientShares {
                    required: amount,
                    available: locked_shares.no,
                })?;

        locked_shares.yes = new_locked_shares_yes;
        locked_shares.no = new_locked_shares_no;

        if locked_shares.yes.is_zero() && locked_shares.no.is_zero() {
            self.locked_shares
                .remove(&key, state)
                .into_orderbook_err()?;
        } else {
            self.locked_shares
                .set(&key, &locked_shares, state)
                .into_orderbook_err()?;
        }

        if let Some(recipient) = recipient {
            self.market
                .transfer_shares_from(
                    market_id, owner, recipient, delta_yes, delta_no, cost_yes, cost_no, state,
                )
                .into_orderbook_err()?;
        }

        Ok(())
    }
}
