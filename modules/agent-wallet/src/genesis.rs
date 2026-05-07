use crate::AgentWalletModule;
use serde::{Deserialize, Serialize};
use sov_modules_api::{GenesisState, Spec};
use tracing::info;

/// Genesis configuration for the agent-wallet module.
///
/// No initial state is required — agents are registered at runtime by owners.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentWalletGenesisConfig<S: Spec> {
    #[serde(skip)]
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Spec> Default for AgentWalletGenesisConfig<S> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Spec> AgentWalletModule<S> {
    pub fn init_module(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        _config: &AgentWalletGenesisConfig<S>,
        _state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        info!("Agent-wallet module initialized");
        Ok(())
    }
}
