// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use utxo_spend_lib::{validate_transact, PublicValuesStruct, TransactWitness};

pub fn main() {
    let witness = sp1_zkvm::io::read::<TransactWitness>();
    let public_values = validate_transact(&witness).unwrap_or_else(|err| {
        panic!("UTXO transact validation failed: {err}");
    });
    let bytes = PublicValuesStruct::abi_encode(&public_values.abi_struct());

    sp1_zkvm::io::commit_slice(&bytes);
}
