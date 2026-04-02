use alloc::vec::Vec;
use slop_algebra::{AbstractField, PrimeField32};
use slop_baby_bear::baby_bear_poseidon2::my_bb_16_perm;
use slop_baby_bear::BabyBear;
use slop_symmetric::CryptographicHasher;

use crate::types::{Address, Hasher, HASH_SIZE};

/// Hash type: 8 BabyBear field elements (~248 bits of security)
pub type Hash = [BabyBear; HASH_SIZE];

/// Domain separation constants
const ADDRESS_DOMAIN: u32 = 3;
const MERKLE_DOMAIN: u32 = 4;

/// Get the global Poseidon2 hasher instance
fn hasher() -> Hasher {
    Hasher::new(my_bb_16_perm())
}

/// Convert hash to 32 bytes for serialization/public output
pub fn hash_to_bytes(hash: &Hash) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    for (i, felt) in hash.iter().enumerate() {
        let val = felt.as_canonical_u32();
        bytes[i * 4..i * 4 + 4].copy_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// Convert 32 bytes to hash (for deserialization).
/// The bytes must have been produced by `hash_to_bytes` — each 4-byte chunk
/// must represent a canonical BabyBear value (< P = 0x78000001).
pub fn bytes_to_hash(bytes: &[u8; 32]) -> Hash {
    let mut hash: Hash = core::array::from_fn(|_| <BabyBear as AbstractField>::zero());
    for (i, chunk) in bytes.chunks_exact(4).enumerate() {
        let val = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        assert!(
            val < 0x78000001,
            "bytes_to_hash: value exceeds BabyBear modulus"
        );
        hash[i] = BabyBear::from_canonical_u32(val);
    }
    hash
}

/// Derives the owner address from a spending key using Poseidon2.
/// address = first 20 bytes of hash(domain || spending_key)
pub fn derive_address(spending_key: &Hash) -> Address {
    let h = hasher();
    let mut input: Vec<BabyBear> = Vec::with_capacity(9);

    // Domain separator
    input.push(BabyBear::from_canonical_u32(ADDRESS_DOMAIN));

    // Spending key (8 field elements)
    input.extend_from_slice(spending_key);

    let hash = h.hash_iter(input);
    let hash_bytes = hash_to_bytes(&hash);

    let mut address = [0u8; 20];
    address.copy_from_slice(&hash_bytes[..20]);
    address
}

/// Hashes two sibling nodes in the Merkle tree using Poseidon2.
/// parent = Poseidon2(domain || left || right)
pub fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let h = hasher();
    let mut input: Vec<BabyBear> = Vec::with_capacity(17);

    // Domain separator
    input.push(BabyBear::from_canonical_u32(MERKLE_DOMAIN));

    // Left child (8 field elements)
    input.extend_from_slice(left);

    // Right child (8 field elements)
    input.extend_from_slice(right);

    h.hash_iter(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use slop_algebra::AbstractField;

    fn zero_hash() -> Hash {
        core::array::from_fn(|_| <BabyBear as AbstractField>::zero())
    }

    fn test_hash(val: u32) -> Hash {
        let mut h = zero_hash();
        for i in 0..HASH_SIZE {
            h[i] = BabyBear::from_canonical_u32(val * 100 + i as u32);
        }
        h
    }

    #[test]
    fn hash_roundtrip() {
        let hash = test_hash(123);
        let bytes = hash_to_bytes(&hash);
        let recovered = bytes_to_hash(&bytes);
        for i in 0..HASH_SIZE {
            assert_eq!(hash[i].as_canonical_u32(), recovered[i].as_canonical_u32());
        }
    }

    #[test]
    fn zero_hash_roundtrip() {
        let hash = zero_hash();
        let bytes = hash_to_bytes(&hash);
        assert_eq!(bytes, [0u8; 32]);
        let recovered = bytes_to_hash(&bytes);
        for i in 0..HASH_SIZE {
            assert_eq!(recovered[i].as_canonical_u32(), 0);
        }
    }

    #[test]
    fn derive_address_deterministic() {
        let key = test_hash(42);
        let addr1 = derive_address(&key);
        let addr2 = derive_address(&key);
        assert_eq!(addr1, addr2);
        assert_eq!(addr1.len(), 20);
    }

    #[test]
    fn derive_address_different_keys() {
        let addr1 = derive_address(&test_hash(1));
        let addr2 = derive_address(&test_hash(2));
        assert_ne!(addr1, addr2);
    }

    #[test]
    fn hash_pair_deterministic() {
        let left = test_hash(1);
        let right = test_hash(2);
        let h1 = hash_pair(&left, &right);
        let h2 = hash_pair(&left, &right);
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_pair_order_matters() {
        let a = test_hash(1);
        let b = test_hash(2);
        assert_ne!(hash_pair(&a, &b), hash_pair(&b, &a));
    }

    #[test]
    fn hash_pair_different_inputs() {
        let a = test_hash(1);
        let b = test_hash(2);
        let c = test_hash(3);
        assert_ne!(hash_pair(&a, &b), hash_pair(&a, &c));
    }
}
