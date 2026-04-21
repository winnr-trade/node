//! Pyth Oracle Module
//!
//! Stores and manages Pyth price feed updates. Supports:
//! - Storing price updates keyed by (feed_id, publish_time)
//! - Looking up historical prices by feed and timestamp
//! - Admin-managed Wormhole guardian set for future VAA verification

mod call;
mod error;
mod event;
mod genesis;
pub mod types;

#[cfg(feature = "native")]
mod query;

pub use call::CallMessage;
pub use call::MAX_BYTES_PRICE_UPDATES;
pub use error::PythError;
pub use event::Event;
pub use genesis::PythGenesisConfig;
pub use types::{GuardianSet, PriceFeedKey, PriceUpdate};

use sov_chain_state::ChainState;
use sov_modules_api::{
    Context, GenesisState, Module, ModuleId, ModuleInfo, ModuleRestApi, Spec, StateMap, StateValue,
    TxState,
};

/// Pyth Oracle Module
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct PythModule<S: Spec> {
    /// Module identifier
    #[id]
    pub id: ModuleId,

    /// Admin address for guardian set management
    #[state]
    pub admin: StateValue<S::Address>,

    /// Wormhole guardian set for VAA signature verification
    #[state]
    pub guardian_set: StateValue<GuardianSet>,

    /// Price updates indexed by (feed_id, publish_time)
    #[state]
    pub price_updates: StateMap<PriceFeedKey, PriceUpdate>,

    /// Chain state module for accessing chain information
    #[module]
    pub chain_state: ChainState<S>,
}

impl<S: Spec> Module for PythModule<S> {
    type Spec = S;
    type Config = PythGenesisConfig<S>;
    type CallMessage = CallMessage;
    type Event = Event;
    type Error = PythError;

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
            CallMessage::UpdatePriceFeeds { update_data } => {
                self.update_price_feeds(update_data, state)
            }

            CallMessage::SetGuardianSet { keys, expiry } => {
                self.set_guardian_set(keys, expiry, ctx, state)
            }
        }
    }
}
