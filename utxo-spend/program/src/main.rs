// These two lines are necessary for the program to properly compile.
//
// Under the hood, we wrap your main function with some extra code so that it behaves properly
// inside the zkVM.
#![no_main]
sp1_zkvm::entrypoint!(main);

use alloy_sol_types::SolType;
use utxo_spend_lib::{validate_transact, PublicValuesStruct, TransactInputs};

pub fn main() {
    let inputs = sp1_zkvm::io::read::<TransactInputs>();
    let public_values = validate_transact(&inputs).unwrap_or_else(|err| {
        panic!("UTXO transact validation failed: {err}");
    });
    let bytes = PublicValuesStruct::abi_encode(&public_values.abi_struct());

    sp1_zkvm::io::commit_slice(&bytes);
}
