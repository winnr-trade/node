//! Genesis configuration for the prediction market module.

use crate::{types::MarketConfig, MarketModule};
use serde::{Deserialize, Serialize};
use sov_modules_api::{GenesisState, Spec};
use tracing::info;

/// Genesis configuration for the prediction market module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketGenesisConfig<S: Spec> {
    /// Admin address with elevated permissions.
    pub admin: S::Address,
    /// Market configuration parameters.
    pub config: MarketConfig,
}

impl<S: Spec> MarketModule<S> {
    pub fn init_module(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &MarketGenesisConfig<S>,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.admin.set(&config.admin, state)?;
        self.config.set(&config.config, state)?;
        self.next_market_id.set(&0u64, state)?;

        info!(
            admin = %config.admin,
            "Prediction market module initialized"
        );
        Ok(())
    }
}
