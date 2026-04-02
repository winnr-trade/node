use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};
use slop_baby_bear::baby_bear_poseidon2::{Perm, BABY_BEAR_DIGEST_SIZE};
use slop_symmetric::PaddingFreeSponge;

use crate::hash::Hash;
use crate::merkle::MerkleProof;
use crate::note::Note;

/// Tree depth for the Merkle tree (supports 2^20 = ~1M notes)
pub const MERKLE_TREE_DEPTH: usize = 20;

/// Number of BabyBear elements in a hash digest
pub const HASH_SIZE: usize = BABY_BEAR_DIGEST_SIZE; // 8

/// The Poseidon2 sponge hasher type (state=16, rate=8, output=8)
pub(crate) type Hasher = PaddingFreeSponge<Perm, 16, 8, HASH_SIZE>;

/// 20-byte address type for note ownership
pub type Address = [u8; 20];

sol! {
    /// Public values that will be verified on-chain.
    /// These are the only values visible to the verifier contract.
    struct PublicValuesStruct {
        /// The Merkle root of the note commitment tree
        bytes32 merkleRoot;
        /// Nullifier of the spent input note (prevents double-spending)
        bytes32 nullifier;
        /// Commitment to the new output note
        bytes32 outputCommitment;
        /// Public value delta (positive = deposit, negative = withdraw, 0 = transfer)
        /// For this simple version: input_value - output_value (withdraw amount)
        int64 publicValueDelta;
    }
}

/// Private inputs to the circuit (known only to the prover).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PrivateInputs {
    /// The input note being spent
    pub input_note: Note,
    /// Merkle proof showing input note exists in the tree
    pub merkle_proof: MerkleProof,
    /// Secret key or spending key of the owner
    /// Used to derive the nullifier and prove ownership
    pub spending_key: Hash,
    /// The output note being created
    pub output_note: Note,
}

/// Public inputs to the circuit (visible to everyone).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicInputs {
    /// Merkle root of the commitment tree
    pub merkle_root: Hash,
    /// Nullifier to be published (prevents double-spending)
    pub nullifier: Hash,
    /// Commitment of the output note
    pub output_commitment: Hash,
    /// Public value delta (withdraw amount, 0 for pure transfer)
    pub public_value_delta: i64,
}
