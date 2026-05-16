use ff::PrimeField;
use slop_bn254::{Bn254Fr, FFBn254Fr};

pub type Felt = Bn254Fr;

pub trait FeltExt {
    fn from_u64(value: u64) -> Self
    where
        Self: Sized;

    fn to_bytes32(&self) -> [u8; 32];

    fn is_within_u64(&self) -> bool;
}

impl FeltExt for Felt {
    fn from_u64(value: u64) -> Self {
        Felt {
            value: FFBn254Fr::from(value),
        }
    }

    fn to_bytes32(&self) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(self.value.to_repr().as_ref());
        out.reverse();
        out
    }

    fn is_within_u64(&self) -> bool {
        let repr = self.value.to_repr();
        repr.as_ref()[8..].iter().all(|&b| b == 0)
    }
}
