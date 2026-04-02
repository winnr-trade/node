//! Genesis configuration for the orderbook module.

use crate::{types::FeeConfig, OrderbookModule};
use serde::{Deserialize, Serialize};
use sov_modules_api::{GenesisState, Spec};
use tracing::info;

/// Genesis configuration for the orderbook module.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderbookGenesisConfig {
    /// Fee configuration.
    pub fee_config: FeeConfig,
}

impl<S: Spec> OrderbookModule<S> {
    pub fn init_module(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &OrderbookGenesisConfig,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.config.set(&config.fee_config, state)?;
        self.next_order_id.set(&1u64, state)?;

        info!(
            maker_fee_bps = config.fee_config.maker_fee_bps,
            taker_fee_bps = config.fee_config.taker_fee_bps,
            "Orderbook module initialized"
        );
        Ok(())
    }
}
