//! Market Module
//!
//! Enables creation and management of binary outcome prediction markets.
//! Users can:
//! - Create markets with a question and resolution time
//! - Mint YES/NO share pairs by depositing collateral
//! - Redeem share pairs back to collateral
//! - Claim winnings after market resolution

mod call;
mod error;
mod event;
mod genesis;
mod types;

#[cfg(feature = "native")]
mod query;
#[cfg(feature = "native")]
pub use query::*;

// #[cfg(test)]
// mod tests;

pub use call::{CallMessage, ResolutionData};
pub use error::MarketError;
pub use event::Event;
pub use genesis::MarketGenesisConfig;
pub use types::*;

// Re-export shared types for convenience
pub use shared_types::{MarketId, MarketStatus, Outcome};

use crate::error::{IntoMarketError, IntoMarketErrorFlat};
use sov_bank::{Amount, Coins, IntoPayable, TokenId};
use sov_modules_api::{
    Context, CredentialId, EventEmitter, GenesisState, Module, ModuleId, ModuleInfo, ModuleRestApi,
    SafeString, Spec, StateMap, StateValue, TxState,
};
use tracing::info;

/// Market Module
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct MarketModule<S: Spec> {
    /// Module identifier
    #[id]
    pub id: ModuleId,

    /// Admin address with elevated permissions
    #[state]
    pub admin: StateValue<S::Address>,

    /// Global market configuration
    #[state]
    pub config: StateValue<MarketConfig>,

    #[state]
    pub supported_collateral_token: StateMap<TokenId, ()>,

    /// Counter for generating unique market IDs
    #[state]
    pub next_market_id: StateValue<u64>,

    /// All markets indexed by MarketId
    #[state]
    pub markets: StateMap<MarketId, Market<S>>,

    /// User positions: (MarketId, Address) -> Position
    #[state]
    pub positions: StateMap<PositionKey<S>, Position>,

    /// Total collateral held by the module per market
    #[state]
    pub market_collateral: StateMap<MarketId, u64>,

    /// Bank module for token operations
    #[module]
    pub bank: sov_bank::Bank<S>,

    /// Chain state module for accessing chain information
    #[module]
    pub chain_state: sov_chain_state::ChainState<S>,
}

impl<S: Spec> Module for MarketModule<S> {
    type Spec = S;
    type Config = MarketGenesisConfig<S>;
    type CallMessage = CallMessage<S>;
    type Event = Event;
    type Error = MarketError;

    fn genesis(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &Self::Config,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.init_module(_header, config, state)
    }

    fn call(
        &mut self,
        msg: Self::CallMessage,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), Self::Error> {
        match msg {
            CallMessage::CreateMarket {
                question,
                collateral_token,
                resolution_time,
                resolver,
            } => self.create_market(
                question,
                collateral_token,
                resolution_time,
                resolver,
                ctx,
                state,
            ),
            CallMessage::MintShares { market_id, amount } => {
                self.mint_shares(market_id, amount, ctx, state)
            }

            CallMessage::RedeemShares { market_id, amount } => {
                self.redeem_shares(market_id, amount, ctx, state)
            }

            CallMessage::ResolveMarket { market_id, data } => {
                self.resolve_market(market_id, data, ctx, state)
            }

            CallMessage::SetSupportedCollateralToken {
                token_id,
                support: supported,
            } => self.set_supported_collateral_token(token_id, supported, ctx, state),

            CallMessage::ClaimWinnings { market_id } => self.claim_winnings(market_id, ctx, state),

            CallMessage::HaltMarket { market_id } => {
                self.set_market_status(market_id, true, ctx, state)
            }

            CallMessage::ResumeMarket { market_id } => {
                self.set_market_status(market_id, false, ctx, state)
            }
        }
    }
}

// ============================================================================
// IMPLEMENTATION
// ============================================================================

impl<S: Spec> MarketModule<S> {
    /// Create a new prediction market
    fn create_market(
        &mut self,
        question: SafeString,
        collateral_token: TokenId,
        resolution_time: u64,
        resolver: Resolver<S>,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let config = self.config.get_or_err(state).into_market_err_flat()?;

        // Validate question length
        if question.len() > config.max_question_length {
            return Err(MarketError::QuestionTooLong {
                length: question.len(),
                max_length: config.max_question_length,
            });
        }

        let token_exists = self
            .bank
            .get_token(&collateral_token, state)
            .is_ok_and(|v| v.is_some());
        if !token_exists {
            return Err(MarketError::UnsupportedCollateralToken {
                token_id: collateral_token,
            });
        }

        let token_supported = self
            .supported_collateral_token
            .get(&collateral_token, state)
            .is_ok_and(|v| v.is_some());
        if !token_supported {
            return Err(MarketError::UnsupportedCollateralToken {
                token_id: collateral_token,
            });
        }

        // Validate resolution time
        let current_time = self
            .chain_state
            .get_time(state)
            .into_market_err()?
            .as_millis() as u64;

        if resolution_time <= current_time + config.min_market_duration {
            return Err(MarketError::ResolutionTimeTooSoon {
                resolution_time,
                earliest_allowed: current_time + config.min_market_duration,
            });
        }

        // Generate market ID
        let market_id = MarketId(
            self.next_market_id
                .get_or_err(state)
                .into_market_err_flat()?,
        );
        self.next_market_id
            .set(&(market_id.0 + 1), state)
            .into_market_err()?;

        // Create market
        let market = Market {
            id: market_id,
            question: question.clone(),
            creator: ctx.sender().clone(),
            collateral_token,
            resolution_time,
            halted: false,
            outcome: None,
            resolver: resolver.clone(),
            total_yes_shares: 0,
            total_no_shares: 0,
            created_at: current_time,
        };

        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;
        self.market_collateral
            .set(&market_id, &0u64, state)
            .into_market_err()?;

        info!(
            market_id = %market_id,
            creator = %ctx.sender(),
            resolver = ?resolver,
            resolution_time = resolution_time,
            "Market created"
        );

        self.emit_event(
            state,
            Event::MarketCreated {
                market_id,
                question,
                creator: ctx.sender().to_string(),
                collateral_token: format!("{:?}", collateral_token),
                resolution_time,
                resolver: resolver.to_string(),
            },
        );

        Ok(())
    }

