use ark_bn254::Bn254;
use ark_groth16::{Groth16 as Groth16Ark, Proof as ProofArk};

type Groth16 = Groth16Ark<Bn254>;
type Proof = ProofArk<Bn254>;

pub fn verify_proof(proof: Proof) -> bool {
    // Groth16::verify_proof(pvk, proof, public_inputs);
    true
}
