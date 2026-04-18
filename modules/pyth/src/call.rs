use crate::error::IntoPythError;
use crate::types::{GuardianSet, PriceUpdate};
use crate::{Event, PriceFeedKey, PythError, PythModule};
use schemars::JsonSchema;
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};

/// Call messages for the Pyth module.
#[serialize(Borsh, Serde)]
#[derive(Debug, Clone, PartialEq, Eq, JsonSchema, UniversalWallet)]
#[schemars(rename = "PythCallMessage")]
#[serde(rename_all = "snake_case")]
pub enum CallMessage {
    /// Submit one or more verified price updates.
    /// Permissionless — anyone can call this with valid price data.
    UpdatePriceFeeds {
        /// Each entry is a price update to store.
        updates: Vec<PriceUpdate>,
    },

    /// Update the Wormhole guardian set (admin only).
    SetGuardianSet {
        /// Guardian public key addresses (20 bytes each).
        keys: Vec<[u8; 20]>,
        /// Expiry timestamp (0 = never expires).
        expiry: u64,
    },
}

impl<S: Spec> PythModule<S> {
    pub(crate) fn update_price_feeds(
        &mut self,
        updates: Vec<PriceUpdate>,
        state: &mut impl TxState<S>,
    ) -> Result<(), PythError> {
        for update in updates {
            let key = PriceFeedKey {
                feed_id: update.feed_id.clone(),
                publish_time: update.publish_time,
            };

            self.price_updates
                .set(&key, &update, state)
                .into_pyth_err()?;

            self.emit_event(
                state,
                Event::PriceUpdated {
                    feed_id: format!("{}", update.feed_id),
                    price: update.price,
                    conf: update.conf,
                    expo: update.expo,
                    publish_time: update.publish_time,
                },
            );
        }

        Ok(())
    }

    pub(crate) fn set_guardian_set(
        &mut self,
        keys: Vec<[u8; 20]>,
        expiry: u64,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), PythError> {
        let admin = self
            .admin
            .get(state)
            .into_pyth_err()?
            .ok_or_else(|| PythError::Any(anyhow::anyhow!("Admin not set")))?;

        if *ctx.sender() != admin {
            return Err(PythError::Unauthorized {
                action: "set guardian set".to_string(),
            });
        }

        let guardian_set = GuardianSet {
            keys: keys.clone(),
            expiry,
        };
        self.guardian_set
            .set(&guardian_set, state)
            .into_pyth_err()?;

        self.emit_event(
            state,
            Event::GuardianSetUpdated {
                num_keys: keys.len(),
                expiry,
            },
        );

        Ok(())
    }
}
