use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Events emitted by the Pyth module.
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
    /// A price update was stored.
    PriceUpdated {
        feed_id: String,
        price: i64,
        conf: u64,
        expo: i32,
        publish_time: u64,
    },

    /// Guardian set was updated.
    GuardianSetUpdated {
        num_keys: usize,
        expiry: u64,
    },
}
