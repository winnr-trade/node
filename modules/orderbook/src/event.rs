//! Events emitted by the orderbook module.

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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
    /// Order was placed.
    OrderPlaced {
        order_id: OrderId,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
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

    /// Trade executed.
    Trade {
        market_id: MarketId,
        outcome: OutcomeSide,
        maker_order_id: OrderId,
        taker_order_id: OrderId,
        price: Price,
        quantity: u64,
        maker: String,
        taker: String,
    },

    /// Best bid/ask updated.
    BookUpdated {
        market_id: MarketId,
        outcome: OutcomeSide,
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
