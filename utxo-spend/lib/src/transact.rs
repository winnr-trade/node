use serde::{Deserialize, Serialize};

use crate::field::{Felt, FeltExt};
use crate::merkle::{compute_merkle_root, index_from_bits, MerkleProof, TREE_DEPTH};
use crate::note::Note;
use crate::public_values::PublicValues;
use crate::{TransactValidationError, TransactValidationResult};

pub const MERKLE_TREE_DEPTH: usize = TREE_DEPTH;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateInputs {
    pub input_note: Note,
    pub output_note: Note,
    pub merkle_proof: MerkleProof,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicInputs {
    pub merkle_root: Felt,
    pub nullifier: Felt,
    pub output_commitment: Felt,
    pub public_value: Felt,
    pub is_deposit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactInputs {
    pub private: PrivateInputs,
    pub public: PublicInputs,
}

pub fn verify_transaction(
    private: &PrivateInputs,
    public: &PublicInputs,
) -> TransactValidationResult<()> {
    if private.merkle_proof.siblings.len() != TREE_DEPTH
        || private.merkle_proof.index_bits.len() != TREE_DEPTH
    {
        return Err(TransactValidationError::MerkleDepthMismatch);
    }

    if private.output_note.owner != private.input_note.owner {
        return Err(TransactValidationError::OwnerChanged);
    }

    if !private.input_note.value.is_within_u64() || !private.output_note.value.is_within_u64() {
        return Err(TransactValidationError::ValueOutOfRange);
    }

    if !public.public_value.is_within_u64() {
        return Err(TransactValidationError::PublicValueOutOfRange);
    }

    let leaf_index = index_from_bits(&private.merkle_proof.index_bits)
        .ok_or(TransactValidationError::MerkleDepthMismatch)?;

    let input_commitment = private.input_note.commitment();
    let output_commitment = private.output_note.commitment();

    let computed_root = compute_merkle_root(input_commitment, &private.merkle_proof)
        .ok_or(TransactValidationError::MerkleDepthMismatch)?;
    if computed_root != public.merkle_root {
        return Err(TransactValidationError::InvalidMerklePath);
    }

    let expected_nullifier = private.input_note.nullifier(leaf_index);
    if expected_nullifier != public.nullifier {
        return Err(TransactValidationError::InvalidNullifier);
    }

    if output_commitment != public.output_commitment {
        return Err(TransactValidationError::InvalidOutputCommitment);
    }

    let conservation_ok = if public.is_deposit {
        private.input_note.value + public.public_value == private.output_note.value
    } else {
        private.output_note.value + public.public_value == private.input_note.value
    };
    if !conservation_ok {
        return Err(TransactValidationError::ValueConservationFailed);
    }

    Ok(())
}

pub fn validate_transact(inputs: &TransactInputs) -> TransactValidationResult<PublicValues> {
    verify_transaction(&inputs.private, &inputs.public)?;

    Ok(PublicValues {
        merkle_root: inputs.public.merkle_root,
        nullifier: inputs.public.nullifier,
        output_commitment: inputs.public.output_commitment,
        public_value: inputs.public.public_value,
        is_deposit: inputs.public.is_deposit,
    })
}

pub fn build_public_inputs(
    private: &PrivateInputs,
    merkle_root: Felt,
    public_value: Felt,
    is_deposit: bool,
) -> TransactValidationResult<PublicInputs> {
    if private.merkle_proof.siblings.len() != TREE_DEPTH
        || private.merkle_proof.index_bits.len() != TREE_DEPTH
    {
        return Err(TransactValidationError::MerkleDepthMismatch);
    }

    let leaf_index = index_from_bits(&private.merkle_proof.index_bits)
        .ok_or(TransactValidationError::MerkleDepthMismatch)?;

    Ok(PublicInputs {
        merkle_root,
        nullifier: private.input_note.nullifier(leaf_index),
        output_commitment: private.output_note.commitment(),
        public_value,
        is_deposit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::{Felt, FeltExt};

    fn zero_proof() -> MerkleProof {
        MerkleProof {
            siblings: vec![Felt::from_u64(0); MERKLE_TREE_DEPTH],
            index_bits: vec![false; MERKLE_TREE_DEPTH],
        }
    }

    fn valid_transfer() -> (PrivateInputs, PublicInputs) {
        let owner = Felt::from_u64(42);

        let input_note = Note {
            owner,
            value: Felt::from_u64(1000),
            salt: Felt::from_u64(1),
        };
        let output_note = Note {
            owner,
            value: Felt::from_u64(900),
            salt: Felt::from_u64(2),
        };

        let merkle_proof = zero_proof();
        let input_commitment = input_note.commitment();
        let merkle_root = compute_merkle_root(input_commitment, &merkle_proof).unwrap();

        let private = PrivateInputs {
            input_note,
            output_note,
            merkle_proof,
        };
        let public =
            build_public_inputs(&private, merkle_root, Felt::from_u64(100), false).unwrap();
        (private, public)
    }

    #[test]
    fn valid_transfer_succeeds() {
        let (private, public) = valid_transfer();
        assert!(verify_transaction(&private, &public).is_ok());
    }

    #[test]
    fn invalid_merkle_proof_fails() {
        let (mut private, public) = valid_transfer();
        private.merkle_proof.siblings[0] = Felt::from_u64(999);
        assert_eq!(
            verify_transaction(&private, &public).unwrap_err(),
            TransactValidationError::InvalidMerklePath
        );
    }

    #[test]
    fn owner_change_fails() {
        let (mut private, public) = valid_transfer();
        private.output_note.owner = Felt::from_u64(777);
        assert_eq!(
            verify_transaction(&private, &public).unwrap_err(),
            TransactValidationError::OwnerChanged
        );
    }

    #[test]
    fn invalid_nullifier_fails() {
        let (private, mut public) = valid_transfer();
        public.nullifier = Felt::from_u64(12345);
        assert_eq!(
            verify_transaction(&private, &public).unwrap_err(),
            TransactValidationError::InvalidNullifier
        );
    }

    #[test]
    fn value_conservation_fails() {
        let (private, mut public) = valid_transfer();
        public.public_value = Felt::from_u64(10);
        assert_eq!(
            verify_transaction(&private, &public).unwrap_err(),
            TransactValidationError::ValueConservationFailed
        );
    }
}
