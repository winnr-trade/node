//! Call messages for the orderbook module.

use schemars::JsonSchema;
use shared_types::{MarketId, OrderId, OrderType, OutcomeSide, Price, Side, Size};
use shielded_pool::ProofBytes;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    HexHash, SafeVec, Spec,
};

/// Maximum byte length of a stealth-order note memo.
pub(crate) const MAX_MEMO_BYTES: usize = 128;

/// Call messages for the orderbook module.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[schemars(bound = "S: Spec", rename = "OrderbookCall")]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum CallMessage<S: Spec> {
    /// Place a new order.
    PlaceOrderNormal {
        /// Which prediction market.
        market_id: MarketId,
        /// YES or NO outcome.
        outcome: OutcomeSide,
        /// Buy or Sell.
        side: Side,
        /// Limit price (ignored for Market orders).
        price: Price,
        /// Quantity of shares.
        quantity: Size,
        /// Order type.
        order_type: OrderType,
    },

    /// Place a new order.
    PlaceOrderStealth {
        /// Zero-knowledge proof of ownership of the stealth address and validity of the withdrawal.
        proof: ProofBytes,
        /// Merkle root of the shielded pool at the time of order placement.
        root: HexHash,
        /// Commitment corresponding to the proof.
        commitment: HexHash,
        /// Nullifier to prevent double-spending.
        nullifier: HexHash,
        /// Stealth address
        stealth_address: S::Address,
        /// Which prediction market.
        market_id: MarketId,
        /// YES or NO outcome.
        outcome: OutcomeSide,
        /// Buy or Sell.
        side: Side,
        /// Limit price (ignored for Market orders).
        price: Price,
        /// Quantity of shares.
        quantity: Size,
        /// Order type.
        order_type: OrderType,
        /// Forwarded verbatim to `withdraw_to` and surfaced in the shielded
        /// pool's `Note` event.
        note_memo: SafeVec<u8, MAX_MEMO_BYTES>,
        /// Computed as hash(view_key, market_id, nonce) where nonce is
        /// per-market. Indexed in `StealthOrderMemo` to allow the owner to
        /// discover their positions on a given market by scanning nonces.
        detection_tag: HexHash,
    },

    /// Cancel an existing order.
    CancelOrder {
        /// Order to cancel.
        order_id: OrderId,
    },

    /// Cancel all orders for a market.
    CancelAllOrders {
        /// Market to cancel orders for.
        market_id: MarketId,
        /// Optionally filter by outcome.
        outcome: Option<OutcomeSide>,
    },
}
