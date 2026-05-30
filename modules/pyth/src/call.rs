use crate::error::IntoPythError;
use crate::types::{GuardianSet, PriceUpdate};
use crate::{Event, PriceFeedKey, PythError, PythModule};
use pythnet_sdk::messages::Message;
use pythnet_sdk::wire::from_slice;
use pythnet_sdk::wire::v1::{AccumulatorUpdateData, Proof};
use schemars::JsonSchema;
use sov_modules_api::macros::{serialize, UniversalWallet};
use sov_modules_api::{Context, EventEmitter, HexHash, SafeVec, Spec, TxState};

pub const MAX_BYTES_PRICE_UPDATES: usize = 5244;

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
        // updates: Vec<PriceUpdate>,
        update_data: SafeVec<u8, MAX_BYTES_PRICE_UPDATES>,
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
        update_data: SafeVec<u8, MAX_BYTES_PRICE_UPDATES>,
        state: &mut impl TxState<S>,
    ) -> Result<(), PythError> {
        let res = AccumulatorUpdateData::try_from_slice(&update_data)
            .ok()
            .unwrap();

        // @todo - verify the VAA signatures using the guardian set
        let Proof::WormholeMerkle { vaa: _, updates } = res.proof;
        let timestamp = self.current_time_ms(state)?;

        for update in updates {
            let msg: Message =
                from_slice::<byteorder::BigEndian, _>(update.message.as_ref()).unwrap();

            let price_feed_msg = if let Message::PriceFeedMessage(msg) = msg {
                msg
            } else {
                return Err(PythError::InvalidUpdateData {
                    reason: "Unsupported message type".to_string(),
                });
            };

            let feed_id = HexHash::from(price_feed_msg.feed_id);
            let publish_time = price_feed_msg.publish_time as u64;

            let key = PriceFeedKey {
                feed_id,
                publish_time,
            };

            let price_update = PriceUpdate {
                feed_id,
                price: price_feed_msg.price,
                conf: price_feed_msg.conf,
                expo: price_feed_msg.exponent,
                publish_time,
            };

            self.price_updates
                .set(&key, &price_update, state)
                .into_pyth_err()?;

            self.emit_event(
                state,
                Event::PriceUpdated {
                    feed_id: format!("{}", price_update.feed_id),
                    price: price_update.price,
                    conf: price_update.conf,
                    expo: price_update.expo,
                    publish_time: price_update.publish_time,
                    timestamp,
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

        let timestamp = self.current_time_ms(state)?;
        self.emit_event(
            state,
            Event::GuardianSetUpdated {
                num_keys: keys.len(),
                expiry,
                timestamp,
            },
        );

        Ok(())
    }
}
