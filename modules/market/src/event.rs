//! Events emitted by the prediction market module.

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::{MarketId, MarketStatus, Outcome};
use sov_modules_api::SafeString;

/// Events emitted by the prediction market module.
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
    /// A new market was created.
    MarketCreated {
        market_id: MarketId,
        question: SafeString,
        creator: String,
        collateral_token: String,
        resolution_time: u64,
        resolver: String,
    },

    /// Market status was changed.
    MarketStatusChanged {
        market_id: MarketId,
        old_status: MarketStatus,
        new_status: MarketStatus,
    },

    /// Shares were minted.
    /// `amount` collateral was deposited; the same quantity of YES and NO shares was issued.
    SharesMinted {
        market_id: MarketId,
        user: String,
        amount: u64,
    },

    /// Shares were redeemed.
    /// `amount` YES+NO share pairs were burned; the same quantity of collateral was returned.
    SharesRedeemed {
        market_id: MarketId,
        user: String,
        amount: u64,
    },

    /// Market was resolved.
    MarketResolved {
        market_id: MarketId,
        outcome: Outcome,
        resolver: String,
    },

    /// Winnings were claimed.
    WinningsClaimed {
        market_id: MarketId,
        user: String,
        winning_shares: u64,
        payout: u64,
    },
}
