use ark_bn254::Fr;
use ark_ff::{BigInteger, PrimeField};
use light_poseidon::{Poseidon, PoseidonHasher};

pub fn poseidon_t3(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut poseidon = Poseidon::<Fr>::new_circom(3).unwrap();

    let x = Fr::from_be_bytes_mod_order(a);
    let y = Fr::from_be_bytes_mod_order(b);
    let hash = poseidon.hash(&[x, y]).unwrap();

    hash.into_bigint()
        .to_bytes_be()
        .as_slice()
        .try_into()
        .expect("Hash output should be 32 bytes")
}

pub fn poseidon_t4(a: &[u8; 32], b: &[u8; 32], c: &[u8; 32]) -> [u8; 32] {
    let mut poseidon = Poseidon::<Fr>::new_circom(3).unwrap();

    let x = Fr::from_be_bytes_mod_order(a);
    let y = Fr::from_be_bytes_mod_order(b);
    let z = Fr::from_be_bytes_mod_order(c);
    let hash = poseidon.hash(&[x, y, z]).unwrap();

    hash.into_bigint()
        .to_bytes_be()
        .as_slice()
        .try_into()
        .expect("Hash output should be 32 bytes")
}
