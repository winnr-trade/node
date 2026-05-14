use axum::routing::get;
use shared_types::{MarketId, Size};
use sov_bank::utils::TokenHolder;
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{axum, UnwrapInfallible};
use sov_modules_api::rest::utils::{errors, ApiResult, Path, Query};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};

use crate::{Market, MarketModule, MarketStatus, Outcome, Position, PositionKey};

const MAX_LIMIT_MARKET_LIST: u64 = 100;
const MAX_LIMIT_ACTIVE_POSITIONS: u64 = 100;

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct MarketListQueryParams {
    page: u64,
    limit: u64,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct MarketSharesParams<S: Spec> {
    market_id: MarketId,
    user_address: S::Address,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct ActivePositionsQueryParams<S: Spec> {
    user_address: S::Address,
    page: u64,
    limit: u64,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct ActivePosition {
    market_id: MarketId,
    question: String,
    outcome: Option<Outcome>,
    yes_shares: Size,
    no_shares: Size,
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
        let from_idx = params.page;
        let limit = params.limit.min(MAX_LIMIT_MARKET_LIST);

        let mut markets = Vec::new();
        for idx in from_idx..(from_idx + limit) {
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

    async fn route_shares(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        params: Query<MarketSharesParams<S>>,
    ) -> ApiResult<Position> {
        let position_id = PositionKey {
            market_id: params.market_id,
            owner: TokenHolder::User(params.user_address),
        };

        let position = state
            .positions
            .get(&position_id, &mut acc)
            .unwrap_infallible()
            .unwrap_or_default();

        Ok(position.into())
    }

    async fn route_active_positions(
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        params: Query<ActivePositionsQueryParams<S>>,
    ) -> ApiResult<Vec<ActivePosition>> {
        let now_ms = state
            .chain_state
            .get_time(&mut acc)
            .unwrap_infallible()
            .as_millis() as u64;
        let from_idx = params.page;
        let limit = params.limit.min(MAX_LIMIT_ACTIVE_POSITIONS);

        let user_active_markets = state
            .user_active_markets
            .get(&params.user_address, &mut acc)
            .unwrap_infallible()
            .unwrap_or_default();

        let mut active_positions = Vec::new();

        for market_id in user_active_markets
            .into_iter()
            .skip(from_idx as usize)
            .take(limit as usize)
        {
            let market = match state.markets.get(&market_id, &mut acc).unwrap_infallible() {
                Some(market) => market,
                None => continue,
            };

            if market.status() != MarketStatus::Active || now_ms >= market.resolution_time {
                continue;
            }

            let position_key = PositionKey {
                market_id,
                owner: TokenHolder::User(params.user_address.clone()),
            };
            let position = match state
                .positions
                .get(&position_key, &mut acc)
                .unwrap_infallible()
            {
                Some(position) => position,
                None => continue,
            };

            if position.is_empty() {
                continue;
            }

            active_positions.push(ActivePosition {
                market_id,
                question: market.question.to_string(),
                outcome: market.outcome,
                yes_shares: position.yes_shares,
                no_shares: position.no_shares,
            });
        }

        Ok(active_positions.into())
    }
}

impl<S: Spec> HasCustomRestApi for MarketModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> axum::Router<()> {
        axum::Router::new()
            .route("/list", get(Self::route_market_list))
            .route("/positions", get(Self::route_active_positions))
            .route("/{marketId}", get(Self::route_market))
            .route("/shares", get(Self::route_shares))
            .route("/status", get(Self::route_status))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        None
    }
}
