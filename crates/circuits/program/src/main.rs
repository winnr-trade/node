//! SP1 guest program for UTXO transaction verification.
//!
//! This program runs inside the SP1 zkVM to generate proofs that a UTXO
//! transaction is valid: correct commitments, valid Merkle proof, correct
//! nullifier, value conservation, and owner authorization.

#![no_main]

sp1_zkvm::entrypoint!(main);

pub fn main() {
    use alloy_sol_types::SolType;
    use circuits_lib::{
        hash_to_bytes, verify_transaction, PrivateInputs, PublicInputs, PublicValuesStruct,
    };

    // Read inputs from the prover (host)
    let private: PrivateInputs = sp1_zkvm::io::read();
    let public: PublicInputs = sp1_zkvm::io::read();

    // Verify all UTXO constraints
    verify_transaction(&private, &public).expect("Transaction verification failed");

    // Encode public values for on-chain verification
    let bytes = PublicValuesStruct::abi_encode(&PublicValuesStruct {
        merkleRoot: hash_to_bytes(&public.merkle_root).into(),
        nullifier: hash_to_bytes(&public.nullifier).into(),
        outputCommitment: hash_to_bytes(&public.output_commitment).into(),
        publicValueDelta: public.public_value_delta,
    });

    // Commit public values to the proof
    sp1_zkvm::io::commit_slice(&bytes);
}
