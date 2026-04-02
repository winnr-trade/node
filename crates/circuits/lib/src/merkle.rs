use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::hash::{hash_pair, Hash};

/// Merkle proof for proving inclusion of a note commitment in the tree.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Sibling hashes along the path from leaf to root
    pub siblings: Vec<Hash>,
    /// Path indices (0 = left, 1 = right) indicating position at each level
    pub path_indices: Vec<u8>,
}

impl MerkleProof {
    /// Verifies that the given leaf is included in a tree with the given root.
    pub fn verify(&self, leaf: &Hash, root: &Hash) -> bool {
        if self.siblings.len() != self.path_indices.len() {
            return false;
        }

        let computed_root = self.compute_root(leaf);
        computed_root == *root
    }

    /// Computes the Merkle root from the leaf using the proof.
    pub fn compute_root(&self, leaf: &Hash) -> Hash {
        let mut current = *leaf;

        for (i, sibling) in self.siblings.iter().enumerate() {
            current = if self.path_indices[i] == 0 {
                // Current node is on the left
                hash_pair(&current, sibling)
            } else {
                // Current node is on the right
                hash_pair(sibling, &current)
            };
        }

        current
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;
    use slop_algebra::AbstractField;
    use slop_baby_bear::BabyBear;
    use crate::types::HASH_SIZE;

    fn zero_hash() -> Hash {
        core::array::from_fn(|_| <BabyBear as AbstractField>::zero())
    }

    fn test_hash(val: u32) -> Hash {
        let mut h = zero_hash();
        for i in 0..HASH_SIZE {
            h[i] = BabyBear::from_canonical_u32(val + i as u32);
        }
        h
    }

    fn build_tree(leaf: &Hash, depth: usize) -> (Hash, MerkleProof) {
        let zero = zero_hash();
        let mut siblings = Vec::with_capacity(depth);
        let mut path_indices = Vec::with_capacity(depth);
        let mut current = *leaf;
        for _ in 0..depth {
            siblings.push(zero);
            path_indices.push(0);
            current = hash_pair(&current, &zero);
        }
        (current, MerkleProof { siblings, path_indices })
    }

    #[test]
    fn verify_valid_proof() {
        let leaf = test_hash(42);
        let (root, proof) = build_tree(&leaf, 3);
        assert!(proof.verify(&leaf, &root));
    }

    #[test]
    fn verify_wrong_leaf() {
        let leaf = test_hash(42);
        let (root, proof) = build_tree(&leaf, 3);
        assert!(!proof.verify(&test_hash(99), &root));
    }

    #[test]
    fn verify_wrong_root() {
        let leaf = test_hash(42);
        let (_, proof) = build_tree(&leaf, 3);
        assert!(!proof.verify(&leaf, &test_hash(99)));
    }

    #[test]
    fn verify_mismatched_lengths() {
        let leaf = test_hash(1);
        let proof = MerkleProof {
            siblings: vec![zero_hash(), zero_hash()],
            path_indices: vec![0],
        };
        assert!(!proof.verify(&leaf, &zero_hash()));
    }

    #[test]
    fn compute_root_deterministic() {
        let leaf = test_hash(10);
        let (root, proof) = build_tree(&leaf, 4);
        assert_eq!(proof.compute_root(&leaf), root);
    }

    #[test]
    fn right_path_index() {
        let leaf = test_hash(7);
        let zero = zero_hash();
        let proof = MerkleProof {
            siblings: vec![zero],
            path_indices: vec![1],
        };
        let expected_root = hash_pair(&zero, &leaf);
        assert!(proof.verify(&leaf, &expected_root));
    }
}
