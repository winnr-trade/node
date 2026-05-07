//! Events emitted by the prediction market module.

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::{MarketId, MarketStatus, Outcome, Size};
use sov_bank::{Amount, TokenId};
use sov_modules_api::SafeString;

/// Reason a position changed — for audit purposes.
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
pub enum PositionUpdateSource {
    /// Shares moved via the orderbook (any settlement kind).
    Trade,
    /// User minted a YES+NO pair directly.
    Mint,
    /// User burned a YES+NO pair directly.
    Burn,
    /// User claimed winnings from a resolved market.
    Claim,
}

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
        collateral_token: TokenId,
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
        amount: Size,
    },

    /// Shares were burned.
    /// `amount` YES+NO share pairs were burned; the same quantity of collateral was returned.
    SharesBurned {
        market_id: MarketId,
        user: String,
        amount: Size,
    },

    /// Shares were transferred between holders.
    SharesTransferred {
        market_id: MarketId,
        from: String,
        to: String,
        yes_amount: Size,
        no_amount: Size,
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
        winning_shares: Size,
        payout: Amount,
    },

    /// A user's position was updated.
    ///
    /// Emitted for every share acquisition or disposal on a user address.
    /// `yes_delta` / `no_delta` are signed: positive = acquired, negative = disposed.
    /// `cost_yes_added` / `cost_no_added` carry the collateral spent on acquisitions only;
    /// they are zero for disposals — the indexer applies a proportional cost reduction using
    /// its own tracked state.
    PositionUpdated {
        market_id: MarketId,
        user_address: String,
        yes_delta: i64,
        no_delta: i64,
        /// Collateral cost for YES shares acquired in this event (0 for disposals).
        cost_yes_added: Amount,
        /// Collateral cost for NO shares acquired in this event (0 for disposals).
        cost_no_added: Amount,
        update_source: PositionUpdateSource,
    },
}
