use crate::error::IntoMarketError;
use crate::{Event, MarketError, MarketModule, PositionKey, PositionUpdateSource};
use shared_types::{MarketId, MarketStatus, Size};
use sov_bank::{utils::TokenHolder, Amount};
use sov_modules_api::{EventEmitter, Spec, TxState};

impl<S: Spec> MarketModule<S> {
    /// Add shares to a user's position and keep the user->markets index in sync.
    pub(crate) fn add_position_shares(
        &mut self,
        market_id: MarketId,
        user_address: &S::Address,
        delta_yes: Size,
        delta_no: Size,
        cost_yes: Amount,
        cost_no: Amount,
        reason: PositionUpdateSource,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let owner = TokenHolder::User(user_address.clone());
        self.add_position_shares_to(
            market_id, &owner, delta_yes, delta_no, cost_yes, cost_no, reason, state,
        )
    }

    /// Add shares to an arbitrary owner position.
    ///
    /// Emits `PositionUpdated` only for `TokenHolder::User` owners.
    pub(crate) fn add_position_shares_to(
        &mut self,
        market_id: MarketId,
        to: &TokenHolder<S>,
        delta_yes: Size,
        delta_no: Size,
        cost_yes: Amount,
        cost_no: Amount,
        reason: PositionUpdateSource,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if delta_yes.is_zero() && delta_no.is_zero() {
            return Ok(());
        }

        let position_key = PositionKey {
            market_id,
            owner: to.clone(),
        };
        let mut position = self
            .positions
            .get(&position_key, state)
            .into_market_err()?
            .unwrap_or_default();

        position.yes_shares = position
            .yes_shares
            .checked_add(delta_yes)
            .ok_or_else(|| anyhow::anyhow!("Overflow in yes_shares"))?;
        position.no_shares = position
            .no_shares
            .checked_add(delta_no)
            .ok_or_else(|| anyhow::anyhow!("Overflow in no_shares"))?;

        self.positions
            .set(&position_key, &position, state)
            .into_market_err()?;

        if let TokenHolder::User(user_address) = to {
            self.add_user_active_market(user_address, market_id, state)?;
            self.emit_event(
                state,
                Event::PositionUpdated {
                    market_id,
                    user_address: user_address.to_string(),
                    yes_delta: delta_yes.0 as i64,
                    no_delta: delta_no.0 as i64,
                    cost_yes_added: cost_yes,
                    cost_no_added: cost_no,
                    update_source: reason,
                },
            );
        }

        Ok(())
    }

    /// Subtract shares from a user's position and remove index membership once empty.
    pub(crate) fn sub_position_shares(
        &mut self,
        market_id: MarketId,
        user_address: &S::Address,
        delta_yes: Size,
        delta_no: Size,
        reason: PositionUpdateSource,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let owner = TokenHolder::User(user_address.clone());
        self.sub_position_shares_from(market_id, &owner, delta_yes, delta_no, reason, state)
    }

    /// Subtract shares from an arbitrary owner position.
    ///
    /// Emits `PositionUpdated` only for `TokenHolder::User` owners.
    /// Cost reduction for disposals is handled by the indexer (proportional to current position).
    pub(crate) fn sub_position_shares_from(
        &mut self,
        market_id: MarketId,
        from: &TokenHolder<S>,
        delta_yes: Size,
        delta_no: Size,
        reason: PositionUpdateSource,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        if delta_yes.is_zero() && delta_no.is_zero() {
            return Ok(());
        }

        let position_key = PositionKey {
            market_id,
            owner: from.clone(),
        };
        let mut position = self
            .positions
            .get(&position_key, state)
            .into_market_err()?
            .ok_or(MarketError::NoPosition { market_id })?;

        if position.yes_shares < delta_yes || position.no_shares < delta_no {
            return Err(MarketError::InsufficientShares {
                required: delta_yes.max(delta_no),
                available_yes: position.yes_shares,
                available_no: position.no_shares,
            });
        }

        position.yes_shares = position.yes_shares.saturating_sub(delta_yes);
        position.no_shares = position.no_shares.saturating_sub(delta_no);

        if position.is_empty() {
            self.positions
                .remove(&position_key, state)
                .into_market_err()?;
            if let TokenHolder::User(user_address) = from {
                self.remove_user_active_market(user_address, market_id, state)?;
                self.emit_event(
                    state,
                    Event::PositionUpdated {
                        market_id,
                        user_address: user_address.to_string(),
                        yes_delta: -(delta_yes.0 as i64),
                        no_delta: -(delta_no.0 as i64),
                        cost_yes_added: Amount::ZERO,
                        cost_no_added: Amount::ZERO,
                        update_source: reason,
                    },
                );
            }
        } else {
            self.positions
                .set(&position_key, &position, state)
                .into_market_err()?;
            if let TokenHolder::User(user_address) = from {
                self.add_user_active_market(user_address, market_id, state)?;
                self.emit_event(
                    state,
                    Event::PositionUpdated {
                        market_id,
                        user_address: user_address.to_string(),
                        yes_delta: -(delta_yes.0 as i64),
                        no_delta: -(delta_no.0 as i64),
                        cost_yes_added: Amount::ZERO,
                        cost_no_added: Amount::ZERO,
                        update_source: reason,
                    },
                );
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
        self.remove_position_from(market_id, &owner, state)
    }

    /// Remove an arbitrary owner position and index membership when applicable.
    pub(crate) fn remove_position_from(
        &mut self,
        market_id: MarketId,
        from: &TokenHolder<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let position_key = PositionKey {
            market_id,
            owner: from.clone(),
        };
        self.positions
            .remove(&position_key, state)
            .into_market_err()?;

        if let TokenHolder::User(user_address) = from {
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
