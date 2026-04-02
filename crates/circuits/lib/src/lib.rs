//! UTXO-style transaction verification library for SP1 circuits.
//!
//! Provides types and helper functions for verifying:
//! - Knowledge of note commitment preimage
//! - Merkle proof for existence of spent note
//! - Correct nullifier calculation
//! - Correct output commitment calculation
//! - Conservation of value (input value == output value)
//! - Owner address preservation
//!
//! Uses Poseidon2 over BabyBear field for ZK-efficient hashing.

#![no_std]
extern crate alloc;

pub mod hash;
pub mod merkle;
pub mod note;
pub mod types;
pub mod verify;

pub use hash::{bytes_to_hash, derive_address, hash_pair, hash_to_bytes, Hash};
pub use merkle::MerkleProof;
pub use note::Note;
pub use types::{
    Address, PrivateInputs, PublicInputs, PublicValuesStruct, HASH_SIZE, MERKLE_TREE_DEPTH,
};
pub use verify::verify_transaction;
