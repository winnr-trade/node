use schemars::JsonSchema;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    HexString, Spec,
};

/// Call messages for the agent-wallet module.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[schemars(bound = "S: Spec", rename = "AgentWalletCall")]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case", bound(serialize = "", deserialize = ""))]
pub enum CallMessage<S: Spec> {
    /// Register an agent wallet with the given scopes and optional expiry.
    ///
    /// Owner authorization happens via an off-chain signature over a canonical
    /// human-readable message generated from these fields.
    /// This enables relayed transactions where `ctx.sender()` can be different
    /// from the owner.
    RegisterAgent {
        /// The agent's address
        agent: S::Address,
        /// Bitmask of allowed scopes (see `SCOPE_*` constants in `types`).
        scopes: u32,
        /// Expiry in milliseconds since epoch. Use `0` for no expiry.
        expires_at: u64,
        /// Monotonic nonce for replay protection. Expected sequence starts at 0.
        nonce: u64,
        /// Owner address
        owner: S::Address,
        /// Signature bytes over the canonical human-readable registration message.
        signature: HexString<[u8; 64]>,
    },

    /// Revoke an agent wallet delegation.
    ///
    /// `ctx.sender()` must be the owner who originally registered the agent.
    RevokeAgent {
        /// The agent address to revoke.
        agent: S::Address,
    },
}
