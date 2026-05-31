//! Events emitted by the orderbook module.
//!
//! All prices are in canonical YES-space.

use crate::SettlementKind;
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::{MarketId, OrderId, OrderType, OutcomeSide, Price, Side, Size};
use sov_modules_api::HexHash;

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
        timestamp: u64,
    },

    /// Order was filled (partially or fully).
    OrderFilled {
        order_id: OrderId,
        filled_quantity: Size,
        remaining_quantity: Size,
        average_price: u64,
        timestamp: u64,
    },

    /// Order was cancelled.
    OrderCancelled {
        order_id: OrderId,
        reason: CancelReason,
        unfilled_quantity: Size,
        timestamp: u64,
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

    /// Emitted when a stealth order is placed, linking the stealth address to
    /// its detection tag for off-chain position discovery.
    ///
    /// The `commitment` field links this event to the shielded pool's `Note`
    /// event for the same transaction. Indexing on `detection_tag` lets the
    /// owner find all their stealth addresses on a given market by scanning
    /// per-market nonces and querying hash(view_key, market_id, nonce).
    StealthOrderMemo {
        /// Shielded-pool commitment from the collateral withdrawal —
        /// correlates with `ShieldedPoolEvent::Note.commitment`.
        commitment: HexHash,
        /// The stealth address that owns the placed order.
        stealth_address: String,
        /// hash(view_key, market_id, nonce) — per-market nonce allows
        /// efficient scanning without brute-forcing across all markets.
        detection_tag: HexHash,
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
