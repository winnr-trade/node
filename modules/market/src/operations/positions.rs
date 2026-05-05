use crate::error::IntoMarketError;
use crate::{MarketError, MarketModule, PositionKey};
use shared_types::{MarketId, MarketStatus, Size};
use sov_bank::utils::TokenHolder;
use sov_modules_api::{Spec, TxState};

impl<S: Spec> MarketModule<S> {
    /// Add shares to a user's position and keep the user->markets index in sync.
    pub(crate) fn add_position_shares(
        &mut self,
        market_id: MarketId,
        user_address: &S::Address,
        yes_add: Size,
        no_add: Size,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let owner = TokenHolder::User(user_address.clone());
        self.add_position_shares_for_owner(market_id, &owner, yes_add, no_add, state)
    }

    /// Add shares to an arbitrary owner position.
    pub(crate) fn add_position_shares_for_owner(
        &mut self,
        market_id: MarketId,
        owner: &TokenHolder<S>,
        yes_add: Size,
        no_add: Size,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if yes_add.is_zero() && no_add.is_zero() {
            return Ok(());
        }

        let position_key = PositionKey {
            market_id,
            owner: owner.clone(),
        };
        let mut position = self
            .positions
            .get(&position_key, state)
            .into_market_err()?
            .unwrap_or_default();

        position.yes_shares = position
            .yes_shares
            .checked_add(yes_add)
            .ok_or_else(|| anyhow::anyhow!("Overflow in yes_shares"))?;
        position.no_shares = position
            .no_shares
            .checked_add(no_add)
            .ok_or_else(|| anyhow::anyhow!("Overflow in no_shares"))?;

        self.positions
            .set(&position_key, &position, state)
            .into_market_err()?;
        if let TokenHolder::User(user_address) = owner {
            self.add_user_active_market(user_address, market_id, state)?;
        }

        Ok(())
    }

    /// Subtract shares from a user's position and remove index membership once empty.
    pub(crate) fn sub_position_shares(
        &mut self,
        market_id: MarketId,
        user_address: &S::Address,
        yes_sub: Size,
        no_sub: Size,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let owner = TokenHolder::User(user_address.clone());
        self.sub_position_shares_for_owner(market_id, &owner, yes_sub, no_sub, state)
    }

    /// Subtract shares from an arbitrary owner position.
    pub(crate) fn sub_position_shares_for_owner(
        &mut self,
        market_id: MarketId,
        owner: &TokenHolder<S>,
        yes_sub: Size,
        no_sub: Size,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if yes_sub.is_zero() && no_sub.is_zero() {
            return Ok(());
        }

        let position_key = PositionKey {
            market_id,
            owner: owner.clone(),
        };
        let mut position = self
            .positions
            .get(&position_key, state)
            .into_market_err()?
            .ok_or(MarketError::NoPosition { market_id })?;

        if position.yes_shares < yes_sub || position.no_shares < no_sub {
            return Err(MarketError::InsufficientShares {
                required: yes_sub.max(no_sub),
                available_yes: position.yes_shares,
                available_no: position.no_shares,
            });
        }

        position.yes_shares = position.yes_shares.saturating_sub(yes_sub);
        position.no_shares = position.no_shares.saturating_sub(no_sub);

        if position.is_empty() {
            self.positions
                .remove(&position_key, state)
                .into_market_err()?;
            if let TokenHolder::User(user_address) = owner {
                self.remove_user_active_market(user_address, market_id, state)?;
            }
        } else {
            self.positions
                .set(&position_key, &position, state)
                .into_market_err()?;
            if let TokenHolder::User(user_address) = owner {
                self.add_user_active_market(user_address, market_id, state)?;
            }
        }

        Ok(())
    }

    /// Remove a user's full market position and index membership.
    pub(crate) fn remove_position(
        &mut self,
        market_id: MarketId,
        user_address: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let owner = TokenHolder::User(user_address.clone());
        self.remove_position_for_owner(market_id, &owner, state)
    }

    /// Remove an arbitrary owner position and index membership when applicable.
    pub(crate) fn remove_position_for_owner(
        &mut self,
        market_id: MarketId,
        owner: &TokenHolder<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let position_key = PositionKey {
            market_id,
            owner: owner.clone(),
        };
        self.positions
            .remove(&position_key, state)
            .into_market_err()?;

        if let TokenHolder::User(user_address) = owner {
            self.remove_user_active_market(user_address, market_id, state)?;
        }

        Ok(())
    }

    /// Bounded cleanup for a single user's market index.
    ///
    /// Returns the number of removed market IDs.
    pub(crate) fn compact_user_active_markets(
        &mut self,
        user_address: &S::Address,
        max_scan: usize,
        state: &mut impl TxState<S>,
    ) -> Result<u64, MarketError> {
        if max_scan == 0 {
            return Ok(0);
        }

        let market_ids = self
            .user_active_markets
            .get(user_address, state)
            .into_market_err()?
            .unwrap_or_default();

        if market_ids.is_empty() {
            return Ok(0);
        }

        let now_ms = self
            .chain_state
            .get_time(state)
            .into_market_err()?
            .as_millis() as u64;
        let mut removed = 0usize;
        let mut kept = Vec::with_capacity(market_ids.len());

        for market_id in market_ids {
            let stale = self.is_stale_user_active_market(user_address, market_id, now_ms, state)?;
            if stale && removed < max_scan {
                removed += 1;
                continue;
            }
            kept.push(market_id);
        }

        if kept.is_empty() {
            self.user_active_markets
                .remove(user_address, state)
                .into_market_err()?;
        } else {
            self.user_active_markets
                .set(user_address, &kept, state)
                .into_market_err()?;
        }

        Ok(removed as u64)
    }

    fn add_user_active_market(
        &mut self,
        user_address: &S::Address,
        market_id: MarketId,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let mut markets = self
            .user_active_markets
            .get(user_address, state)
            .into_market_err()?
            .unwrap_or_default();

        if !markets.contains(&market_id) {
            markets.push(market_id);
            self.user_active_markets
                .set(user_address, &markets, state)
                .into_market_err()?;
        }

        Ok(())
    }

    fn remove_user_active_market(
        &mut self,
        user_address: &S::Address,
        market_id: MarketId,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let Some(mut markets) = self
            .user_active_markets
            .get(user_address, state)
            .into_market_err()?
        else {
            return Ok(());
        };

        markets.retain(|m| *m != market_id);

        if markets.is_empty() {
            self.user_active_markets
                .remove(user_address, state)
                .into_market_err()?;
        } else {
            self.user_active_markets
                .set(user_address, &markets, state)
                .into_market_err()?;
        }

        Ok(())
    }

    fn is_stale_user_active_market(
        &self,
        user_address: &S::Address,
        market_id: MarketId,
        now_ms: u64,
        state: &mut impl TxState<S>,
    ) -> Result<bool, MarketError> {
        let owner = TokenHolder::User(user_address.clone());
        let position_key = PositionKey { market_id, owner };

        let Some(position) = self.positions.get(&position_key, state).into_market_err()? else {
            return Ok(true);
        };

        if position.is_empty() {
            return Ok(true);
        }

        let Some(market) = self.markets.get(&market_id, state).into_market_err()? else {
            return Ok(true);
        };

        if market.status() != MarketStatus::Active {
            return Ok(true);
        }

        Ok(now_ms >= market.resolution_time)
    }
}
