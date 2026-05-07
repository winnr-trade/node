use crate::types::{PriceFeedKey, PriceUpdate};
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{
    axum::{routing::get, Json, Router},
    UnwrapInfallible,
};
use sov_modules_api::rest::utils::{errors, ApiResult, Path};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, HexHash, Spec};

use crate::PythModule;

impl<S: Spec> PythModule<S> {
    async fn route_status() -> ApiResult<String> {
        Ok(Json("OK".to_string()))
    }

    async fn route_price_at(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        Path((feed_id, timestamp)): Path<(HexHash, u64)>,
    ) -> ApiResult<PriceUpdate> {
        let key = PriceFeedKey {
            feed_id: feed_id.clone(),
            publish_time: timestamp,
        };

        let price = state
            .price_updates
            .get(&key, &mut acc)
            .unwrap_infallible()
            .ok_or_else(|| {
                errors::not_found_404("PriceUpdate", format!("{}@{}", feed_id, timestamp))
            })?;

        Ok(price.into())
    }
}

impl<S: Spec> HasCustomRestApi for PythModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> Router<()> {
        Router::new()
            .route("/status", get(Self::route_status))
            .route("/price/:feed_id/:timestamp", get(Self::route_price_at))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        None
    }
}
