//! Key types for StateMap storage in the orderbook module.
//!
//! These wrapper types implement Display and FromStr as required by StateMap.

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use shared_types::{MarketId, OutcomeSide, Price, Side};
use sov_modules_api::Spec;
use std::str::FromStr;

/// Key for price level in the order book: (MarketId, OutcomeSide, Price)
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct PriceLevelKey {
    pub market_id: MarketId,
    pub outcome: OutcomeSide,
    pub price: Price,
}

impl core::fmt::Display for PriceLevelKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let outcome = match self.outcome {
            OutcomeSide::Yes => "Y",
            OutcomeSide::No => "N",
        };
        write!(f, "{}:{}:{}", self.market_id.0, outcome, self.price.0)
    }
}

impl FromStr for PriceLevelKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            anyhow::bail!("Invalid PriceLevelKey format");
        }
        let market_id = MarketId(u64::from_str(parts[0])?);
        let outcome = match parts[1] {
            "Y" => OutcomeSide::Yes,
            "N" => OutcomeSide::No,
            _ => anyhow::bail!("Invalid outcome side"),
        };
        let price = Price(u64::from_str(parts[2])?);
        Ok(PriceLevelKey {
            market_id,
            outcome,
            price,
        })
    }
}

/// Key for order book: (MarketId, OutcomeSide)
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct BookKey {
    pub market_id: MarketId,
    pub outcome: OutcomeSide,
}

impl core::fmt::Display for BookKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let outcome = match self.outcome {
            OutcomeSide::Yes => "Y",
            OutcomeSide::No => "N",
        };
        write!(f, "{}:{}", self.market_id.0, outcome)
    }
}

impl FromStr for BookKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid BookKey format");
        }
        let market_id = MarketId(u64::from_str(parts[0])?);
        let outcome = match parts[1] {
            "Y" => OutcomeSide::Yes,
            "N" => OutcomeSide::No,
            _ => anyhow::bail!("Invalid outcome side"),
        };
        Ok(BookKey { market_id, outcome })
    }
}

/// Key for order book side: (MarketId, OutcomeSide, Side)
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize,
)]
pub struct BookSideKey {
    pub market_id: MarketId,
    pub outcome: OutcomeSide,
    pub side: Side,
}

impl core::fmt::Display for BookSideKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let outcome = match self.outcome {
            OutcomeSide::Yes => "Y",
            OutcomeSide::No => "N",
        };
        let side = match self.side {
            Side::Bid => "B",
            Side::Ask => "A",
        };
        write!(f, "{}:{}:{}", self.market_id.0, outcome, side)
    }
}

impl FromStr for BookSideKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() != 3 {
            anyhow::bail!("Invalid BookSideKey format");
        }
        let market_id = MarketId(u64::from_str(parts[0])?);
        let outcome = match parts[1] {
            "Y" => OutcomeSide::Yes,
            "N" => OutcomeSide::No,
            _ => anyhow::bail!("Invalid outcome side"),
        };
        let side = match parts[2] {
            "B" => Side::Bid,
            "A" => Side::Ask,
            _ => anyhow::bail!("Invalid side"),
        };
        Ok(BookSideKey {
            market_id,
            outcome,
            side,
        })
    }
}

/// Key for user position per market: (Address, MarketId)
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
