use market::MarketId;
use shared_types::{OutcomeSide, Price, Side};
use sov_modules_api::prelude::axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::Query,
    response::{IntoResponse, Response},
    routing::{any, get},
    Json, Router,
};
use sov_modules_api::prelude::tokio::time::{sleep, Duration};
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::rest::utils::{errors, ApiResult, Path};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};

use crate::{BookSideKey, OrderbookModule, PriceLevelKey};

use std::collections::HashMap;

impl<S: Spec> OrderbookModule<S> {
    async fn route_status() -> ApiResult<String> {
        Ok(Json("OK".to_string()))
    }

    // WebSocket handler for orderbook data with query params and real state access
    async fn route_ws(
        ws: WebSocketUpgrade,
        state: ApiState<S, Self>,
        acc: ApiStateAccessor<S>,
        Query(params): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let market_id = params.get("market_id").and_then(|s| s.parse::<u64>().ok());
        ws.on_upgrade(move |socket| Self::handle_orderbook_socket(socket, state, acc, market_id))
    }

    // Stream real orderbook snapshot for the given market_id
    async fn handle_orderbook_socket(
        mut socket: WebSocket,
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        market_id: Option<u64>,
    ) {
        // Only stream if market_id is provided
        if let Some(market_id) = market_id {
            // For both outcomes (YES/NO)
            for &outcome in &[OutcomeSide::Yes, OutcomeSide::No] {
                // Collect bids and asks for this market/outcome
                let bids =
                    Self::get_orderbook_side(market_id, outcome, Side::Bid, &state, &mut acc);
                let asks =
                    Self::get_orderbook_side(market_id, outcome, Side::Ask, &state, &mut acc);
                let snapshot = serde_json::json!({
                    "market_id": market_id,
                    "outcome": format!("{:?}", outcome),
                    "bids": bids,
                    "asks": asks
                });
                let msg = Message::Text(snapshot.to_string());
                if socket.send(msg).await.is_err() {
                    break;
                }
            }
        } else {
            let err = serde_json::json!({"error": "Missing or invalid market_id"});
            let _ = socket.send(Message::Text(err.to_string())).await;
        }
    }

    /// Helper to get orderbook side as Vec<[price, quantity]>
    pub fn get_orderbook_side(
        market_id: u64,
        outcome: OutcomeSide,
        side: Side,
        state: &ApiState<S, Self>,
        acc: &mut ApiStateAccessor<S>,
    ) -> Vec<[u64; 2]> {
        let mut levels = Vec::new();
        let key = BookSideKey {
            market_id: MarketId(market_id),
            outcome,
            side,
        };
        // Get all price levels for this side
        if let Some(prices) = state.price_levels.get(&key, acc).ok().flatten() {
            for price in prices {
                let level_key = PriceLevelKey {
                    market_id: MarketId(market_id),
                    outcome,
                    price: Price(price.0),
                };
                let order_ids = match side {
                    Side::Bid => state.bids.get(&level_key, acc),
                    Side::Ask => state.asks.get(&level_key, acc),
                };
                let mut qty = 0u64;
                if let Ok(Some(ids)) = order_ids {
                    for oid in ids {
                        if let Ok(Some(order)) = state.orders.get(&oid, acc) {
                            qty += order.remaining_quantity;
                        }
                    }
                }
                levels.push([price.0, qty]);
            }
        }
        levels
    }
}

impl<S: Spec> HasCustomRestApi for OrderbookModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> Router<()> {
        Router::new()
            .route("/status", get(Self::route_status))
            .route("/ws", any(Self::route_ws))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        // let mut api: OpenApi = serde_yaml::from_str(include_str!("../../../openapi/orderbook.yaml"))
        //     .expect("Invalid OpenAPI spec");

        // for path_item in api.paths.paths.values_mut() {
        //     path_item.extensions = None;
        // }

        // Some(api)
        None
    }
}
