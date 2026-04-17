use crate::{MarketSideKey, Order, OrderbookModule, PriceLevelKey};
use market::MarketId;
use shared_types::{OrderId, Price, Side};
use sov_modules_api::prelude::tokio::time::{sleep, Duration};
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{
    axum::{
        extract::{
            ws::{Message, WebSocket, WebSocketUpgrade},
            Query,
        },
        response::{IntoResponse, Response},
        routing::{any, get},
        Json, Router,
    },
    UnwrapInfallible,
};
use sov_modules_api::rest::utils::{errors, ApiResult, Path};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct UserOrdersQueryParams<S: Spec> {
    user_address: S::Address,
    market_id: Option<MarketId>,
}

impl<S: Spec> OrderbookModule<S> {
    // WebSocket handler for canonical book data
    async fn route_ws(
        Query(params): Query<HashMap<String, String>>,
        ws: WebSocketUpgrade,
        state: ApiState<S, Self>,
        acc: ApiStateAccessor<S>,
    ) -> impl IntoResponse {
        let market_id = params.get("market_id").and_then(|s| s.parse::<u64>().ok());
        ws.on_upgrade(move |socket| Self::handle_orderbook_socket(socket, state, acc, market_id))
    }

    // Stream canonical YES-space book snapshot
    async fn handle_orderbook_socket(
        mut socket: WebSocket,
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
        market_id: Option<u64>,
    ) {
        if let Some(market_id) = market_id {
            let bids = Self::get_orderbook_side(market_id, Side::Bid, &state, &mut acc);
            let asks = Self::get_orderbook_side(market_id, Side::Ask, &state, &mut acc);
            let snapshot = serde_json::json!({
                "market_id": market_id,
                "yes_bids": bids,
                "yes_asks": asks
            });
            let msg = Message::Text(snapshot.to_string());
            let _ = socket.send(msg).await;
        } else {
            let err = serde_json::json!({"error": "Missing or invalid market_id"});
            let _ = socket.send(Message::Text(err.to_string())).await;
        }
    }

    /// Get canonical book side as Vec<[price, quantity]>.
    pub fn get_orderbook_side(
        market_id: u64,
        side: Side,
        state: &ApiState<S, Self>,
        acc: &mut ApiStateAccessor<S>,
    ) -> Vec<[u64; 2]> {
        let mut levels = Vec::new();
        let key = MarketSideKey {
            market_id: MarketId(market_id),
            side,
        };
        if let Some(prices) = state.price_levels.get(&key, acc).ok().flatten() {
            for price in prices {
                let level_key = PriceLevelKey {
                    market_id: MarketId(market_id),
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

    async fn route_user_orders(
        params: Query<UserOrdersQueryParams<S>>,
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
    ) -> ApiResult<Vec<Order<S>>> {
        let user_address = params.user_address;
        let maybe_market_id = params.market_id;

        let orders_ids = state
            .user_orders
            .get(&user_address, &mut acc)
            .unwrap()
            .unwrap_or_default();

        let mut orders = Vec::new();
        for oid in orders_ids {
            let order = state
                .orders
                .get(&oid, &mut acc)
                .unwrap_infallible()
                .ok_or_else(|| errors::not_found_404("Order", oid))?;

            let include = match maybe_market_id {
                Some(market_id) => order.market_id == market_id,
                None => true,
            };

            if include {
                orders.push(order);
            }
        }

        Ok(orders.into())
    }

    async fn route_status() -> ApiResult<String> {
        Ok(Json("OK".to_string()))
    }
}

impl<S: Spec> HasCustomRestApi for OrderbookModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> Router<()> {
        Router::new()
            .route("/status", get(Self::route_status))
            .route("/ws", any(Self::route_ws))
            .route("/user-orders", get(Self::route_user_orders))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        None
    }
}
