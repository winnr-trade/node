use sov_modules_api::{err_detail, ErrorContext, ErrorDetail};

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "error_code", rename_all = "snake_case")]
pub enum AgentWalletError {
    #[error("Agent {agent} is not delegated by any owner")]
    AgentNotDelegated { agent: String },

    #[error("Policy not found for agent {agent}")]
    PolicyNotFound { agent: String },

    #[error("Policy for agent {agent} expired at {expired_at}")]
    PolicyExpired { agent: String, expired_at: u64 },

    #[error("Scope not granted: required {required:#010x}, granted {granted:#010x}")]
    ScopeNotGranted { required: u32, granted: u32 },

    #[error("Invalid scopes: must be non-zero and only contain known bits")]
    InvalidScopes,

    #[error("Invalid expiry: expires_at must be 0 (no expiry) or a future timestamp")]
    InvalidExpiry,

    #[error("Invalid owner signature")]
    InvalidSignature,

    #[error("Invalid owner public key bytes")]
    InvalidPublicKey,

    #[error("Invalid signature bytes")]
    InvalidSignatureBytes,

    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("Nonce overflow for owner")]
    NonceOverflow,

    #[error("Agent address must differ from owner address")]
    AgentCannotBeOwner,

    #[error("Unauthorized: {action}")]
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

impl ErrorDetail for AgentWalletError {
    fn error_detail(&self) -> Result<ErrorContext, Box<dyn std::error::Error + Send + Sync>> {
        let mut detail = err_detail!(self);
        detail.insert("message".to_owned(), self.to_string().into());
        Ok(detail)
    }
}

/// Convenience trait: convert any `Result<T, E: Display>` into `Result<T, AgentWalletError>`.
pub trait IntoAgentWalletError<T> {
    fn into_agent_wallet_err(self) -> Result<T, AgentWalletError>;
}

impl<T, E: std::fmt::Display> IntoAgentWalletError<T> for Result<T, E> {
    fn into_agent_wallet_err(self) -> Result<T, AgentWalletError> {
        self.map_err(|e| AgentWalletError::Any(anyhow::anyhow!("{}", e)))
    }
}
