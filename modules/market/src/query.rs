use axum::routing::get;
use shared_types::MarketId;
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{axum, serde_yaml, UnwrapInfallible};
use sov_modules_api::rest::utils::{errors, ApiResult, Path, Query};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};

use crate::{Market, MarketModule, Position, PositionKey};

const LIMIT_MARKET_LIST: u64 = 10;

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct MarketListQueryParams {
    from_id: MarketId,
}

impl<S: Spec> MarketModule<S> {
    async fn route_status() -> ApiResult<String> {
        Ok(axum::Json("OK".to_string()))
    }

    async fn route_market_list(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        params: Query<MarketListQueryParams>,
    ) -> ApiResult<Vec<Market<S>>> {
        let from_idx = params.from_id.0;

        let mut markets = Vec::new();
        for idx in from_idx..(from_idx + LIMIT_MARKET_LIST) {
            let market_id = MarketId(idx);
            let maybe_market = state.markets.get(&market_id, &mut acc).unwrap_infallible();

            if let Some(market) = maybe_market {
                markets.push(market);
            } else {
                break;
            }
        }

        Ok(markets.into())
    }

    async fn route_market(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        Path(market_id): Path<MarketId>,
    ) -> ApiResult<Market<S>> {
        let market = state
            .markets
            .get(&market_id, &mut acc)
            .unwrap_infallible()
            .ok_or_else(|| errors::not_found_404("Market", market_id))?;

        Ok(market.into())
    }

    async fn route_outcome_shares(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        Path((market_id, user_address)): Path<(MarketId, S::Address)>,
    ) -> ApiResult<Position> {
        let position_id = PositionKey {
            market_id,
            address: user_address,
        };

        let position = state
            .positions
            .get(&position_id, &mut acc)
            .unwrap_infallible()
            .unwrap_or_default();

        Ok(position.into())
    }
}

impl<S: Spec> HasCustomRestApi for MarketModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> axum::Router<()> {
        axum::Router::new()
            .route("/status", get(Self::route_status))
            .route("/list", get(Self::route_market_list))
            .route("/:marketId", get(Self::route_market))
            .route(
                "/:marketId/shares/:userAddress",
                get(Self::route_outcome_shares),
            )
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        // let mut api: OpenApi = serde_yaml::from_str(include_str!("../../../openapi/market.yaml"))
        //     .expect("Invalid OpenAPI spec");

        // for path_item in api.paths.paths.values_mut() {
        //     path_item.extensions = None;
        // }

        // Some(api)
        None
    }
}
