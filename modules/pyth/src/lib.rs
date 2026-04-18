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
pub use error::PythError;
pub use event::Event;
pub use genesis::PythGenesisConfig;
pub use types::{GuardianSet, PriceFeedKey, PriceUpdate};

use crate::error::IntoPythError;
use sov_modules_api::{
    Context, GenesisState, HexHash, Module, ModuleId, ModuleInfo, ModuleRestApi, Spec, StateMap,
    StateValue, TxState,
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
    pub chain_state: sov_chain_state::ChainState<S>,
}

impl<S: Spec> PythModule<S> {
    /// Look up a price update by feed ID and publish timestamp.
    pub fn get_price(
        &self,
        feed_id: &HexHash,
        publish_time: u64,
        state: &mut impl TxState<S>,
    ) -> Result<Option<PriceUpdate>, PythError> {
        let key = PriceFeedKey {
            feed_id: feed_id.clone(),
            publish_time,
        };
        self.price_updates.get(&key, state).into_pyth_err()
    }
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
            CallMessage::UpdatePriceFeeds { updates } => self.update_price_feeds(updates, state),
            CallMessage::SetGuardianSet { keys, expiry } => {
                self.set_guardian_set(keys, expiry, ctx, state)
            }
        }
    }
}
