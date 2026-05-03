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
mod operations;
mod types;

#[cfg(feature = "native")]
mod query;
use pyth::PythModule;
#[cfg(feature = "native")]
pub use query::*;

// #[cfg(test)]
// mod tests;

pub use call::{CallMessage, ResolutionData};
pub use error::MarketError;
pub use event::Event;
pub use genesis::MarketGenesisConfig;
use sov_chain_state::ChainState;
pub use types::*;

// Re-export shared types for convenience
pub use shared_types::{MarketId, MarketStatus, Outcome};

use sov_bank::{Bank, TokenId};
use sov_modules_api::{
    Context, GenesisState, Module, ModuleId, ModuleInfo, ModuleRestApi, Spec, StateMap, StateValue,
    TxState,
};

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

    /// User index: user -> markets where the user has non-zero shares.
    #[state]
    pub user_active_markets: StateMap<S::Address, Vec<MarketId>>,

    /// Total collateral held by the module per market.
    #[state]
    pub market_collateral: StateMap<MarketId, u64>,

    /// Bank module for token operations
    #[module]
    pub bank: Bank<S>,

    /// Chain state module for accessing chain information
    #[module]
    pub chain_state: ChainState<S>,

    /// Pyth oracle module for price feed resolution
    #[module]
    pub pyth: PythModule<S>,
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

            CallMessage::CompactUserActiveMarkets { max_scan } => {
                self.compact_user_active_markets(ctx.sender(), max_scan as usize, state)?;
                Ok(())
            }

            CallMessage::HaltMarket { market_id } => {
                self.set_market_status(market_id, true, ctx, state)
            }

            CallMessage::ResumeMarket { market_id } => {
                self.set_market_status(market_id, false, ctx, state)
            }
        }
    }
}
