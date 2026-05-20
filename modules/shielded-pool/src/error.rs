use sov_modules_api::{err_detail, ErrorContext, ErrorDetail, HexHash};

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "error_code", rename_all = "snake_case")]
pub enum ShieldedPoolError {
    #[error("User '{sender}' is unauthorized, only the admin can set the value.")]
    Unauthorized { sender: String },

    #[error("Invalid nullifier: {nullifier:?} has already been used.")]
    DuplicateNullifier { nullifier: HexHash },

    #[error("Duplicate commitment: {commitment:?} has already been committed.")]
    DuplicateCommitment { commitment: HexHash },

    #[error("Already shielded: User has already made a deposit.")]
    AlreadyShielded,

    #[error("unknown Merkle root: {root:?}")]
    UnknownRoot { root: HexHash },

    #[error("invalid proof")]
    InvalidProof,

    #[error("proof serialization error: {message}")]
    Serialization { message: String },

    #[error("proof verification failed: {message}")]
    VerificationFailed { message: String },

    // So we can still wrap anyhow::Error for unexpected errors
    // State getters/setters often use anyhow
    #[error(transparent)]
    #[serde(serialize_with = "serialize_anyhow")]
    Any(#[from] anyhow::Error),
}

fn serialize_anyhow<S>(err: &anyhow::Error, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&err.to_string())
}

/// Extension trait to convert any error to ShieldedPoolError
pub trait IntoShieldedPoolError<T> {
    fn into_shielded_pool_err(self) -> Result<T, ShieldedPoolError>;
}

impl<T, E: std::fmt::Display> IntoShieldedPoolError<T> for Result<T, E> {
    fn into_shielded_pool_err(self) -> Result<T, ShieldedPoolError> {
        self.map_err(|e| ShieldedPoolError::Any(anyhow::anyhow!("{}", e)))
    }
}

impl ErrorDetail for ShieldedPoolError {
    fn error_detail(&self) -> Result<ErrorContext, Box<dyn std::error::Error + Send + Sync>> {
        // Serializes `ShieldedPoolError` to a JSON object
        let mut detail = err_detail!(self);
        detail.insert("message".to_owned(), self.to_string().into());
        Ok(detail)
    }
}
