//! Incremental Merkle tree for tracking deposit commitments.
//!
//! Uses Poseidon2 over BabyBear as the hash function, matching the
//! ZK circuit in `circuits-lib`.
//!
//! - The tree initialization pre-fills `roots` to capacity, which causes
//!   `insert()` to immediately fail. Needs a `next_index` counter instead.
//! - `roots` grows unboundedly; consider a circular buffer.
//! - `is_known_root` is O(n); consider a `HashSet`.

use circuits_lib::hash::{bytes_to_hash, hash_pair, hash_to_bytes};
use sov_modules_api::{macros::serialize, HexHash};

/// An incremental (append-only) Merkle tree storing leaf commitments.
///
/// FIXME: See module-level docs — the current implementation has known
/// structural issues.
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

    /// Poseidon2 hash of two children in the Merkle tree.
    /// Matches the `hash_pair` function used inside the ZK circuit.
    fn hash(left: HexHash, right: HexHash) -> HexHash {
        let left_hash = bytes_to_hash(&left.0);
        let right_hash = bytes_to_hash(&right.0);
        let result = hash_pair(&left_hash, &right_hash);
        HexHash::from(hash_to_bytes(&result))
    }
}
