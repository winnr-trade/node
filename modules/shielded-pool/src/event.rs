//! Events emitted by the shielded pool module.
//!
//! **Note:** Event fields should be carefully chosen to avoid leaking private
//! information. In a production shielded pool, events would typically only
//! contain commitments and nullifiers — not user addresses or amounts.

use schemars::JsonSchema;
use sov_modules_api::macros::serialize;

// TODO: Revise event fields for privacy — emitting user + amount defeats
// the purpose of a shielded pool. Consider emitting only commitment hashes.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum ShieldedPoolEvent {
    Deposit { user: String, amount: u64 },
    Withdrawal { user: String, amount: u64 },
}
