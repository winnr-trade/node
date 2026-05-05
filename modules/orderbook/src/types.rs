//! Types specific to the orderbook module.
//!
//! All orders are stored in canonical YES-space representation.
//! NO orders are transformed: BUY NO @ p → SELL YES @ (1-p), SELL NO @ p → BUY YES @ (1-p).

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::{
    MarketId, OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side, TokenIdExt,
};
use sov_bank::TokenId;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    Spec,
};

// An order request from the user
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
pub struct OrderRequest {
    pub market_id: MarketId,
    pub outcome: OutcomeSide,
    pub side: Side,
    pub price: Price,
    pub quantity: u64,
    pub order_type: OrderType,
}

/// An order in the canonical YES-space book.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Order<S: Spec> {
    /// Unique order ID.
    pub id: OrderId,
    /// Which prediction market.
    pub market_id: MarketId,
    /// Original user intent: YES or NO outcome.
    pub outcome: OutcomeSide,
    /// Original user intent: Bid or Ask.
    pub side: Side,
    /// Canonical side in YES-space (Bid = buying YES, Ask = selling YES).
    pub canonical_side: Side,
    /// Canonical price in YES-space basis points.
    pub canonical_price: Price,
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

    /// Collateral locked for remaining unfilled quantity.
    ///
    /// Only meaningful for BUY orders (`side == Side::Bid`). For SELL orders,
    /// shares are reserved instead of collateral — use `remaining_quantity` directly.
    pub fn locked_collateral(&self, token: &TokenId) -> u64 {
        match self.canonical_side {
            Side::Bid => self.canonical_price.cost(self.remaining_quantity, token),
            Side::Ask => self
                .canonical_price
                .complement()
                .cost(self.remaining_quantity, token),
        }
    }
}

/// A fill (executed trade).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fill {
    /// Maker order ID.
    pub order_id: OrderId,
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
}

/// Locked share balances per user+market.
#[derive(
    Clone,
    Copy,
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
pub struct LockedShares {
    /// YES shares locked for resting SELL YES orders.
    pub yes: u64,
    /// NO shares locked for resting SELL NO orders.
    pub no: u64,
}

/// How a fill is settled between counterparties.
///
/// Settlement depends on the original intents of both parties:
/// - **MintPair**: BUY YES vs BUY NO — mint YES+NO pair, distribute shares.
/// - **TransferYes**: BUY YES vs SELL YES — transfer existing YES shares.
/// - **TransferNo**: BUY NO vs SELL NO — transfer existing NO shares.
/// - **MergePair**: SELL YES vs SELL NO — burn YES+NO pair, release backing collateral.
///
/// Classification is determined by the original `Side` (Bid = BUY, Ask = SELL) of each
/// counterparty, not the canonical side.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SettlementKind {
    /// Both sides are buying (BUY YES + BUY NO): mint new YES+NO pair from collateral.
    MintPair,
    /// Buyer purchases existing YES shares from seller (BUY YES + SELL YES).
    TransferYes,
    /// Buyer purchases existing NO shares from seller (BUY NO + SELL NO).
    TransferNo,
    /// Both sides are selling (SELL YES + SELL NO): burn YES+NO pair, release collateral.
    MergePair,
}

impl SettlementKind {
    /// Classify settlement from the original `Side` of each counterparty.
    ///
    /// - `canonical_buyer_side`: original `Side` of the bid-side (canonical buyer) party.
    /// - `canonical_seller_side`: original `Side` of the ask-side (canonical seller) party.
    ///
    /// Original `Side::Bid` means the party placed a BUY order; `Side::Ask` means SELL.
    pub fn classify(canonical_buyer_side: Side, canonical_seller_side: Side) -> Self {
        match (canonical_buyer_side, canonical_seller_side) {
            // BUY YES vs BUY NO → mint pair
            (Side::Bid, Side::Bid) => Self::MintPair,
            // BUY YES vs SELL YES → transfer YES shares
            (Side::Bid, Side::Ask) => Self::TransferYes,
            // SELL NO vs BUY NO → transfer NO shares
            (Side::Ask, Side::Bid) => Self::TransferNo,
            // SELL NO vs SELL YES → burn pair
            (Side::Ask, Side::Ask) => Self::MergePair,
        }
    }
}
