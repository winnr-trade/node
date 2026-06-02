use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Events emitted by the agent-wallet module.
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// An agent wallet was registered (or replaced) by an owner.
    AgentRegistered {
        owner: String,
        agent: String,
        scopes: u32,
        expires_at: u64,
        timestamp: u64,
    },

    /// An agent wallet delegation was revoked by the owner.
    AgentRevoked {
        owner: String,
        agent: String,
        timestamp: u64,
    },
}
