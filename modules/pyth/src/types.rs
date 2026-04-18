use schemars::JsonSchema;
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::HexHash;
use std::str::FromStr;

/// Composite key for looking up a price update by feed and timestamp.
#[derive(Clone, Debug, PartialEq, Eq, Hash, JsonSchema)]
#[serialize(Borsh, Serde)]
pub struct PriceFeedKey {
    /// Pyth price feed identifier (32 bytes).
    pub feed_id: HexHash,
    /// Timestamp at which the price was published.
    pub publish_time: u64,
}

impl core::fmt::Display for PriceFeedKey {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}:{}", self.feed_id, self.publish_time)
    }
}

impl FromStr for PriceFeedKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid PriceFeedKey format");
        }
        let feed_id = HexHash::from_str(parts[0]).map_err(|e| anyhow::anyhow!("{:?}", e))?;
        let publish_time = u64::from_str(parts[1])?;
        Ok(PriceFeedKey {
            feed_id,
            publish_time,
        })
    }
}

/// A verified Pyth price update.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[serialize(Borsh, Serde)]
pub struct PriceUpdate {
    /// Pyth price feed identifier (32 bytes).
    pub feed_id: HexHash,
    /// Price value.
    pub price: i64,
    /// Confidence interval.
    pub conf: u64,
    /// Price exponent (e.g. -8 means price is in units of 10^-8).
    pub expo: i32,
    /// Timestamp when the price was published.
    pub publish_time: u64,
}

/// Wormhole guardian set for VAA signature verification.
#[derive(Clone, Debug, PartialEq, Eq, JsonSchema)]
#[serialize(Borsh, Serde)]
pub struct GuardianSet {
    /// Guardian public key addresses (20 bytes each, Ethereum-style).
    pub keys: Vec<[u8; 20]>,
    /// Expiry timestamp (0 = never expires).
    pub expiry: u64,
}
