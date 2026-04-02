//! Call messages for the orderbook module.

use schemars::JsonSchema;
use shared_types::{MarketId, OrderId, OrderType, OutcomeSide, Price, Side};
use sov_bank::TokenId;
use sov_modules_api::{
    macros::{serialize, UniversalWallet},
    HexHash, Spec,
};

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
        quantity: u64,
        /// Order type.
        order_type: OrderType,
    },

    /// Place a new order.
    PlaceOrderStealth {
        /// Zero-knowledge proof of ownership of the stealth address and validity of the withdrawal.
        proof: Vec<u8>,
        /// Commitment corresponding to the proof.
        commitment: HexHash,
        /// Nullifier to prevent double-spending.
        nullifier: HexHash,
        /// Stealth address
        stealth_address: S::Address,
        /// Which prediction market.
        market_id: MarketId,
        /// Token ID for collateral.
        token_id: TokenId,
        /// YES or NO outcome.
        outcome: OutcomeSide,
        /// Buy or Sell.
        side: Side,
        /// Limit price (ignored for Market orders).
        price: Price,
        /// Quantity of shares.
        quantity: u64,
        /// Order type.
        order_type: OrderType,
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
