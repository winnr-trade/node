use serde::{Deserialize, Serialize};

use crate::field::{Felt, FeltExt};
use crate::hash::{poseidon2_hash_2, poseidon2_hash_3};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub owner: Felt,
    pub value: Felt,
    pub salt: Felt,
}

impl Note {
    pub fn commitment(&self) -> Felt {
        poseidon2_hash_3(self.owner, self.value, self.salt)
    }

    pub fn nullifier(&self, leaf_index: u32) -> Felt {
        let index_felt = Felt::from_u64(leaf_index as u64);
        poseidon2_hash_2(self.commitment(), index_felt)
    }
}
