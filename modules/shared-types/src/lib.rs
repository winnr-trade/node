//! Shared types for the prediction market application.
//!
//! This crate contains common types used across multiple modules
//! to avoid circular dependencies.

use schemars::JsonSchema;
use sov_bank::TokenId;
use sov_modules_api::macros::{serialize, UniversalWallet};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};

// ============================================================================
// TOKEN EXTENSIONS
// ============================================================================

/// Extension trait to extract decimals from a TokenId hash.
/// Decimals are stored at byte [31] of the 32-byte hash.
pub trait TokenIdExt {
    fn get_decimals(&self) -> u8;
}

impl TokenIdExt for TokenId {
    fn get_decimals(&self) -> u8 {
        self.as_ref()[31]
    }
}

// ============================================================================
// MARKET TYPES
// ============================================================================

/// Unique identifier for a prediction market.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema, UniversalWallet,
)]
#[serialize(Borsh, Serde)]
pub struct MarketId(pub u64);

impl Display for MarketId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Market({})", self.0)
    }
}

impl From<u64> for MarketId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl FromStr for MarketId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = u64::from_str(s)?;
        Ok(MarketId(id))
    }
}

/// Possible outcomes for a binary prediction market.
#[derive(Clone, Copy, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The predicted event occurred.
    Yes,
    /// The predicted event did not occur.
    No,
    /// The market was deemed invalid (triggers refund).
    Invalid,
}

/// Current status of a market.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum MarketStatus {
    /// Market is open for trading.
    #[default]
    Active,
    /// Trading is temporarily halted.
    Halted,
    /// Market has been resolved with a final outcome.
    Resolved,
}

// ============================================================================
// ORDER TYPES
// ============================================================================

/// Unique identifier for an order.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema, UniversalWallet,
)]
#[serialize(Borsh, Serde)]
pub struct OrderId(pub u64);

impl core::fmt::Display for OrderId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Order({})", self.0)
    }
}

impl From<u64> for OrderId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl FromStr for OrderId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let id = u64::from_str(s)?;
        Ok(OrderId(id))
    }
}

/// Price in basis points (0-10000).
///
/// - `10000` = 1.00 (100% probability, used for settlement)
/// - `6500` = 0.65 (65% probability)
/// - `100` = 0.01 (1% probability)
///
/// Valid trading range is 1-9999 (0.01% to 99.99%).
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema, UniversalWallet,
)]
#[serialize(Borsh, Serde)]
pub struct Price(pub u64);

impl Price {
    /// Minimum valid trading price (0.01%).
    pub const MIN: Price = Price(1);
    /// Maximum valid trading price (99.99%).
    pub const MAX: Price = Price(9999);
    /// Full value price used for settlement (100%).
    pub const ONE: Price = Price(10000);
    /// Price basis (10000 = 100%).
    pub const BASIS: u64 = 10000;

    /// Create a new price, returning an error if out of valid range.
    pub fn new(value: u64) -> Result<Self, anyhow::Error> {
        if Self::is_valid_value(value) {
            Ok(Price(value))
        } else {
            Err(anyhow::anyhow!(
                "Invalid price {}, expected {}..={}",
                value,
                Self::MIN.0,
                Self::MAX.0
            ))
        }
    }

    /// Check if a raw price value is within tradeable range (1-9999).
    pub fn is_valid(&self) -> bool {
        Self::is_valid_value(self.0)
    }

    /// Check if a raw price value is within tradeable range (1-9999).
    pub fn is_valid_value(value: u64) -> bool {
        value >= Self::MIN.0 && value <= Self::MAX.0
    }

    /// Get the complementary price.
    /// If YES = 0.65 (6500), then NO = 0.35 (3500).
    pub fn complement(&self) -> Price {
        Price(Self::BASIS.saturating_sub(self.0))
    }

    /// Calculate cost for a given quantity in collateral base units.
    ///
    /// E.g., 100 shares at price 6500 (0.65) with 6 decimals (USDC) = 65,000,000 base units.
    /// Formula: (quantity * price_bps * 10^decimals) / BASIS
    pub fn cost(&self, quantity: u64, token: &TokenId) -> u64 {
        let scale = 10u128.pow(token.get_decimals() as u32);
        ((self.0 as u128 * quantity as u128 * scale) / Self::BASIS as u128) as u64
    }

