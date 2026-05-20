//! Call messages for the prediction market module.

use crate::types::Resolver;
use schemars::JsonSchema;
use shared_types::{MarketId, Outcome, Size};
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::{SafeString, Spec};

/// Data required to resolve a market, depending on the resolver type.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionData {
    /// Resolve by designated address: caller provides the outcome directly.
    Address { outcome: Outcome },
    /// Resolve via Pyth oracle: caller provides the publish_time of a stored price update.
    Pyth { publish_time: u64 },
}

/// Call messages for the prediction market module.
#[serialize(Borsh, Serde)]
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[schemars(bound = "S: Spec", rename = "MarketCallMessage")]
#[serde(rename_all = "snake_case", bound(serialize = "", deserialize = ""))]
pub enum CallMessage<S: Spec> {
    /// Create a new prediction market.
    CreateMarket {
        /// The question bytes being predicted.
        question: SafeString,
        /// Slot after which resolution is allowed.
        resolution_time: u64,
        /// How this market should be resolved.
        resolver: Resolver<S>,
    },

    /// Mint YES and NO shares by depositing collateral.
    MintShares {
        /// Market to mint shares for.
        market_id: MarketId,
        /// Amount of collateral to deposit (mints equal YES and NO).
        amount: Size,
    },

    /// Burn pairs of YES and NO shares for collateral.
    BurnShares {
        /// Market to burn from.
        market_id: MarketId,
        /// Number of share pairs to burn.
        amount: Size,
    },

    /// Resolve a market with a final outcome.
    ResolveMarket {
        /// Market to resolve.
        market_id: MarketId,
        /// Resolution data matching the market's resolver type.
        data: ResolutionData,
    },

    /// Claim winnings from a resolved market.
    ClaimWinnings {
        /// Market to claim from.
        market_id: MarketId,
    },

    /// Compact active-market index for the sender.
    CompactUserActiveMarkets {
        /// Maximum entries to scan in this call.
        max_scan: u32,
    },

    /// Halt trading on a market (admin only).
    HaltMarket {
        /// Market to halt.
        market_id: MarketId,
    },

    /// Resume trading on a halted market (admin only).
    ResumeMarket {
        /// Market to resume.
        market_id: MarketId,
    },
}
