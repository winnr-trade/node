use sov_modules_api::{err_detail, ErrorContext, ErrorDetail};

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "error_code", rename_all = "snake_case")]
pub enum PythError {
    #[error("Invalid price update data: {reason}")]
    InvalidUpdateData { reason: String },

    #[error("Price update verification failed: {reason}")]
    VerificationFailed { reason: String },

    #[error("Price feed not found: {feed_id}")]
    FeedNotFound { feed_id: String },

    #[error("Unauthorized to {action}")]
    Unauthorized { action: String },

    #[error("{0}")]
    #[serde(serialize_with = "serialize_anyhow")]
    Any(#[from] anyhow::Error),
}

fn serialize_anyhow<S>(err: &anyhow::Error, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&err.to_string())
}

impl ErrorDetail for PythError {
    fn error_detail(&self) -> Result<ErrorContext, Box<dyn std::error::Error + Send + Sync>> {
        let mut detail = err_detail!(self);
        detail.insert("message".to_owned(), self.to_string().into());
        Ok(detail)
    }
}

/// Extension trait to convert any error to PythError.
pub trait IntoPythError<T> {
    fn into_pyth_err(self) -> Result<T, PythError>;
}

impl<T, E: std::fmt::Display> IntoPythError<T> for Result<T, E> {
    fn into_pyth_err(self) -> Result<T, PythError> {
        self.map_err(|e| PythError::Any(anyhow::anyhow!("{}", e)))
    }
}
