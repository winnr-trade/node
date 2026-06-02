use crate::types::ProofBytes;
use schemars::JsonSchema;
use sov_bank::Amount;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    HexHash, HexString, SafeVec, Spec,
};

pub(crate) const MAX_MEMO_BYTES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[schemars(bound = "S: Spec", rename = "ShieldedPoolCall")]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case", bound(serialize = "", deserialize = ""))]
pub enum CallMessage<S: Spec> {
    /// Create the very first shielded account entry for `owner`.
    ///
    /// `owner` is the logical account owner; `signature` is an ed25519 signature
    /// over the canonical registration message, proving control of `owner`.
    /// This allows the transaction to be submitted by a relayer (`ctx.sender()`)
    /// while the shielded account is attributed to `owner`.
    RegisterAccount {
        owner: S::Address,
        signature: HexString<[u8; 64]>,
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
    },

    /// Deposit collateral into the shielded pool from `ctx.sender()`.
    ///
    /// The transaction must be signed by the address holding the collateral
    /// (e.g. a stealth address). No separate owner signature is required —
    /// the transaction signature itself is the authorization.
    Deposit {
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
    },

    /// Deposit collateral into the shielded pool via an owner signature.
    ///
    /// `owner` is the address whose tokens are transferred; `signature` is an
    /// ed25519 signature over the canonical deposit message, allowing a relayer
    /// to submit the transaction without holding the user's private key.
    DepositViaSignature {
        owner: S::Address,
        signature: HexString<[u8; 64]>,
        proof: ProofBytes,
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
    },

    /// Withdraw collateral from the shielded pool.
    ///
    /// `owner` proves knowledge of the shielded note; `recipient` is the address
    /// that receives the tokens; `signature` covers both, preventing a relayer
    /// from redirecting funds.
    Withdraw {
        owner: S::Address,
        signature: HexString<[u8; 64]>,
        recipient: S::Address,
        proof: ProofBytes,
        /// Merkle root at the time the proof was generated.
        root: HexHash,
        amount: Amount,
        commitment: HexHash,
        nullifier: HexHash,
        memo: SafeVec<u8, MAX_MEMO_BYTES>,
    },
}
