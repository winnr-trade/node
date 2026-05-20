//! Error types for the market module.

use shared_types::{MarketId, MarketStatus, Size};
use sov_modules_api::{err_detail, ErrorContext, ErrorDetail, StateValueError};

#[derive(Debug, thiserror::Error, serde::Serialize)]
#[serde(tag = "error_code", rename_all = "snake_case")]
// #[derive(Debug, thiserror::Error)]
pub enum MarketError {
    #[error("Market {market_id} not found")]
    MarketNotFound { market_id: MarketId },

    #[error("Market {market_id} is not active (status: {status:?})")]
    MarketNotActive {
        market_id: MarketId,
        status: MarketStatus,
    },

    #[error("Market {market_id} is already resolved")]
    MarketAlreadyResolved { market_id: MarketId },

    #[error("Market {market_id} is not yet resolved")]
    MarketNotResolved { market_id: MarketId },

    #[error("Unauthorized resolver for market {market_id}: expected {expected}, got {actual}")]
    UnauthorizedResolver {
        market_id: MarketId,
        expected: String,
        actual: String,
    },

    #[error(
        "Resolution time {resolution_time} is too early, earliest allowed: {earliest_allowed}"
    )]
    ResolutionTimeTooSoon {
        resolution_time: u64,
        earliest_allowed: u64,
    },

    #[error("Resolution time {resolution_time} has not passed for market {market_id}, current: {current_time}")]
    ResolutionTimeTooEarly {
        market_id: MarketId,
        resolution_time: u64,
        current_time: u64,
    },

    #[error("Question too long: {length} bytes, max: {max_length}")]
    QuestionTooLong { length: usize, max_length: usize },

    #[error("Insufficient shares: required {required}, available YES: {available_yes}, NO: {available_no}")]
    InsufficientShares {
        required: Size,
        available_yes: Size,
        available_no: Size,
    },

    #[error("No position found for market {market_id}")]
    NoPosition { market_id: MarketId },

    #[error("No winnings to claim for market {market_id}")]
    NoWinningsToClaim { market_id: MarketId },

    #[error("Amount must be greater than zero")]
    ZeroAmount,

    #[error("Unauthorized to {action}")]
    Unauthorized { action: String },

    #[error("Invalid resolver type for market {market_id}: expected {expected}, got {actual}")]
    InvalidResolverType {
        market_id: MarketId,
        expected: String,
        actual: String,
    },

    #[error("Pyth price feed {feed_id} not found at publish_time {publish_time}")]
    PythFeedNotFound { feed_id: String, publish_time: u64 },

    #[error("Optimistic oracle resolution is not yet implemented")]
    OptimisticResolutionNotImplemented,

    // #[error(transparent)]
    // Any(#[from] anyhow::Error),
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

/// Extension trait to convert any error to PredictionMarketError
pub trait IntoMarketError<T> {
    fn into_market_err(self) -> Result<T, MarketError>;
}

impl<T, E: std::fmt::Display> IntoMarketError<T> for Result<T, E> {
    fn into_market_err(self) -> Result<T, MarketError> {
        self.map_err(|e| MarketError::Any(anyhow::anyhow!("{}", e)))
    }
}

/// Extension trait to flatten nested Result from get_or_err and convert to PredictionMarketError
pub trait IntoMarketErrorFlat<T> {
    fn into_market_err_flat(self) -> Result<T, MarketError>;
}

// For Result<Result<T, InnerErr>, OuterErr> - handles get_or_err pattern
impl<T, OuterErr, InnerErr> IntoMarketErrorFlat<T> for Result<Result<T, InnerErr>, OuterErr>
where
    OuterErr: std::fmt::Display,
    InnerErr: std::fmt::Display,
{
    fn into_market_err_flat(self) -> Result<T, MarketError> {
        self.map_err(|e| MarketError::Any(anyhow::anyhow!("{}", e)))?
            .map_err(|e| MarketError::Any(anyhow::anyhow!("{}", e)))
    }
}

// For Result<Option<T>, Err> - handles get pattern
impl<T, E: std::fmt::Display> IntoMarketErrorFlat<T> for Result<Option<T>, E> {
    fn into_market_err_flat(self) -> Result<T, MarketError> {
        self.map_err(|e| MarketError::Any(anyhow::anyhow!("{}", e)))?
            .ok_or_else(|| MarketError::Any(anyhow::anyhow!("Value not found in state")))
    }
}

// Implement From for StateValueError (handles get_or_err inner error)
impl<U: sov_modules_api::CompileTimeNamespace> From<StateValueError<U>> for MarketError {
    fn from(err: StateValueError<U>) -> Self {
        MarketError::Any(anyhow::anyhow!("{}", err))
    }
}

impl ErrorDetail for MarketError {
    fn error_detail(&self) -> Result<ErrorContext, Box<dyn std::error::Error + Send + Sync>> {
        let mut detail = err_detail!(self); // Serializes `PredictionMarketError` to a JSON object
        detail.insert("message".to_owned(), self.to_string().into());
        Ok(detail)
    }
}
