//! Key types for StateMap storage in the orderbook module.
//!
//! All keys are in canonical YES-space. There is one unified order book per market.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use shared_types::{MarketId, Price, Side};
use sov_modules_api::Spec;
use std::str::FromStr;

/// Key for a price level in the canonical book: (MarketId, Price).
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct PriceLevelKey {
    pub market_id: MarketId,
    pub price: Price,
}

impl core::fmt::Display for PriceLevelKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.market_id.0, self.price.0)
    }
}

impl FromStr for PriceLevelKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid PriceLevelKey format");
        }
        let market_id = MarketId(u64::from_str(parts[0])?);
        let price = Price(u64::from_str(parts[1])?);
        Ok(PriceLevelKey { market_id, price })
    }
}

/// Key for book-level lookups per market side: (MarketId, Side).
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct MarketSideKey {
    pub market_id: MarketId,
    pub side: Side,
}

impl core::fmt::Display for MarketSideKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let side = match self.side {
            Side::Bid => "B",
            Side::Ask => "A",
        };
        write!(f, "{}:{}", self.market_id.0, side)
    }
}

impl FromStr for MarketSideKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid MarketSideKey format");
        }
        let market_id = MarketId(u64::from_str(parts[0])?);
        let side = match parts[1] {
            "B" => Side::Bid,
            "A" => Side::Ask,
            _ => anyhow::bail!("Invalid side"),
        };
        Ok(MarketSideKey { market_id, side })
    }
}

/// Key for user state per market: (Address, MarketId).
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct UserMarketKey<S: Spec> {
    pub address: S::Address,
    pub market_id: MarketId,
}

impl<S: Spec> core::fmt::Display for UserMarketKey<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.address, self.market_id.0)
    }
}

impl<S: Spec> FromStr for UserMarketKey<S> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid UserMarketKey format");
        }
        let address = S::Address::from_str(parts[0]).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let market_id = MarketId(u64::from_str(parts[1])?);
        Ok(UserMarketKey { address, market_id })
    }
}
