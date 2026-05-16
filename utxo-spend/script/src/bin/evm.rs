use alloy_sol_types::SolType;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, HashableKey, ProvingKey, SP1ProofWithPublicValues, SP1Stdin, SP1VerifyingKey,
};
use std::path::PathBuf;
use utxo_spend_lib::{
    build_public_inputs, compute_merkle_root, Felt, FeltExt, MerkleProof, Note, PrivateInputs,
    PublicValuesStruct, TransactWitness, TREE_DEPTH,
};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
const UTXO_TRANSACT_ELF: Elf = include_elf!("utxo-spend-program");

/// The arguments for the EVM command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct EVMArgs {
    #[arg(long, default_value = "20")]
    private_input_value: u64,

    #[arg(long, default_value_t = true)]
    is_deposit: bool,

    #[arg(long, value_enum, default_value = "groth16")]
    system: ProofSystem,
}

/// Enum representing the available proof systems
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum ProofSystem {
    Plonk,
    Groth16,
}

/// A fixture that can be used to test the verification of SP1 zkVM proofs inside Solidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SP1UtxoTransactProofFixture {
    merkle_root: String,
    nullifier: String,
    output_commitment: String,
    public_value: String,
    is_deposit: bool,
    vkey: String,
    public_values: String,
    proof: String,
}

fn main() {
    sp1_sdk::utils::setup_logger();
    let args = EVMArgs::parse();
    let client = ProverClient::from_env();
    let pk = client
        .setup(UTXO_TRANSACT_ELF)
        .expect("failed to setup elf");
    let mut stdin = SP1Stdin::new();
    stdin.write(&demo_witness(args.private_input_value, args.is_deposit));

    println!("private_input_value: {}", args.private_input_value);
    println!("is_deposit: {}", args.is_deposit);
    println!("Proof System: {:?}", args.system);

    let proof = match args.system {
        ProofSystem::Plonk => client.prove(&pk, stdin).plonk().run(),
        ProofSystem::Groth16 => client.prove(&pk, stdin).groth16().run(),
    }
    .expect("failed to generate proof");

    create_proof_fixture(&proof, pk.verifying_key(), args.system);
}

/// Create a fixture for the given proof.
fn create_proof_fixture(
    proof: &SP1ProofWithPublicValues,
    vk: &SP1VerifyingKey,
    system: ProofSystem,
) {
    let bytes = proof.public_values.as_slice();
    let decoded = PublicValuesStruct::abi_decode(bytes).unwrap();

    let fixture = SP1UtxoTransactProofFixture {
        merkle_root: format!("0x{}", hex::encode(decoded.merkle_root)),
        nullifier: format!("0x{}", hex::encode(decoded.nullifier)),
        output_commitment: format!("0x{}", hex::encode(decoded.output_commitment)),
        public_value: format!("0x{}", hex::encode(decoded.public_value)),
        is_deposit: decoded.is_deposit,
        vkey: vk.bytes32().to_string(),
        public_values: format!("0x{}", hex::encode(bytes)),
        proof: format!("0x{}", hex::encode(proof.bytes())),
    };

    println!("Verification Key: {}", fixture.vkey);
    println!("Public Values: {}", fixture.public_values);
    println!("Proof Bytes: {}", fixture.proof);
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../contracts/src/fixtures");
    std::fs::create_dir_all(&fixture_path).expect("failed to create fixture path");
    std::fs::write(
        fixture_path.join(format!("{:?}-fixture.json", system).to_lowercase()),
        serde_json::to_string_pretty(&fixture).unwrap(),
    )
    .expect("failed to write fixture");
}

fn demo_witness(private_input_value: u64, is_deposit: bool) -> TransactWitness {
    let owner = Felt::from_u64(42);
    let input_value = Felt::from_u64(private_input_value);
    let public_value = Felt::from_u64(10);
    let output_value = if is_deposit {
        input_value + public_value
    } else {
        input_value - public_value
    };

    let input_note = Note {
        owner,
        value: input_value,
        salt: Felt::from_u64(7),
    };

    let output_note = Note {
        owner,
        value: output_value,
        salt: Felt::from_u64(8),
    };

    let merkle_proof = MerkleProof {
        siblings: vec![Felt::from_u64(0); TREE_DEPTH],
        index_bits: vec![false; TREE_DEPTH],
    };

    let input_commitment = input_note.commitment();
    let merkle_root = compute_merkle_root(input_commitment, &merkle_proof)
        .expect("fixed-depth demo merkle proof");

    let private = PrivateInputs {
        input_note,
        output_note,
        merkle_proof,
    };

    let public = build_public_inputs(&private, merkle_root, public_value, is_deposit)
        .expect("demo public input construction should succeed");

    TransactWitness { private, public }
}
