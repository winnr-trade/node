//! Core types for the agent-wallet module.

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sov_modules_api::Spec;
use std::str::FromStr;

// ============================================================================
// Scope flags
// ============================================================================

/// Bitmask flag: agent may place orders.
pub const SCOPE_PLACE_ORDER: u32 = 1 << 31;

/// Bitmask flag: agent may cancel a single order.
pub const SCOPE_CANCEL_ORDER: u32 = 1 << 30;

/// Bitmask flag: agent may cancel all orders for a market.
pub const SCOPE_CANCEL_ALL_ORDERS: u32 = 1 << 29;

/// All valid scope bits combined. Used for validation.
pub const SCOPE_ALL_VALID: u32 = SCOPE_PLACE_ORDER | SCOPE_CANCEL_ORDER | SCOPE_CANCEL_ALL_ORDERS;

// ============================================================================
// Composite key: (owner, agent)
// ============================================================================

/// StateMap key for a delegation: owner address + agent address.
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct OwnerAgentKey<S: Spec> {
    pub owner: S::Address,
    pub agent: S::Address,
}

impl<S: Spec> core::fmt::Display for OwnerAgentKey<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.owner, self.agent)
    }
}

impl<S: Spec> FromStr for OwnerAgentKey<S> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid OwnerAgentKey format; expected 'owner:agent'");
        }
        let owner = S::Address::from_str(parts[0]).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let agent = S::Address::from_str(parts[1]).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(OwnerAgentKey { owner, agent })
    }
}

// ============================================================================
// Agent policy
// ============================================================================

/// Delegation policy stored for an (owner, agent) pair.
///
/// Revocation is represented by deleting the entry from state — no status field needed.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct AgentPolicy {
    /// Bitmask of allowed scopes (see `SCOPE_*` constants).
    pub scopes: u32,
    /// Expiry in milliseconds since epoch. `0` means no expiry.
    pub expires_at: u64,
}