    /// Calculate quantity affordable with given collateral in base units.
    pub fn quantity_for_collateral(&self, collateral: u64, token: &TokenId) -> u64 {
        if self.0 == 0 {
            return 0;
        }
        let scale = 10u128.pow(token.get_decimals() as u32);
        ((collateral as u128 * Self::BASIS as u128) / (self.0 as u128 * scale)) as u64
    }
}

impl Display for Price {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}%", self.0 as f64 / 100.0)
    }
}

impl FromStr for Price {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = u64::from_str(s)?;
        Self::new(value)
    }
}

/// Which outcome side an order is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeSide {
    No,
    Yes,
}

impl OutcomeSide {
    pub fn opposite(&self) -> Self {
        match self {
            OutcomeSide::Yes => OutcomeSide::No,
            OutcomeSide::No => OutcomeSide::Yes,
        }
    }
}

/// Order direction (buy or sell).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Buying shares (willing to pay up to limit price).
    Bid,
    /// Selling shares (willing to sell at limit price or better).
    Ask,
}

impl Side {
    pub fn opposite(&self) -> Self {
        match self {
            Side::Bid => Side::Ask,
            Side::Ask => Side::Bid,
        }
    }
}

/// Order type determining execution behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    /// Standard limit order - rests on book if not filled.
    #[default]
    Limit,
    /// Execute at best available price (no limit).
    Market,
    /// Maker only - rejected if would match immediately.
    PostOnly,
    /// Fill what's possible, cancel the rest immediately.
    ImmediateOrCancel,
    /// Must fill entirely or cancel entirely.
    FillOrKill,
}

/// Order status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    /// Order is active on the book.
    #[default]
    Open,
    /// Order was fully filled.
    Filled,
    /// Order was partially filled (may still be open or cancelled).
    PartiallyFilled,
    /// Order was cancelled.
    Cancelled,
    /// Order expired.
    Expired,
    /// Order was rejected.
    Rejected,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_complement() {
        let yes_price = Price(6500); // 65%
        let no_price = yes_price.complement();
        assert_eq!(no_price, Price(3500)); // 35%

        // Complements should sum to 100%
        assert_eq!(yes_price.0 + no_price.0, Price::BASIS);
    }

    #[test]
    fn test_price_cost() {
        let price = Price(6500); // 65%
        let mut token_bytes = [0u8; 32];
        token_bytes[31] = 6; // 6 decimals
        let token = TokenId::from(token_bytes);

        // 100 shares at 0.65 with 6 decimals = 65,000,000 base units
        assert_eq!(price.cost(100, &token), 65_000_000);

        // 1000 shares at 0.65 with 6 decimals = 650,000,000 base units
        assert_eq!(price.cost(1000, &token), 650_000_000);
    }

    #[test]
    fn test_price_is_valid() {
        assert!(Price(1).is_valid());
        assert!(Price(5000).is_valid());
        assert!(Price(9999).is_valid());

        assert!(!Price(0).is_valid());
        assert!(!Price(10000).is_valid());
        assert!(!Price(10001).is_valid());
    }

    #[test]
    fn test_price_is_valid_value() {
        assert!(Price::is_valid_value(1));
        assert!(Price::is_valid_value(5000));
        assert!(Price::is_valid_value(9999));

        assert!(!Price::is_valid_value(0));
        assert!(!Price::is_valid_value(10000));
        assert!(!Price::is_valid_value(10001));
    }

    #[test]
    fn test_price_new_validation() {
        assert_eq!(Price::new(1).unwrap(), Price(1));
        assert!(Price::new(0).is_err());
        assert!(Price::new(10000).is_err());
    }

    #[test]
    fn test_outcome_side_opposite() {
        assert_eq!(OutcomeSide::Yes.opposite(), OutcomeSide::No);
        assert_eq!(OutcomeSide::No.opposite(), OutcomeSide::Yes);
    }

    #[test]
    fn test_side_opposite() {
        assert_eq!(Side::Bid.opposite(), Side::Ask);
        assert_eq!(Side::Ask.opposite(), Side::Bid);
    }
}
