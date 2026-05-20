use ark_bn254::{Bn254, Fq, Fq2, Fr, G1Affine, G2Affine};
use ark_ff::PrimeField;
use ark_groth16::Proof;
use schemars::JsonSchema;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    HexHash, SafeVec,
};

pub const PROOF_SIZE_BYTES: usize = 256;

/// Raw Groth16 proof bytes: 8 × 32-byte big-endian field elements.
///
/// Byte layout:
///   [0..32]    pi_a.x
///   [32..64]   pi_a.y
///   [64..96]   pi_b.x.c0
///   [96..128]  pi_b.x.c1
///   [128..160] pi_b.y.c0
///   [160..192] pi_b.y.c1
///   [192..224] pi_c.x
///   [224..256] pi_c.y
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
pub struct ProofBytes(pub SafeVec<u8, PROOF_SIZE_BYTES>);

/// Ordered list of public signals for a Groth16/BN254 circuit, expressed as `HexHash` values.
///
/// Each entry maps 1-to-1 to a public signal in the circom circuit (same order as snarkjs `public.json`).
/// Call [`PublicInputs::to_fr_vec`] to get the `Vec<Fr>` expected by [`crate::verifier::verify`].
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
pub struct PublicInputs(pub Vec<HexHash>);

impl PublicInputs {
    /// Convert each `HexHash` to an `Fr` field element (big-endian, reduced mod the scalar field order).
    pub fn to_fr_vec(&self) -> Vec<Fr> {
        self.0
            .iter()
            .map(|h| Fr::from_be_bytes_mod_order(&h.0))
            .collect()
    }
}

impl ProofBytes {
    /// Convert to an arkworks `Proof<Bn254>` ready to pass into [`crate::verifier::verify`].
    ///
    /// Never fails: field elements are reduced mod the curve order and point
    /// validity is not checked here — an invalid proof simply won't verify.
    pub fn to_ark_proof(&self) -> Proof<Bn254> {
        let b: &[u8] = self.0.as_ref();

        let fq = |chunk: &[u8]| {
            let arr: [u8; 32] = chunk.try_into().expect("slice is always 32 bytes");
            Fq::from_be_bytes_mod_order(&arr)
        };

        Proof {
            a: G1Affine::new(fq(&b[0..32]), fq(&b[32..64])),
            b: G2Affine::new(
                Fq2::new(fq(&b[64..96]), fq(&b[96..128])),
                Fq2::new(fq(&b[128..160]), fq(&b[160..192])),
            ),
            c: G1Affine::new(fq(&b[192..224]), fq(&b[224..256])),
        }
    }
}
