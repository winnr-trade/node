use serde::{Deserialize, Serialize};
use sov_modules_api::{GenesisState, Spec};
use tracing::info;

use crate::ShieldedPoolModule;

/// Genesis configuration for the shielded pool module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShieldedPoolGenesisConfig<S: Spec> {
    /// Admin address with elevated permissions.
    pub admin: S::Address,
}

impl<S: Spec> ShieldedPoolModule<S> {
    pub fn init_module(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &ShieldedPoolGenesisConfig<S>,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.admin.set(&config.admin, state)?;

        // TODO: Initialize the incremental Merkle tree here:
        //   let tree = IncrementalMerkleTree::new(depth, zero_value);
        //   self.imt.set(&tree, state)?;

        info!(
            admin = %config.admin,
            "Shielded pool module initialized"
        );
        Ok(())
    }
}
