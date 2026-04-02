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
    SharesMinted {
        market_id: MarketId,
        user: String,
        collateral_amount: u64,
        yes_shares: u64,
        no_shares: u64,
    },

    /// Shares were redeemed.
    SharesRedeemed {
        market_id: MarketId,
        user: String,
        yes_shares_burned: u64,
        no_shares_burned: u64,
        collateral_returned: u64,
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
