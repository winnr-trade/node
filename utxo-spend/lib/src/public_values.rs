use alloy_sol_types::sol;

use crate::field::{Felt, FeltExt};

#[derive(Debug, Clone)]
pub struct PublicValues {
    pub merkle_root: Felt,
    pub nullifier: Felt,
    pub output_commitment: Felt,
    pub public_value: Felt,
    pub is_deposit: bool,
}

sol! {
    /// The public values encoded as a struct that can be easily deserialized inside Solidity.
    struct PublicValuesStruct {
        bytes32 merkle_root;
        bytes32 nullifier;
        bytes32 output_commitment;
        bytes32 public_value;
        bool is_deposit;
    }
}

impl PublicValues {
    pub fn abi_struct(&self) -> PublicValuesStruct {
        PublicValuesStruct {
            merkle_root: self.merkle_root.to_bytes32().into(),
            nullifier: self.nullifier.to_bytes32().into(),
            output_commitment: self.output_commitment.to_bytes32().into(),
            public_value: self.public_value.to_bytes32().into(),
            is_deposit: self.is_deposit,
        }
    }
}
