//! Incremental Merkle tree for tracking deposit commitments.
//!
//! Uses Poseidon over BN254 as the hash function, matching the
//!
//! - The tree initialization pre-fills `roots` to capacity, which causes
//!   `insert()` to immediately fail. Needs a `next_index` counter instead.
//! - `roots` grows unboundedly; consider a circular buffer.
//! - `is_known_root` is O(n); consider a `HashSet`.
use sov_modules_api::{macros::serialize, HexHash};

use crate::hash::poseidon_t3;

pub const ZERO_LEAF: [u8; 32] = [1u8; 32];

/// An incremental (append-only) Merkle tree storing leaf commitments.
#[derive(Clone, Debug, PartialEq, Eq)]
#[serialize(Serde, Borsh)]
#[serde(rename_all = "snake_case")]
pub struct IncrementalMerkleTree {
    pub depth: u8,
    pub zero_value: HexHash,
    pub filled_subtrees: Vec<HexHash>,
    pub roots: Vec<HexHash>,
}

impl IncrementalMerkleTree {
    pub fn new(depth: u8, zero_value: HexHash) -> Self {
        let filled_subtrees = vec![zero_value; depth as usize];
        let roots = vec![zero_value; 1 << depth];
        Self {
            depth,
            zero_value,
            filled_subtrees,
            roots,
        }
    }

    pub fn insert(&mut self, leaf: HexHash) -> Result<u64, &'static str> {
        let mut index = self.roots.len() as u64;
        if index >= (1 << self.depth) {
            return Err("Merkle tree is full");
        }

        let mut current_hash = leaf;
        for level in 0..self.depth {
            if index % 2 == 0 {
                self.filled_subtrees[level as usize] = current_hash;
                current_hash = Self::hash(current_hash, self.zero_value);
            } else {
                current_hash = Self::hash(self.filled_subtrees[level as usize], current_hash);
            }
            index /= 2;
        }
        self.roots.push(current_hash);
        Ok((self.roots.len() - 1) as u64)
    }

    pub fn is_known_root(&self, root: HexHash) -> bool {
        self.roots.contains(&root)
    }

    pub fn last_root(&self) -> HexHash {
        *self.roots.last().unwrap_or(&self.zero_value)
    }

    /// Poseidon hash of two children in the Merkle tree.
    fn hash(a: HexHash, b: HexHash) -> HexHash {
        let x = poseidon_t3(&a.0, &b.0);
        HexHash::from(x)
    }
}
