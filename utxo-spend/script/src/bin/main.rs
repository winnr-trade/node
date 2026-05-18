use alloy_sol_types::SolType;
use clap::Parser;
use sp1_sdk::{include_elf, Elf, ProveRequest, Prover, ProverClient, ProvingKey, SP1Stdin};
use std::time::Instant;
use utxo_spend_lib::{
    build_public_inputs, compute_merkle_root, Felt, FeltExt, MerkleProof, Note, PrivateInputs,
    PublicValuesStruct, TransactInputs, TREE_DEPTH,
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

#[tokio::main]
async fn main() {
    sp1_sdk::utils::setup_logger();
    dotenv::dotenv().ok();
    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }
    let client = ProverClient::from_env().await;
    let mut stdin = SP1Stdin::new();
    let witness = demo_witness(args.n as u64, true);
    stdin.write(&witness);

    println!("private input value: {}", args.n);

    if args.execute {
        tracing::info!("starting execute phase");
        let (output, report) = client.execute(UTXO_TRANSACT_ELF, stdin).await.unwrap();
        tracing::info!("execute phase completed");
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
        let total_start = Instant::now();
        let setup_start = Instant::now();
        tracing::info!("starting setup phase");
        let pk = client
            .setup(UTXO_TRANSACT_ELF)
            .await
            .expect("failed to setup elf");
        let setup_elapsed = setup_start.elapsed();
        tracing::info!("setup phase completed");

        let prove_start = Instant::now();
        tracing::info!("starting prove phase");
        let proof = client
            .prove(&pk, stdin)
            .core()
            .await
            .expect("failed to generate proof");
        let prove_elapsed = prove_start.elapsed();
        let proof_size_bytes = proof.bytes().len();
        tracing::info!("prove phase completed");

        println!("Successfully generated proof!");
        println!("Proof system: Core");
        println!("Proof size: {} bytes", proof_size_bytes);
        println!("Setup only: {:.3}s", setup_elapsed.as_secs_f64());
        println!("Proof generation only: {:.3}s", prove_elapsed.as_secs_f64());

        // Verify the proof.
        let verify_start = Instant::now();
        tracing::info!("starting verify phase");
        client
            .verify(&proof, pk.verifying_key(), None)
            .expect("failed to verify proof");
        let verify_elapsed = verify_start.elapsed();
        let total_elapsed = total_start.elapsed();
        tracing::info!("verify phase completed");

        println!("Successfully verified proof!");
        println!("Verify only: {:.3}s", verify_elapsed.as_secs_f64());
        println!(
            "End-to-end (prove path): {:.3}s",
            total_elapsed.as_secs_f64()
        );
    }
}

fn demo_witness(private_input_value: u64, is_deposit: bool) -> TransactInputs {
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

    TransactInputs { private, public }
}
