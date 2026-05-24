use crate::types::ProofBytes;
use schemars::JsonSchema;
use sov_bank::Amount;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    HexHash, SafeVec,
};

pub(crate) const MAX_MEMO_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum CallMessage {
    /// Create the very first entry
    CreateAccount {
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
    },

    /// Deposit collateral into the shielded pool.
    Deposit {
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
    },

    /// Withdraw collateral from the shielded pool.
    Withdraw {
        proof: ProofBytes,
        /// Merkle root at the time the proof was generated.
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
    },
}
