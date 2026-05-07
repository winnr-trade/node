//! Error types for the orderbook module.

use shared_types::{MarketId, OrderId, Size};
use sov_bank::Amount;
use sov_modules_api::{err_detail, ErrorContext, ErrorDetail, StateValueError};
use thiserror::Error;

/// Errors that can occur in the orderbook module.
#[derive(Debug, Error, serde::Serialize)]
#[serde(tag = "error_code", rename_all = "snake_case")]
pub enum OrderbookError {
    #[error("Order {order_id} not found")]
    OrderNotFound { order_id: OrderId },

    #[error("Market {market_id} not found")]
    MarketNotFound { market_id: MarketId },

    #[error("Market {market_id} is not active: {status}")]
    MarketNotActive { market_id: MarketId, status: String },

    #[error("Invalid price {price}: must be between 1 and 9999")]
    InvalidPrice { price: u64 },

    #[error("Order size {size} is below minimum {minimum}")]
    OrderTooSmall { size: Size, minimum: Size },

    #[error("Not order owner: owned by {owner}, sender is {sender}")]
    NotOrderOwner {
        order_id: OrderId,
        owner: String,
        sender: String,
    },

    #[error("Order {order_id} cannot be cancelled (status: {status})")]
    OrderNotCancellable { order_id: OrderId, status: String },

    #[error("PostOnly order would match immediately")]
    PostOnlyWouldMatch,

    #[error("FillOrKill: requested {requested}, only {available} available")]
    FillOrKillNotFilled { requested: Size, available: Size },

    #[error("Insufficient shares: required {required}, available {available}")]
    InsufficientShares { required: Size, available: Size },

    #[error("Insufficient collateral: required {required}, available {available}")]
    InsufficientCollateral { required: Amount, available: Amount },

    #[error("Quantity must be greater than zero")]
    ZeroQuantity,

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

/// Extension trait to convert any error to OrderbookError
pub trait IntoOrderbookError<T> {
    fn into_orderbook_err(self) -> Result<T, OrderbookError>;
}

impl<T, E: std::fmt::Display> IntoOrderbookError<T> for Result<T, E> {
    fn into_orderbook_err(self) -> Result<T, OrderbookError> {
        self.map_err(|e| OrderbookError::Any(anyhow::anyhow!("{}", e)))
    }
}

/// Extension trait to flatten nested Result from get_or_err and convert to OrderbookError
pub trait IntoOrderbookErrorFlat<T> {
    fn into_orderbook_err_flat(self) -> Result<T, OrderbookError>;
}

// For Result<Result<T, InnerErr>, OuterErr> - handles get_or_err pattern
impl<T, OuterErr, InnerErr> IntoOrderbookErrorFlat<T> for Result<Result<T, InnerErr>, OuterErr>
where
    OuterErr: std::fmt::Display,
    InnerErr: std::fmt::Display,
{
    fn into_orderbook_err_flat(self) -> Result<T, OrderbookError> {
        self.map_err(|e| OrderbookError::Any(anyhow::anyhow!("{}", e)))?
            .map_err(|e| OrderbookError::Any(anyhow::anyhow!("{}", e)))
    }
}

// For Result<Option<T>, Err> - handles get pattern
impl<T, E: std::fmt::Display> IntoOrderbookErrorFlat<T> for Result<Option<T>, E> {
    fn into_orderbook_err_flat(self) -> Result<T, OrderbookError> {
        self.map_err(|e| OrderbookError::Any(anyhow::anyhow!("{}", e)))?
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Value not found in state")))
    }
}

// Implement From for StateValueError (handles get_or_err inner error)
impl<U: sov_modules_api::CompileTimeNamespace> From<StateValueError<U>> for OrderbookError {
    fn from(err: StateValueError<U>) -> Self {
        OrderbookError::Any(anyhow::anyhow!("{}", err))
    }
}

impl ErrorDetail for OrderbookError {
    fn error_detail(&self) -> Result<ErrorContext, Box<dyn std::error::Error + Send + Sync>> {
        let mut detail = err_detail!(self);
        detail.insert("message".to_owned(), self.to_string().into());
        Ok(detail)
    }
}
