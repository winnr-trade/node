use alloy_sol_types::SolType;
use clap::Parser;
use sp1_sdk::{
    blocking::{ProveRequest, Prover, ProverClient},
    include_elf, Elf, ProvingKey, SP1Stdin,
};
use utxo_spend_lib::{
    build_public_inputs, compute_merkle_root, Felt, FeltExt, MerkleProof, Note, PrivateInputs,
    PublicValuesStruct, TransactWitness, TREE_DEPTH,
};

/// The ELF (executable and linkable format) file for the Succinct RISC-V zkVM.
const UTXO_TRANSACT_ELF: Elf = include_elf!("utxo-spend-program");

/// The arguments for the command.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long, default_value = "20")]
    n: u32,
}

fn main() {
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }
    let client = ProverClient::from_env();
    let mut stdin = SP1Stdin::new();
    let witness = demo_witness(args.n as u64, true);
    stdin.write(&witness);

    println!("private input value: {}", args.n);

    if args.execute {
        let (output, report) = client.execute(UTXO_TRANSACT_ELF, stdin).run().unwrap();
        println!("Program executed successfully.");

        let decoded = PublicValuesStruct::abi_decode(output.as_slice()).unwrap();
        println!("merkle_root: 0x{}", hex::encode(decoded.merkle_root));
        println!("nullifier: 0x{}", hex::encode(decoded.nullifier));
        println!(
            "output_commitment: 0x{}",
            hex::encode(decoded.output_commitment)
        );
        println!("public_value: 0x{}", hex::encode(decoded.public_value));
        println!("is_deposit: {}", decoded.is_deposit);
        println!("Number of cycles: {}", report.total_instruction_count());
    } else {
        let pk = client
            .setup(UTXO_TRANSACT_ELF)
            .expect("failed to setup elf");
        let proof = client
            .prove(&pk, stdin)
            .run()
            .expect("failed to generate proof");

        println!("Successfully generated proof!");

        // Verify the proof.
        client
            .verify(&proof, pk.verifying_key(), None)
            .expect("failed to verify proof");
        println!("Successfully verified proof!");
    }
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

    let proof = MerkleProof {
        siblings: vec![Felt::from_u64(0); TREE_DEPTH],
        index_bits: vec![false; TREE_DEPTH],
    };

    let input_commitment = input_note.commitment();
    let merkle_root =
        compute_merkle_root(input_commitment, &proof).expect("fixed-depth demo merkle proof");

    let private = PrivateInputs {
        input_note,
        output_note,
        merkle_proof: proof,
    };

    let public = build_public_inputs(&private, merkle_root, public_value, is_deposit)
        .expect("demo public input construction should succeed");

    TransactWitness { private, public }
}
