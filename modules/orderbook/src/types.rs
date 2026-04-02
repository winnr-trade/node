//! Types specific to the orderbook module.

use borsh::{BorshDeserialize, BorshSerialize};
use shared_types::{MarketId, OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sov_modules_api::Spec;

/// An order in the book.
#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct Order<S: Spec> {
    /// Unique order ID.
    pub id: OrderId,
    /// Which prediction market.
    pub market_id: MarketId,
    /// YES or NO outcome.
    pub outcome: OutcomeSide,
    /// Bid or Ask.
    pub side: Side,
    /// Limit price in basis points.
    pub price: Price,
    /// Original quantity.
    pub original_quantity: u64,
    /// Remaining unfilled quantity.
    pub remaining_quantity: u64,
    /// Order owner.
    pub owner: S::Address,
    /// Order type.
    pub order_type: OrderType,
    /// Slot when order was placed.
    pub created_at: u64,
    /// Current status.
    pub status: OrderStatus,
}

impl<S: Spec> Order<S> {
    /// Check if order is fully filled.
    pub fn is_filled(&self) -> bool {
        self.remaining_quantity == 0
    }

    /// Get filled quantity.
    pub fn filled_quantity(&self) -> u64 {
        self.original_quantity - self.remaining_quantity
    }

    /// Calculate collateral required for remaining quantity.
    pub fn required_collateral(&self) -> u64 {
        self.price.cost(self.remaining_quantity)
    }
}

/// A fill (executed trade).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fill {
    /// Maker order ID.
    pub maker_order_id: OrderId,
    /// Taker order ID.
    pub taker_order_id: OrderId,
    /// Execution price.
    pub price: Price,
    /// Quantity filled.
    pub quantity: u64,
    /// Maker fee.
    pub maker_fee: u64,
    /// Taker fee.
    pub taker_fee: u64,
}

/// Fee configuration.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
)]
pub struct FeeConfig {
    /// Maker fee in basis points (e.g., 10 = 0.1%).
    pub maker_fee_bps: u16,
    /// Taker fee in basis points (e.g., 30 = 0.3%).
    pub taker_fee_bps: u16,
    /// Minimum order size.
    pub min_order_size: u64,
    /// Maximum open orders per user per market.
    pub max_orders_per_user: u32,
}

/// Result of matching an incoming order.
#[derive(Debug, Default)]
pub struct MatchResult {
    /// Fills that occurred.
    pub fills: Vec<Fill>,
    /// Total quantity filled.
    pub total_filled: u64,
    /// Remaining unfilled quantity.
    pub remaining: u64,
    /// Whether remaining quantity should rest on book.
    pub should_post: bool,
}
