use crate::{error::IntoOrderbookError, event::CancelReason, UserMarketKey};
use market::{MarketId, PositionKey};
use shared_types::{OrderId, OrderStatus, OutcomeSide, Side};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};

pub mod book;
pub mod canonical;
pub mod helpers;
pub mod matching;
pub mod placement;

use crate::{Event, OrderbookError, OrderbookModule};

impl<S: Spec> OrderbookModule<S> {
    pub(crate) fn lock_collateral(
        &mut self,
        user: &S::Address,
        market_id: MarketId,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = UserMarketKey {
            address: user.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or(0);
        self.locked_collateral
            .set(&key, &(current + amount), state)
            .into_orderbook_err()?;
        Ok(())
    }

    pub(crate) fn unlock_collateral(
        &mut self,
        user: &S::Address,
        market_id: MarketId,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = UserMarketKey {
            address: user.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or(0);
        let new_val = current.saturating_sub(amount);

        if new_val == 0 {
            self.locked_collateral
                .remove(&key, state)
                .into_orderbook_err()?;
        } else {
            self.locked_collateral
                .set(&key, &new_val, state)
                .into_orderbook_err()?;
        }

        Ok(())
    }

    // ========================================================================
    // SHARE RESERVATION (for SELL orders)
    // ========================================================================

    /// Reserve shares for a SELL order of the given outcome.
    pub(crate) fn lock_shares(
        &mut self,
        user: &S::Address,
        market_id: MarketId,
        outcome: OutcomeSide,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let position_key = PositionKey {
            market_id,
            address: user.clone(),
        };
        let position = self
            .market
            .positions
            .get(&position_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let key = UserMarketKey {
            address: user.clone(),
            market_id,
        };

        let mut locked_shares = self
            .locked_shares
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let (current_locked, current_owned) = match outcome {
            OutcomeSide::Yes => (locked_shares.yes, position.yes_shares),
            OutcomeSide::No => (locked_shares.no, position.no_shares),
        };

        let available = current_owned.saturating_sub(current_locked);
        if available < amount {
            return Err(OrderbookError::InsufficientShares {
                required: amount,
                available,
            });
        }

        let new_locked = current_locked
            .checked_add(amount)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in locked_shares")))?;

        match outcome {
            OutcomeSide::Yes => locked_shares.yes = new_locked,
            OutcomeSide::No => locked_shares.no = new_locked,
        }

        self.locked_shares
            .set(&key, &locked_shares, state)
            .into_orderbook_err()?;

        Ok(())
    }

    /// Unreserve shares for the given outcome (on fill or cancel).
    pub(crate) fn unlock_shares(
        &mut self,
        user: &S::Address,
        market_id: MarketId,
        outcome: OutcomeSide,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = UserMarketKey {
            address: user.clone(),
            market_id,
        };

        let mut locked_shares = self
            .locked_shares
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let current_shares = match outcome {
            OutcomeSide::Yes => locked_shares.yes,
            OutcomeSide::No => locked_shares.no,
        };

        let new_val = current_shares.saturating_sub(amount);

        match outcome {
            OutcomeSide::Yes => locked_shares.yes = new_val,
            OutcomeSide::No => locked_shares.no = new_val,
        }

        if locked_shares.yes == 0 && locked_shares.no == 0 {
            self.locked_shares
                .remove(&key, state)
                .into_orderbook_err()?;
        } else {
            self.locked_shares
                .set(&key, &locked_shares, state)
                .into_orderbook_err()?;
        }

        Ok(())
    }
}
