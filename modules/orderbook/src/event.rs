//! Events emitted by the orderbook module.
//!
//! All prices are in canonical YES-space.

use crate::SettlementKind;
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::{MarketId, OrderId, OrderType, OutcomeSide, Price, Side, Size};

/// Events emitted by the orderbook module.
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
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Order was placed (includes both original intent and canonical form).
    OrderPlaced {
        order_id: OrderId,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        canonical_side: Side,
        canonical_price: Price,
        quantity: Size,
        order_type: OrderType,
        owner: String,
    },

    /// Order was filled (partially or fully).
    OrderFilled {
        order_id: OrderId,
        filled_quantity: Size,
        remaining_quantity: Size,
        average_price: u64,
    },

    /// Order was cancelled.
    OrderCancelled {
        order_id: OrderId,
        reason: CancelReason,
        unfilled_quantity: Size,
    },

    /// Trade executed between counterparties.
    Trade {
        market_id: MarketId,
        maker_order_id: OrderId,
        taker_order_id: OrderId,
        /// Canonical YES-space execution price.
        price: Price,
        quantity: Size,
        /// Canonical buyer (bid-side party).
        buyer: String,
        /// Canonical seller (ask-side party).
        seller: String,
        /// How this trade was settled.
        settlement_kind: SettlementKind,
        /// Timestamp of trade execution
        timestamp: u64,
    },

    /// Best bid/ask updated (canonical YES-space).
    BookUpdated {
        market_id: MarketId,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
        /// Block timestamp in milliseconds when the book state changed.
        timestamp: u64,
    },
}

/// Reason for order cancellation.
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
#[serde(rename_all = "snake_case")]
pub enum CancelReason {
    /// User requested cancellation.
    UserRequested,
    /// Order expired.
    Expired,
    /// Market was closed/halted.
    MarketClosed,
    /// PostOnly order would have matched.
    PostOnlyWouldMatch,
    /// FillOrKill couldn't be fully filled.
    FillOrKillNotFilled,
    /// Self-trade prevention.
    SelfTradePrevention,
}
