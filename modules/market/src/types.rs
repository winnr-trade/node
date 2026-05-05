//! Types specific to the prediction market module.

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::{MarketId, Size};
use sov_bank::utils::TokenHolder;
use sov_bank::TokenId;
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::{HexHash, SafeString, Spec};
use std::str::FromStr;

pub use shared_types::{MarketStatus, Outcome};

// ============================================================================
// RESOLVER TYPES
// ============================================================================

/// Defines how a market gets resolved.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(bound(serialize = "", deserialize = ""))]
#[schemars(bound = "S: Spec", rename = "Resolver")]
pub enum Resolver<S: Spec> {
    /// A designated address that can submit the outcome directly.
    Address(S::Address),

    /// Pyth oracle price feed. Outcome is `Yes` if the price at resolution
    /// time falls within `[lower_bound, upper_bound]`, `No` otherwise.
    /// A `None` bound means unbounded in that direction.
    Pyth {
        /// Pyth price feed identifier (32 bytes).
        feed_id: HexHash,
        /// Inclusive lower bound on the price. `None` = no floor.
        lower_bound: Option<u64>,
        /// Inclusive upper bound on the price. `None` = no ceiling.
        upper_bound: Option<u64>,
    },

    /// Optimistic oracle (Resolution mechanics to be implemented in the future.)
    Optimistic {},
}

impl<S: Spec> core::fmt::Display for Resolver<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match serde_json::to_string(self) {
            Ok(json) => f.write_str(&json),
            Err(_) => write!(f, "{:?}", self),
        }
    }
}

/// A prediction market.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct Market<S: Spec> {
    /// Unique market identifier.
    pub id: MarketId,
    /// The question being predicted.
    pub question: SafeString,
    /// Address of the market creator.
    pub creator: S::Address,
    /// Token used as collateral.
    pub collateral_token: TokenId,
    /// Slot after which market can be resolved.
    pub resolution_time: u64,
    /// Whether trading is halted by admin.
    pub halted: bool,
    /// Final outcome (set after resolution).
    pub outcome: Option<Outcome>,
    /// How this market gets resolved.
    pub resolver: Resolver<S>,
    /// Total outcome shares in circulation (YES == NO always, so one counter suffices).
    pub total_shares: Size,
    /// Slot when market was created.
    pub created_at: u64,
}

impl<S: Spec> Market<S> {
    /// Derive the market status from stored fields.
    pub fn status(&self) -> MarketStatus {
        if self.outcome.is_some() {
            MarketStatus::Resolved
        } else if self.halted {
            MarketStatus::Halted
        } else {
            MarketStatus::Active
        }
    }
}

/// User's position in a market.
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
pub struct Position {
    /// YES shares held.
    pub yes_shares: Size,
    /// NO shares held.
    pub no_shares: Size,
}

impl Position {
    /// Check if position is empty.
    pub fn is_empty(&self) -> bool {
        self.yes_shares.is_zero() && self.no_shares.is_zero()
    }

    /// Get minimum of YES and NO shares (redeemable pairs).
    pub fn min_shares(&self) -> Size {
        self.yes_shares.min(self.no_shares)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
#[serde(bound(serialize = "", deserialize = ""))]
pub struct PositionKey<S: Spec> {
    pub market_id: MarketId,
    pub owner: TokenHolder<S>,
}

impl<S: Spec> core::fmt::Display for PositionKey<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Serialize both fields in a parseable format
        let owner = serde_json::to_string(&self.owner).map_err(|_| core::fmt::Error)?;
        write!(f, "{}:{}", self.market_id.0, owner)
    }
}

impl<S: Spec> FromStr for PositionKey<S> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse back from the Display format
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("invalid PositionKey format"));
        }

        let market_id = MarketId(u64::from_str(parts[0])?);
        let owner = serde_json::from_str(parts[1])?;
        Ok(PositionKey { market_id, owner })
    }
}

/// Global market configuration.
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
pub struct MarketConfig {
    /// Maximum question length in bytes.
    pub max_question_length: usize,
    /// Minimum slots before resolution time.
    pub min_market_duration: u64,
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            max_question_length: 500,
            min_market_duration: 100,
        }
    }
}
