use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use slop_algebra::AbstractField;
use slop_baby_bear::baby_bear_poseidon2::my_bb_16_perm;
use slop_baby_bear::BabyBear;
use slop_symmetric::CryptographicHasher;

use crate::hash::Hash;
use crate::types::{Address, Hasher};

/// Domain separation constants
const COMMITMENT_DOMAIN: u32 = 1;
const NULLIFIER_DOMAIN: u32 = 2;

fn hasher() -> Hasher {
    Hasher::new(my_bb_16_perm())
}

fn bytes_to_felts(bytes: &[u8]) -> Vec<BabyBear> {
    let mut result = Vec::with_capacity((bytes.len() + 2) / 3);
    for chunk in bytes.chunks(3) {
        let mut value: u32 = 0;
        for (i, &byte) in chunk.iter().enumerate() {
            value |= (byte as u32) << (i * 8);
        }
        result.push(BabyBear::from_canonical_u32(value));
    }
    result
}

fn u64_to_felts(value: u64) -> [BabyBear; 3] {
    let b = value.to_le_bytes();
    [
        BabyBear::from_canonical_u32(b[0] as u32 | (b[1] as u32) << 8 | (b[2] as u32) << 16),
        BabyBear::from_canonical_u32(b[3] as u32 | (b[4] as u32) << 8 | (b[5] as u32) << 16),
        BabyBear::from_canonical_u32(b[6] as u32 | (b[7] as u32) << 8),
    ]
}

/// Represents a note in the UTXO system.s
/// A note is the fundamental unit of value, similar to a UTXO in Bitcoin
/// or a note in Zcash.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    /// Owner's address (who can spend this note)
    pub owner: Address,
    /// Value contained in the note
    pub value: u64,
    /// Random salt for hiding the commitment
    pub salt: Hash,
}

impl Note {
    /// Creates a new note with the given parameters.
    pub fn new(owner: Address, value: u64, salt: Hash) -> Self {
        Self { owner, value, salt }
    }

    /// Computes the commitment for this note using Poseidon2.
    /// commitment = Poseidon2(domain || owner || value || salt)
    pub fn commitment(&self) -> Hash {
        Self::compute_commitment(&self.owner, self.value, &self.salt)
    }

    /// Computes a note commitment from individual fields.
    /// commitment = Poseidon2(domain || owner_felts || value_felts || salt)
    pub fn compute_commitment(owner: &Address, value: u64, salt: &Hash) -> Hash {
        let h = hasher();
        let mut input: Vec<BabyBear> = Vec::with_capacity(20);

        input.push(BabyBear::from_canonical_u32(COMMITMENT_DOMAIN));
        input.extend(bytes_to_felts(owner));
        input.extend(u64_to_felts(value));
        input.extend_from_slice(salt);

        h.hash_iter(input)
    }

    /// Computes the nullifier for this note given a spending key.
    /// nullifier = Poseidon2(domain || commitment || spending_key)
    /// The nullifier uniquely identifies a note spend without revealing which note.
    pub fn nullifier(&self, spending_key: &Hash) -> Hash {
        Self::compute_nullifier(&self.commitment(), spending_key)
    }

