use crate::types::GuardianSet;
use crate::PythModule;
use serde::{Deserialize, Serialize};
use sov_modules_api::{GenesisState, Spec};
use tracing::info;

/// Genesis configuration for the Pyth module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PythGenesisConfig<S: Spec> {
    /// Admin address for guardian set management.
    pub admin: S::Address,
    /// Initial Wormhole guardian set.
    pub guardian_set: GuardianSet,
}

impl<S: Spec> PythModule<S> {
    pub fn init_module(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &PythGenesisConfig<S>,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.admin.set(&config.admin, state)?;
        self.guardian_set.set(&config.guardian_set, state)?;

        info!(
            admin = %config.admin,
            num_guardians = config.guardian_set.keys.len(),
            "Pyth module initialized"
        );
        Ok(())
    }
}
