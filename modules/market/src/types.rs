//! Types specific to the prediction market module.

use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::MarketId;
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::{SafeString, Spec};
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
        feed_id: [u8; 32],
        /// Inclusive lower bound on the price. `None` = no floor.
        lower_bound: Option<i64>,
        /// Inclusive upper bound on the price. `None` = no ceiling.
        upper_bound: Option<i64>,
    },

    /// Optimistic oracle (UMA-style propose → dispute → finalize).
    /// Resolution mechanics to be implemented in the future.
    Optimistic,
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
    pub collateral_token: sov_bank::TokenId,
    /// Slot after which market can be resolved.
    pub resolution_time: u64,
    /// Current market status.
    pub status: MarketStatus,
    /// Final outcome (set after resolution).
    pub outcome: Option<Outcome>,
    /// How this market gets resolved.
    pub resolver: Resolver<S>,
    /// Total YES shares in circulation.
    pub total_yes_shares: u64,
    /// Total NO shares in circulation.
    pub total_no_shares: u64,
    /// Slot when market was created.
    pub created_at: u64,
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
    pub yes_shares: u64,
    /// NO shares held.
    pub no_shares: u64,
}

impl Position {
    /// Check if position is empty.
    pub fn is_empty(&self) -> bool {
        self.yes_shares == 0 && self.no_shares == 0
    }

    /// Get minimum of YES and NO shares (redeemable pairs).
    pub fn min_shares(&self) -> u64 {
        self.yes_shares.min(self.no_shares)
    }
}

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct PositionKey<S: Spec> {
    pub market_id: MarketId,
    pub address: S::Address,
}

impl<S: Spec> core::fmt::Display for PositionKey<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // Serialize both fields in a parseable format
        write!(f, "{}:{}", self.market_id.0, self.address)
    }
}

impl<S: Spec> FromStr for PositionKey<S> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Parse back from the Display format
        let parts: Vec<&str> = s.split(':').collect();
        let market_id = MarketId(u64::from_str(parts[0])?);
        let address = S::Address::from_str(parts[1]).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        Ok(PositionKey { market_id, address })
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
