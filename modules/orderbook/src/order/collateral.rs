use crate::error::IntoOrderbookError;
use crate::{OrderbookError, OrderbookModule, UserMarketKey};
use market::{MarketId, PositionKey};
use shared_types::OutcomeSide;
use sov_bank::utils::TokenHolder;
use sov_bank::{Amount, Coins, IntoPayable, Payable};
use sov_modules_api::{Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    /// Lock collateral for a BUY order and transfer it into the module account.
    pub(crate) fn lock_collateral(
        &mut self,
        user: &S::Address,
        market_id: MarketId,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        if amount == 0 {
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
                user,
                self.id.to_payable(),
                Coins {
                    amount: Amount(amount as u128),
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_orderbook_err()?;

        let key = UserMarketKey {
            address: user.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or(0);
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
        locked_owner: &S::Address,
        recipient: Option<impl Payable<S>>,
        market_id: MarketId,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        if amount == 0 {
            return Ok(());
        }

        let key = UserMarketKey {
            address: locked_owner.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or(0);

        let new_val =
            current
                .checked_sub(amount)
                .ok_or(OrderbookError::InsufficientCollateral {
                    required: amount,
                    available: current,
                })?;

        if new_val == 0 {
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
                        amount: Amount(amount as u128),
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
        amount: u64,
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

        let (delta_shares_yes, delta_shares_no) = match outcome {
            OutcomeSide::Yes => (amount, 0),
            OutcomeSide::No => (0, amount),
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
            .checked_add(delta_shares_yes)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in locked_shares")))?;

        let new_locked_shares_no = locked_shares
            .no
            .checked_add(delta_shares_no)
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
    pub(crate) fn unlock_shares_to(
        &mut self,
        owner: &S::Address,
        recipient: Option<&S::Address>,
        market_id: MarketId,
        outcome: OutcomeSide,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        if amount == 0 {
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

        let (delta_shares_yes, delta_shares_no) = match outcome {
            OutcomeSide::Yes => (amount, 0),
            OutcomeSide::No => (0, amount),
        };

        let new_locked_shares_yes = locked_shares.yes.checked_sub(delta_shares_yes).ok_or(
            OrderbookError::InsufficientShares {
                required: amount,
                available: locked_shares.yes,
            },
        )?;
        let new_locked_shares_no = locked_shares.no.checked_sub(delta_shares_no).ok_or(
            OrderbookError::InsufficientShares {
                required: amount,
                available: locked_shares.no,
            },
        )?;

        locked_shares.yes = new_locked_shares_yes;
        locked_shares.no = new_locked_shares_no;

        if locked_shares.yes == 0 && locked_shares.no == 0 {
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
                    market_id,
                    owner,
                    recipient,
                    delta_shares_yes,
                    delta_shares_no,
                    state,
                )
                .into_orderbook_err()?;
        }

        Ok(())
    }
}
