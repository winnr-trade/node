//! Call messages for the prediction market module.

use schemars::JsonSchema;
use shared_types::{MarketId, Outcome};
use sov_bank::TokenId;
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::{SafeString, Spec};

/// Call messages for the prediction market module.
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[schemars(bound = "S: Spec", rename = "MarketCallMessage")]
#[serialize(Borsh, Serde)]
#[serde(rename_all = "snake_case")]
pub enum CallMessage<S: Spec> {
    /// Create a new prediction market.
    CreateMarket {
        /// The question bytes being predicted.
        question: SafeString,
        /// Token used as collateral.
        collateral_token: TokenId,
        /// Slot after which resolution is allowed.
        resolution_time: u64,
        /// Address authorized to resolve this market.
        resolver: S::Address,
    },

    /// Mint YES and NO shares by depositing collateral.
    MintShares {
        /// Market to mint shares for.
        market_id: MarketId,
        /// Amount of collateral to deposit (mints equal YES and NO).
        amount: u64,
    },

    /// Redeem pairs of YES and NO shares for collateral.
    RedeemShares {
        /// Market to redeem from.
        market_id: MarketId,
        /// Number of share pairs to redeem.
        amount: u64,
    },

    /// Resolve a market with a final outcome.
    ResolveMarket {
        /// Market to resolve.
        market_id: MarketId,
        /// Final outcome.
        outcome: Outcome,
    },

    /// Claim winnings from a resolved market.
    ClaimWinnings {
        /// Market to claim from.
        market_id: MarketId,
    },

    /// Set supported collateral token (admin only).
    SetSupportedCollateralToken {
        /// Token to set support for.
        token_id: TokenId,
        /// Whether the token is supported.
        support: bool,
    },

    /// Halt trading on a market (admin/resolver only).
    HaltMarket {
        /// Market to halt.
        market_id: MarketId,
    },

    /// Resume trading on a halted market (admin/resolver only).
    ResumeMarket {
        /// Market to resume.
        market_id: MarketId,
    },
}