    /// Computes a nullifier from a commitment and spending key.
    /// nullifier = Poseidon2(domain || commitment || spending_key)
    pub fn compute_nullifier(commitment: &Hash, spending_key: &Hash) -> Hash {
        let h = hasher();
        let mut input: Vec<BabyBear> = Vec::with_capacity(17);

        input.push(BabyBear::from_canonical_u32(NULLIFIER_DOMAIN));
        input.extend_from_slice(commitment);
        input.extend_from_slice(spending_key);

        h.hash_iter(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Hash;
    use crate::types::HASH_SIZE;
    use slop_algebra::{AbstractField, PrimeField32};

    fn zero_hash() -> Hash {
        core::array::from_fn(|_| <BabyBear as AbstractField>::zero())
    }

    fn test_salt(val: u32) -> Hash {
        let mut salt = zero_hash();
        for i in 0..HASH_SIZE {
            salt[i] = BabyBear::from_canonical_u32(val + i as u32);
        }
        salt
    }

    fn test_spending_key(val: u32) -> Hash {
        let mut key = zero_hash();
        for i in 0..HASH_SIZE {
            key[i] = BabyBear::from_canonical_u32(val * 100 + i as u32);
        }
        key
    }

    #[test]
    fn commitment_deterministic() {
        let owner = [1u8; 20];
        let c1 = Note::compute_commitment(&owner, 1000, &test_salt(42));
        let c2 = Note::compute_commitment(&owner, 1000, &test_salt(42));
        assert_eq!(c1, c2);
    }

    #[test]
    fn commitment_different_salt() {
        let owner = [1u8; 20];
        let c1 = Note::compute_commitment(&owner, 1000, &test_salt(42));
        let c2 = Note::compute_commitment(&owner, 1000, &test_salt(43));
        assert_ne!(c1, c2);
    }

    #[test]
    fn commitment_different_value() {
        let owner = [1u8; 20];
        let salt = test_salt(1);
        assert_ne!(
            Note::compute_commitment(&owner, 100, &salt),
            Note::compute_commitment(&owner, 200, &salt),
        );
    }

    #[test]
    fn commitment_different_owner() {
        let salt = test_salt(1);
        assert_ne!(
            Note::compute_commitment(&[1u8; 20], 100, &salt),
            Note::compute_commitment(&[2u8; 20], 100, &salt),
        );
    }

    #[test]
    fn note_commitment_matches_static() {
        let note = Note::new([1u8; 20], 1000, test_salt(42));
        let c1 = note.commitment();
        let c2 = Note::compute_commitment(&[1u8; 20], 1000, &test_salt(42));
        assert_eq!(c1, c2);
    }

    #[test]
    fn nullifier_deterministic() {
        let commitment = test_salt(1);
        let key = test_spending_key(2);
        assert_eq!(
            Note::compute_nullifier(&commitment, &key),
            Note::compute_nullifier(&commitment, &key),
        );
    }

    #[test]
    fn nullifier_different_key() {
        let commitment = test_salt(1);
        assert_ne!(
            Note::compute_nullifier(&commitment, &test_spending_key(2)),
            Note::compute_nullifier(&commitment, &test_spending_key(3)),
        );
    }

    #[test]
    fn nullifier_different_commitment() {
        let key = test_spending_key(1);
        assert_ne!(
            Note::compute_nullifier(&test_salt(1), &key),
            Note::compute_nullifier(&test_salt(2), &key),
        );
    }

    #[test]
    fn note_nullifier_matches_static() {
        let key = test_spending_key(5);
        let note = Note::new([1u8; 20], 500, test_salt(10));
        assert_eq!(
            note.nullifier(&key),
            Note::compute_nullifier(&note.commitment(), &key)
        );
    }

    #[test]
    fn bytes_to_felts_length() {
        assert_eq!(bytes_to_felts(&[0, 1, 2, 3, 4, 5]).len(), 2);
        assert_eq!(bytes_to_felts(&[0]).len(), 1);
        assert_eq!(bytes_to_felts(&[0, 1, 2, 3]).len(), 2);
    }

    #[test]
    fn u64_to_felts_values() {
        let f = u64_to_felts(0);
        assert_eq!(f[0].as_canonical_u32(), 0);
        assert_eq!(f[1].as_canonical_u32(), 0);
        assert_eq!(f[2].as_canonical_u32(), 0);

        let f = u64_to_felts(1);
        assert_eq!(f[0].as_canonical_u32(), 1);
        assert_eq!(f[1].as_canonical_u32(), 0);
        assert_eq!(f[2].as_canonical_u32(), 0);

        // 0xAABBCCDDEEFF0011
        let val: u64 = 0xAABB_CCDD_EEFF_0011;
        let f = u64_to_felts(val);
        // bytes LE: [0x11, 0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA]
        assert_eq!(f[0].as_canonical_u32(), 0xFF0011); // bytes 0-2
        assert_eq!(f[1].as_canonical_u32(), 0xCCDDEE); // bytes 3-5
        assert_eq!(f[2].as_canonical_u32(), 0xAABB); // bytes 6-7

        // u64::MAX should not panic — all elements stay < BabyBear P
        let f = u64_to_felts(u64::MAX);
        assert!(f[0].as_canonical_u32() < 0x78000001);
        assert!(f[1].as_canonical_u32() < 0x78000001);
        assert!(f[2].as_canonical_u32() < 0x78000001);
    }
}
