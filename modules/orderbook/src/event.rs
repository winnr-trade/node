//! Events emitted by the orderbook module.
//!
//! All prices are in canonical YES-space.

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::SettlementKind;
use shared_types::{MarketId, OrderId, OrderType, OutcomeSide, Price, Side};

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
        quantity: u64,
        order_type: OrderType,
        owner: String,
    },

    /// Order was filled (partially or fully).
    OrderFilled {
        order_id: OrderId,
        filled_quantity: u64,
        remaining_quantity: u64,
        average_price: u64,
    },

    /// Order was cancelled.
    OrderCancelled {
        order_id: OrderId,
        reason: CancelReason,
        unfilled_quantity: u64,
    },

    /// Trade executed between counterparties.
    Trade {
        market_id: MarketId,
        maker_order_id: OrderId,
        taker_order_id: OrderId,
        /// Canonical YES-space execution price.
        price: Price,
        quantity: u64,
        /// Canonical buyer (bid-side party).
        buyer: String,
        /// Canonical seller (ask-side party).
        seller: String,
        /// How this trade was settled.
        settlement_kind: SettlementKind,
    },

    /// Best bid/ask updated (canonical YES-space).
    BookUpdated {
        market_id: MarketId,
        best_bid: Option<Price>,
        best_ask: Option<Price>,
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