    /// Mint YES and NO shares by depositing collateral
    fn mint_shares(
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
        market.total_yes_shares = market
            .total_yes_shares
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow in total_yes_shares"))?;
        market.total_no_shares = market
            .total_no_shares
            .checked_add(amount)
            .ok_or_else(|| anyhow::anyhow!("Overflow in total_no_shares"))?;
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
    fn redeem_shares(
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
        // .ok_or(PredictionMarketError::NoPosition { market_id })?;

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
        market.total_yes_shares -= amount;
        market.total_no_shares -= amount;
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

    /// Resolve a market with a final outcome
    fn resolve_market(
        &mut self,
        market_id: MarketId,
        data: call::ResolutionData,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let mut market = self
            .markets
            .get(&market_id, state)
            .into_market_err()?
            .ok_or(MarketError::MarketNotFound { market_id })?;

        // Check resolution time has passed
        let current_time = self
            .chain_state
            .get_time(state)
            .into_market_err()?
            .as_millis() as u64;
        if current_time < market.resolution_time {
            return Err(MarketError::ResolutionTimeTooEarly {
                market_id,
                resolution_time: market.resolution_time,
                current_time,
            });
        }

        // Market must not already be resolved
        if market.outcome.is_some() {
            return Err(MarketError::MarketAlreadyResolved { market_id });
        }

        // Dispatch based on resolver type
        let outcome = match (&market.resolver, &data) {
            (Resolver::Address(resolver_addr), call::ResolutionData::Address { outcome }) => {
                // Verify caller is the designated resolver
                if *context.sender() != *resolver_addr {
                    return Err(MarketError::UnauthorizedResolver {
                        market_id,
                        expected: resolver_addr.to_string(),
                        actual: context.sender().to_string(),
                    });
                }
                *outcome
            }
            (Resolver::Pyth { .. }, call::ResolutionData::Pyth { .. }) => {
                return Err(MarketError::PythResolutionNotImplemented);
            }
            (Resolver::Optimistic {}, _) => {
                return Err(MarketError::OptimisticResolutionNotImplemented);
            }
            // Mismatched resolver type and resolution data
            (resolver, _) => {
                return Err(MarketError::InvalidResolverType {
                    market_id,
                    expected: format!("{:?}", resolver),
                    actual: format!("{:?}", data),
                });
            }
        };

        // Update market
        market.outcome = Some(outcome);
        self.markets
            .set(&market_id, &market, state)
            .into_market_err()?;

        info!(
            market_id = %market_id,
            outcome = ?outcome,
            resolver = %context.sender(),
            "Market resolved"
        );

        self.emit_event(
            state,
            Event::MarketResolved {
                market_id,
                outcome,
                resolver: context.sender().to_string(),
            },
        );

        Ok(())
    }

    /// Claim winnings from a resolved market
    fn claim_winnings(
        &mut self,
        market_id: MarketId,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), MarketError> {
        let market = self
            .markets
            .get(&market_id, state)
            .into_market_err()?
            .ok_or(MarketError::MarketNotFound { market_id })?;

        // Market must be resolved
        let outcome = market
            .outcome
            .ok_or(MarketError::MarketNotResolved { market_id })?;

        // Get user position
        let position_key: PositionKey<S> = PositionKey {
            market_id,
            address: ctx.sender().clone(),
        };
        let position = self
            .positions
            .get(&position_key, state)
            .into_market_err()?
            .ok_or(MarketError::NoPosition { market_id })?;

        // Calculate payout based on outcome
        let (winning_shares, payout) = match outcome {
            Outcome::Yes => (position.yes_shares, position.yes_shares),
            Outcome::No => (position.no_shares, position.no_shares),
            Outcome::Invalid => {
                // Refund: return collateral for complete pairs
                let pairs = position.yes_shares.min(position.no_shares);
                (pairs, pairs)
            }
        };

        if payout == 0 {
            return Err(MarketError::NoWinningsToClaim { market_id });
        }

        // Remove position
        self.positions
            .remove(&position_key, state)
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
                &current_collateral.saturating_sub(payout),
                state,
            )
            .into_market_err()?;

        // Transfer winnings
        self.bank
            .transfer_from(
                self.id.to_payable(),
                ctx.sender(),
                Coins {
                    amount: Amount(payout as u128),
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_market_err()?;

        info!(
            market_id = %market_id,
            user = %ctx.sender(),
            winning_shares = winning_shares,
            payout = payout,
            "Winnings claimed"
        );

        self.emit_event(
            state,
            Event::WinningsClaimed {
                market_id,
                user: ctx.sender().to_string(),
                winning_shares,
                payout,
            },
        );

        Ok(())
    }

    /// Support collateral token for markets
    fn set_supported_collateral_token(
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
    fn set_market_status(
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

    // ========================================================================
    // HELPERS
    // ========================================================================

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
