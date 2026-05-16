use serde::{Deserialize, Serialize};

use crate::field::Felt;
use crate::hash::poseidon2_hash_2;

pub const TREE_DEPTH: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    pub siblings: Vec<Felt>,
    pub index_bits: Vec<bool>,
}

pub fn compute_merkle_root(leaf: Felt, proof: &MerkleProof) -> Option<Felt> {
    if proof.siblings.len() != TREE_DEPTH || proof.index_bits.len() != TREE_DEPTH {
        return None;
    }

    let mut current = leaf;
    for level in 0..TREE_DEPTH {
        let sibling = proof.siblings[level];
        current = if proof.index_bits[level] {
            poseidon2_hash_2(sibling, current)
        } else {
            poseidon2_hash_2(current, sibling)
        };
    }

    Some(current)
}

pub fn index_from_bits(bits: &[bool]) -> Option<u32> {
    if bits.len() != TREE_DEPTH {
        return None;
    }

    let mut index = 0u32;
    for (i, bit) in bits.iter().enumerate() {
        if *bit {
            index |= 1u32 << i;
        }
    }
    Some(index)
}
