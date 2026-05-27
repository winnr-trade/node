//! Events emitted by the shielded pool module.

use schemars::JsonSchema;
use sov_bank::Amount;
use sov_modules_api::{macros::serialize, HexHash};

/// Discriminates what kind of shielded-pool transaction created this note.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum NoteKind {
    CreateAccount,
    Deposit,
    Withdraw,
}

/// Emitted by every shielded-pool transaction that inserts a new note into the tree.
///
/// The `memo` field carries the caller-supplied encrypted payload and is the
/// primary channel through which the recipient can recover note secrets off-chain.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Note {
        kind: NoteKind,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        leaf_index: u64,
        timestamp: u64,
        memo: Vec<u8>,
    },
}
