use schemars::JsonSchema;
use sov_bank::TokenId;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    HexHash, SafeVec,
};

pub const PROOF_SIZE_BYTES: usize = 256;
pub type Proof = SafeVec<u8, PROOF_SIZE_BYTES>;

#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[schemars(rename = "ShieldedPoolCall")]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum CallMessage {
    /// Initial deposit for an address
    ShieldFirst {
        token_id: TokenId,
        amount: u64,
        blinded_address: HexHash,
    },

    /// Deposit collateral into the shielded pool.
    Shield {
        proof: Proof,
        token_id: TokenId,
        amount: u64,
        commitment: HexHash,
        nullifier: HexHash,
    },

    /// Withdraw collateral from the shielded pool.
    UnShield {
        proof: Proof,
        token_id: TokenId,
        amount: u64,
        commitment: HexHash,
        nullifier: HexHash,
    },
}
