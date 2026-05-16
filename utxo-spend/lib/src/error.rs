#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactValidationError {
    ValueOutOfRange,
    PublicValueOutOfRange,
    OwnerChanged,
    MerkleDepthMismatch,
    InvalidMerklePath,
    InvalidNullifier,
    InvalidOutputCommitment,
    ValueConservationFailed,
}

impl core::fmt::Display for TransactValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ValueOutOfRange => write!(f, "private value exceeds allowed u64 range"),
            Self::PublicValueOutOfRange => write!(f, "public value exceeds allowed u64 range"),
            Self::OwnerChanged => write!(f, "output owner must match input owner"),
            Self::MerkleDepthMismatch => write!(f, "merkle proof depth does not match tree depth"),
            Self::InvalidMerklePath => write!(f, "invalid merkle path for note commitment"),
            Self::InvalidNullifier => write!(f, "invalid nullifier computation"),
            Self::InvalidOutputCommitment => write!(f, "invalid output commitment computation"),
            Self::ValueConservationFailed => {
                write!(f, "input/output/public values do not satisfy conservation")
            }
        }
    }
}

pub type TransactValidationResult<T> = Result<T, TransactValidationError>;
