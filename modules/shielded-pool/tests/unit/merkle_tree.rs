use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use light_poseidon::{Poseidon, PoseidonHasher};
use shielded_pool::{IncrementalMerkleTree, ZERO_LEAF};
use sov_modules_api::HexHash;

fn hash_pair(a: HexHash, b: HexHash) -> HexHash {
    let mut poseidon = Poseidon::<Fr>::new_circom(2).unwrap();
    let x = Fr::from_be_bytes_mod_order(&a.0);
    let y = Fr::from_be_bytes_mod_order(&b.0);
    let hash = poseidon.hash(&[x, y]).unwrap();

    let mut bytes = [0u8; 32];
    let hash_bytes = hash.into_bigint().to_bytes_be();
    bytes[(32 - hash_bytes.len())..].copy_from_slice(&hash_bytes);
    HexHash::from(bytes)
}

fn level_zeros(depth: u8) -> Vec<HexHash> {
    let mut current = HexHash::from(ZERO_LEAF);
    let mut zeros = Vec::with_capacity(depth as usize);

    for _ in 0..depth {
        zeros.push(current);
        current = hash_pair(current, current);
    }

    zeros
}

#[test]
fn new_prefills_zero_hashes_for_each_level() {
    let depth = 4;
    let zeros = level_zeros(depth);
    let root_zero = hash_pair(zeros[depth as usize - 1], zeros[depth as usize - 1]);

    let tree = IncrementalMerkleTree::new(depth, root_zero);

    assert_eq!(tree.filled_subtrees, zeros);
}

#[test]
fn insert_uses_level_specific_zero_hashes() {
    let depth = 3;
    let zeros = level_zeros(depth);
    let root_zero = hash_pair(zeros[depth as usize - 1], zeros[depth as usize - 1]);

    let leaf = HexHash::from([9u8; 32]);
    let mut expected_root = leaf;
    for zero in &zeros {
        expected_root = hash_pair(expected_root, *zero);
    }

    let mut tree = IncrementalMerkleTree::new(depth, root_zero);
    tree.roots.clear();

    let inserted_index = tree.insert(leaf).expect("insert should succeed");

    assert_eq!(inserted_index, 0);
    assert_eq!(tree.last_root(), expected_root);
}

#[test]
fn last_root_returns_zero_when_roots_are_empty() {
    let depth = 2;
    let root_zero = HexHash::from([7u8; 32]);
    let mut tree = IncrementalMerkleTree::new(depth, root_zero);
    tree.roots.clear();

    assert_eq!(tree.last_root(), root_zero);
}

#[test]
fn is_known_root_reflects_membership() {
    let depth = 2;
    let root_zero = HexHash::from([4u8; 32]);
    let known = HexHash::from([5u8; 32]);
    let unknown = HexHash::from([6u8; 32]);

    let mut tree = IncrementalMerkleTree::new(depth, root_zero);
    tree.roots.clear();
    tree.roots.push(known);

    assert!(tree.is_known_root(known));
    assert!(!tree.is_known_root(unknown));
}

#[test]
fn insert_returns_error_when_tree_is_full() {
    let depth = 2;
    let root_zero = HexHash::from([3u8; 32]);
    let mut tree = IncrementalMerkleTree::new(depth, root_zero);
    tree.roots.clear();
    tree.roots = vec![HexHash::from([8u8; 32]); 1 << depth];

    let result = tree.insert(HexHash::from([9u8; 32]));

    assert_eq!(result, Err("Merkle tree is full"));
}
