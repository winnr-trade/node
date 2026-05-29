use crate::{MarketSideKey, OrderbookModule, PriceLevelKey};
use market::MarketId;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side, Size};
use sov_bank::Amount;
use sov_modules_api::prelude::utoipa::openapi::OpenApi;
use sov_modules_api::prelude::{
    axum::{
        extract::{
            ws::{Message, WebSocket, WebSocketUpgrade},
            Query,
        },
        response::IntoResponse,
        routing::{any, get},
        Json, Router,
    },
    UnwrapInfallible,
};
use sov_modules_api::rest::utils::{errors, ApiResult};
use sov_modules_api::rest::{ApiState, HasCustomRestApi};
use sov_modules_api::{ApiStateAccessor, Spec};
use std::collections::HashMap;

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
struct UserOrdersQueryParams<S: Spec> {
    user_address: S::Address,
    market_id: Option<MarketId>,
}

#[derive(Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize, Clone)]
#[serde(bound(serialize = "", deserialize = ""))]
struct UserOrdersResponse<S: Spec> {
    id: OrderId,
    market_id: MarketId,
    outcome: OutcomeSide,
    side: Side,
    canonical_side: Side,
    canonical_price: Price,
    original_quantity: Size,
    remaining_quantity: Size,
    owner: S::Address,
    order_type: OrderType,
    created_at: u64,
    status: OrderStatus,
    market_question: String,
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
            let msg = Message::Text(snapshot.to_string().into());
            let _ = socket.send(msg).await;
        } else {
            let err = serde_json::json!({"error": "Missing or invalid market_id"});
            let _ = socket.send(Message::Text(err.to_string().into())).await;
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
                let mut qty = Size::ZERO;
                if let Ok(Some(ids)) = order_ids {
                    for oid in ids {
                        if let Ok(Some(order)) = state.orders.get(&oid, acc) {
                            qty = qty.saturating_add(order.remaining_quantity);
                        }
                    }
                }
                levels.push([price.0, qty.0]);
            }
        }
        levels
    }

    async fn route_user_orders(
        params: Query<UserOrdersQueryParams<S>>,
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
    ) -> ApiResult<Vec<UserOrdersResponse<S>>> {
        let user_address = params.user_address;
        let maybe_market_id = params.market_id;

        let order_ids = state
            .user_orders
            .get(&user_address, &mut acc)
            .unwrap_infallible()
            .unwrap_or_default();

        let mut orders = Vec::new();
        let mut market_questions: HashMap<MarketId, String> = HashMap::new();

        for oid in order_ids {
            let order = state
                .orders
                .get(&oid, &mut acc)
                .unwrap_infallible()
                .ok_or_else(|| errors::not_found_404("Order", oid))?;

            let include = match maybe_market_id {
                Some(market_id) => order.market_id == market_id,
                None => true,
            };

            if !include {
                continue;
            }

            let market_question = if let Some(question) = market_questions.get(&order.market_id) {
                question.clone()
            } else {
                let market = state
                    .market
                    .markets
                    .get(&order.market_id, &mut acc)
                    .unwrap_infallible()
                    .ok_or_else(|| errors::not_found_404("Market", order.market_id))?;
                let question = market.question.to_string();
                market_questions.insert(order.market_id, question.clone());
                question
            };

            orders.push(UserOrdersResponse {
                id: order.id,
                market_id: order.market_id,
                market_question,
                outcome: order.outcome,
                side: order.side,
                canonical_side: order.canonical_side,
                canonical_price: order.canonical_price,
                original_quantity: order.original_quantity,
                remaining_quantity: order.remaining_quantity,
                owner: order.owner,
                order_type: order.order_type,
                created_at: order.created_at,
                status: order.status,
            });
        }

        Ok(orders.into())
    }

    async fn route_status() -> ApiResult<String> {
        Ok(Json("OK".to_string()))
    }

    /// Simulate a market BUY of `quantity` shares of `outcome` and return the
    /// total collateral required, along with how much of the quantity is
    /// fillable from current book depth.
    ///
    /// - BUY YES: walks ASK levels cheapest-first; cost per fill = ask_price × qty / BASIS
    /// - BUY NO:  walks BID levels highest-first; cost per fill = (BASIS − bid_price) × qty / BASIS
    async fn route_buy_quote(
        params: Query<BuyQuoteParams>,
        state: ApiState<S, Self>,
        mut acc: ApiStateAccessor<S>,
    ) -> ApiResult<BuyQuoteResponse> {
        let market_id = MarketId(params.market_id);
        let quantity = Size(params.quantity);

        let market = state
            .market
            .markets
            .get(&market_id, &mut acc)
            .unwrap_infallible()
            .ok_or_else(|| errors::not_found_404("Market", market_id))?;

        let token = market.collateral_token;

        // BUY YES taker matches against ASK side.
        // BUY NO (canonical Ask) taker matches against BID side.
        let book_side = match params.outcome {
            OutcomeSide::Yes => Side::Ask,
            OutcomeSide::No => Side::Bid,
        };

        let key = MarketSideKey { market_id, side: book_side };
        let mut levels = state
            .price_levels
            .get(&key, &mut acc)
            .unwrap_infallible()
            .unwrap_or_default();

        match book_side {
            Side::Ask => levels.sort(),
            Side::Bid => levels.sort_by(|a, b| b.cmp(a)),
        }

        let mut remaining = quantity;
        let mut collateral = Amount::ZERO;

        'outer: for price_level in levels {
            let level_key = PriceLevelKey { market_id, price: price_level };
            let order_ids = match book_side {
                Side::Bid => &state.bids,
                Side::Ask => &state.asks,
            }
            .get(&level_key, &mut acc)
            .unwrap_infallible()
            .unwrap_or_default();

            for oid in order_ids {
                if remaining.is_zero() {
                    break 'outer;
                }

                let order = match state.orders.get(&oid, &mut acc).unwrap_infallible() {
                    Some(o) => o,
                    None => continue,
                };

                let fill_qty = remaining.min(order.remaining_quantity);
                let cost = match params.outcome {
                    OutcomeSide::Yes => price_level.cost(fill_qty, &token),
                    OutcomeSide::No => price_level.complement().cost(fill_qty, &token),
                };

                collateral = collateral.saturating_add(cost);
                remaining = remaining.saturating_sub(fill_qty);
            }
        }

        let fillable = quantity.saturating_sub(remaining);

        Ok(Json(BuyQuoteResponse {
            collateral_required: collateral.0,
            fillable_quantity: fillable.0,
            unfillable_quantity: remaining.0,
        }))
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BuyQuoteParams {
    market_id: u64,
    quantity: u64,
    outcome: OutcomeSide,
}

#[derive(Debug, serde::Serialize)]
struct BuyQuoteResponse {
    /// Total collateral (in base units) needed for the fillable portion.
    collateral_required: u128,
    /// Shares fillable from current book depth.
    fillable_quantity: u64,
    /// Shares not fillable (book too thin).
    unfillable_quantity: u64,
}

impl<S: Spec> HasCustomRestApi for OrderbookModule<S> {
    type Spec = S;

    fn custom_rest_api(&self, state: ApiState<Self::Spec>) -> Router<()> {
        Router::new()
            .route("/status", get(Self::route_status))
            .route("/ws", any(Self::route_ws))
            .route("/user-orders", get(Self::route_user_orders))
            .route("/buy-quote", get(Self::route_buy_quote))
            .with_state(state.with(self.clone()))
    }

    fn custom_openapi_spec(&self) -> Option<OpenApi> {
        None
    }
}
