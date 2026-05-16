use slop_bn254::outer_perm;
use slop_symmetric::Permutation;

use crate::field::Felt;

pub fn poseidon2_hash_2(left: Felt, right: Felt) -> Felt {
    let permutation = outer_perm();
    let state = [left, right, Felt::default()];
    permutation.permute(state)[0]
}

pub fn poseidon2_hash_3(a: Felt, b: Felt, c: Felt) -> Felt {
    let permutation = outer_perm();
    let state = [a, b, c];
    permutation.permute(state)[0]
}
