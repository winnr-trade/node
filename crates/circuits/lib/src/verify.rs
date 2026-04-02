use crate::hash::derive_address;
use crate::note::Note;
use crate::types::{PrivateInputs, PublicInputs, MERKLE_TREE_DEPTH};

/// Verifies all the constraints for a UTXO transaction.
/// Returns the public values that should be committed to.
///
/// This function checks:
/// 1. Input note commitment is correctly computed
/// 2. Merkle proof is valid (input note exists in tree)
/// 3. Nullifier is correctly computed
/// 4. Spending key corresponds to input note owner (authorization)
/// 5. Output commitment is correctly computed
/// 6. Value conservation: input_value = output_value + public_delta
/// 7. Owner preservation: input owner == output owner
///
/// Additional constraints enforced:
/// - No negative values
/// - Merkle proof depth matches expected tree depth
pub fn verify_transaction(
    private: &PrivateInputs,
    public: &PublicInputs,
) -> Result<(), &'static str> {
    // 1. Verify input note commitment
    let input_commitment = private.input_note.commitment();

    // 2. Verify Merkle proof (input note exists in the tree)
    if !private
        .merkle_proof
        .verify(&input_commitment, &public.merkle_root)
    {
        return Err("Invalid Merkle proof: input note not in tree");
    }

    // Verify Merkle proof has correct depth
    if private.merkle_proof.siblings.len() != MERKLE_TREE_DEPTH {
        return Err("Invalid Merkle proof depth");
    }

    // 3. Verify nullifier is correctly computed
    let expected_nullifier = Note::compute_nullifier(&input_commitment, &private.spending_key);
    if expected_nullifier != public.nullifier {
        return Err("Invalid nullifier computation");
    }

    // 4. Verify spending key corresponds to input note owner (authorization)
    let derived_address = derive_address(&private.spending_key);
    if derived_address != private.input_note.owner {
        return Err("Spending key does not match note owner");
    }

    // 5. Verify output commitment is correctly computed
    let expected_output_commitment = private.output_note.commitment();
    if expected_output_commitment != public.output_commitment {
        return Err("Invalid output commitment computation");
    }

    // 6. Verify value conservation
    // input_value = output_value + public_delta
    // For deposits: public_delta < 0 (adding value to system)
    // For withdraws: public_delta > 0 (removing value from system)
    // For transfers: public_delta = 0
    let input_value = private.input_note.value as i64;
    let output_value = private.output_note.value as i64;
    let expected_delta = input_value - output_value;

    if expected_delta != public.public_value_delta {
        return Err("Value conservation violated");
    }

    // 7. Verify owner preservation (same owner for input and output)
    if private.input_note.owner != private.output_note.owner {
        return Err("Owner address must remain the same");
    }

    // Additional safety checks
    // Ensure no underflow occurred (values should be non-negative)
    // This is implicitly checked by using u64 for values

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::{hash_pair, Hash};
    use crate::merkle::MerkleProof;
    use crate::types::HASH_SIZE;
    use alloc::vec::Vec;
    use slop_algebra::AbstractField;
    use slop_baby_bear::BabyBear;

    fn zero_hash() -> Hash {
        core::array::from_fn(|_| <BabyBear as AbstractField>::zero())
    }

    fn test_hash(val: u32) -> Hash {
        let mut h = zero_hash();
        for i in 0..HASH_SIZE {
            h[i] = BabyBear::from_canonical_u32(val + i as u32);
        }
        h
    }

    fn build_tree(leaf: &Hash, depth: usize) -> (Hash, MerkleProof) {
        let zero = zero_hash();
        let mut siblings = Vec::with_capacity(depth);
        let mut path_indices = Vec::with_capacity(depth);
        let mut current = *leaf;
        for _ in 0..depth {
            siblings.push(zero);
            path_indices.push(0);
            current = hash_pair(&current, &zero);
        }
        (
            current,
            MerkleProof {
                siblings,
                path_indices,
            },
        )
    }

    fn valid_transfer() -> (PrivateInputs, PublicInputs) {
        let spending_key = test_hash(42);
        let owner = derive_address(&spending_key);
        let input_note = Note::new(owner, 1000, test_hash(1));
        let commitment = input_note.commitment();
        let (merkle_root, merkle_proof) = build_tree(&commitment, MERKLE_TREE_DEPTH);
        let nullifier = Note::compute_nullifier(&commitment, &spending_key);
        let output_note = Note::new(owner, 1000, test_hash(2));
        let output_commitment = output_note.commitment();

        let private = PrivateInputs {
            input_note,
            merkle_proof,
            spending_key,
            output_note,
        };
        let public = PublicInputs {
            merkle_root,
            nullifier,
            output_commitment,
            public_value_delta: 0,
        };
        (private, public)
    }

    #[test]
    fn valid_transfer_succeeds() {
        let (p, q) = valid_transfer();
        assert!(verify_transaction(&p, &q).is_ok());
    }

    #[test]
    fn valid_withdrawal() {
        let spending_key = test_hash(42);
        let owner = derive_address(&spending_key);
        let input_note = Note::new(owner, 1000, test_hash(1));
        let commitment = input_note.commitment();
        let (merkle_root, merkle_proof) = build_tree(&commitment, MERKLE_TREE_DEPTH);
        let nullifier = Note::compute_nullifier(&commitment, &spending_key);
        let output_note = Note::new(owner, 700, test_hash(2));

        let private = PrivateInputs {
            input_note,
            merkle_proof,
            spending_key,
            output_note: output_note.clone(),
        };
        let public = PublicInputs {
            merkle_root,
            nullifier,
            output_commitment: output_note.commitment(),
            public_value_delta: 300,
        };
        assert!(verify_transaction(&private, &public).is_ok());
    }

    #[test]
    fn invalid_merkle_proof() {
        let (mut p, q) = valid_transfer();
        p.merkle_proof.siblings[0] = test_hash(999);
        assert_eq!(
            verify_transaction(&p, &q).unwrap_err(),
            "Invalid Merkle proof: input note not in tree"
        );
    }

    #[test]
    fn invalid_nullifier() {
        let (p, mut q) = valid_transfer();
        q.nullifier = test_hash(999);
        assert_eq!(
            verify_transaction(&p, &q).unwrap_err(),
            "Invalid nullifier computation"
        );
    }

    #[test]
    fn wrong_spending_key() {
        let real_key = test_hash(42);
        let wrong_key = test_hash(99);
        let owner = derive_address(&real_key);
        let input_note = Note::new(owner, 1000, test_hash(1));
        let commitment = input_note.commitment();
        let (merkle_root, merkle_proof) = build_tree(&commitment, MERKLE_TREE_DEPTH);
        let nullifier = Note::compute_nullifier(&commitment, &wrong_key);
        let output_note = Note::new(owner, 1000, test_hash(2));

        let private = PrivateInputs {
            input_note,
            merkle_proof,
            spending_key: wrong_key,
            output_note: output_note.clone(),
        };
        let public = PublicInputs {
            merkle_root,
            nullifier,
            output_commitment: output_note.commitment(),
            public_value_delta: 0,
        };
        assert_eq!(
            verify_transaction(&private, &public).unwrap_err(),
            "Spending key does not match note owner"
        );
    }

    #[test]
    fn invalid_output_commitment() {
        let (p, mut q) = valid_transfer();
        q.output_commitment = test_hash(999);
        assert_eq!(
            verify_transaction(&p, &q).unwrap_err(),
            "Invalid output commitment computation"
        );
    }

    #[test]
    fn value_conservation_violated() {
        let (p, mut q) = valid_transfer();
        q.public_value_delta = 100;
        assert_eq!(
            verify_transaction(&p, &q).unwrap_err(),
            "Value conservation violated"
        );
    }

    #[test]
    fn different_owners_rejected() {
        let spending_key = test_hash(42);
        let owner = derive_address(&spending_key);
        let input_note = Note::new(owner, 1000, test_hash(1));
        let commitment = input_note.commitment();
        let (merkle_root, merkle_proof) = build_tree(&commitment, MERKLE_TREE_DEPTH);
        let nullifier = Note::compute_nullifier(&commitment, &spending_key);
        let output_note = Note::new([99u8; 20], 1000, test_hash(2));

        let private = PrivateInputs {
            input_note,
            merkle_proof,
            spending_key,
            output_note: output_note.clone(),
        };
        let public = PublicInputs {
            merkle_root,
            nullifier,
            output_commitment: output_note.commitment(),
            public_value_delta: 0,
        };
        assert_eq!(
            verify_transaction(&private, &public).unwrap_err(),
            "Owner address must remain the same"
        );
    }
}
