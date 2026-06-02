use serde::{Deserialize, Serialize};
use sov_bank::TokenId;
use sov_modules_api::{GenesisState, HexHash, Spec};
use tracing::info;

use crate::{IncrementalMerkleTree, ShieldedPoolModule, ZERO_LEAF};

/// Depth of the Merkle tree — must match the ZK circuit (UI: TREE_DEPTH = 32).
pub const TREE_DEPTH: u8 = 32;

/// Genesis configuration for the shielded pool module.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShieldedPoolGenesisConfig<S: Spec> {
    /// Admin address with elevated permissions.
    pub admin: S::Address,
    /// The single token accepted by this pool.
    pub token_id: TokenId,
}

impl<S: Spec> ShieldedPoolModule<S> {
    pub fn init_module(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &ShieldedPoolGenesisConfig<S>,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.admin.set(&config.admin, state)?;
        self.token_id.set(&config.token_id, state)?;

        let tree = IncrementalMerkleTree::new(TREE_DEPTH, HexHash::from(ZERO_LEAF));
        self.tree.set(&tree, state)?;

        info!(
            admin = %config.admin,
            token_id = ?config.token_id,
            "Shielded pool module initialized"
        );
        Ok(())
    }
}
