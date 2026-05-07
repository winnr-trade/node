use crate::error::IntoOrderbookError;
use crate::{
    Event, Fill, MarketSideKey, MatchResult, OrderbookError, OrderbookModule, PriceLevelKey,
    SettlementKind,
};
use market::MarketId;
use shared_types::{OrderId, OrderStatus, OrderType, Price, Side, Size};
use sov_bank::{Amount, TokenId};
use sov_modules_api::{EventEmitter, Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    /// Compute matches for an incoming order against the book.
    ///
    /// Side-agnostic: operates purely on bid/ask book structure.
    /// Read-only: produces fills without modifying book state or emitting events.
    /// The caller is responsible for executing the returned fills via `execute_matches`.
    pub(crate) fn compute_matches(
        &self,
        market_id: MarketId,
        side: Side,
        price: Price,
        quantity: Size,
        taker: &S::Address,
        order_type: OrderType,
        collateral_token: &TokenId,
        state: &mut impl TxState<S>,
    ) -> Result<MatchResult, OrderbookError> {
        let opposite_side = side.opposite();

        // Market orders match at any price; limit orders match up to their limit price
        let limit_price = match order_type {
            OrderType::Market => None,
            _ => Some(price),
        };

        let price_levels =
            self.get_matchable_price_levels(market_id, opposite_side, limit_price, side, state)?;

        let config = self
            .config
            .get(state)
            .into_orderbook_err()?
            .unwrap_or_default();

        let mut remaining = quantity;
        let mut total_filled = Size::ZERO;
        let mut fills = Vec::new();

        for price_level in price_levels {
            if remaining.is_zero() {
                break;
            }

            let level_key = PriceLevelKey {
                market_id,
                price: price_level,
            };
            let book = match opposite_side {
                Side::Bid => &self.bids,
                Side::Ask => &self.asks,
            };
            let order_ids = book
                .get(&level_key, state)
                .into_orderbook_err()?
                .unwrap_or_default();

            for maker_order_id in order_ids {
                if remaining.is_zero() {
                    break;
                }

                let maker_order = match self
                    .orders
                    .get(&maker_order_id, state)
                    .into_orderbook_err()?
                {
                    Some(o) => o,
                    None => continue,
                };

                if maker_order.owner == *taker {
                    continue;
                }

                let fill_qty = remaining.min(maker_order.remaining_quantity);
                let fill_price = maker_order.canonical_price;
                let notional = fill_price.cost(fill_qty, collateral_token);

                fills.push(Fill {
                    order_id: maker_order_id,
                    price: fill_price,
                    quantity: fill_qty,
                    maker_fee: Amount((notional.0 * config.maker_fee_bps as u128) / 10000),
                    taker_fee: Amount((notional.0 * config.taker_fee_bps as u128) / 10000),
                });

                remaining = remaining.saturating_sub(fill_qty);
                total_filled = total_filled.saturating_add(fill_qty);
            }
        }

        Ok(MatchResult {
            fills,
            total_quantity_filled: total_filled,
            remaining_quantity: remaining,
        })
    }

    /// Execute all fills from matching: classify settlement, settle positions,
    /// update maker orders, maintain book, emit events.
    pub(crate) fn execute_matches(
        &mut self,
        market_id: MarketId,
        canonical_side: Side,
        taker: &S::Address,
        taker_order_id: OrderId,
        taker_side: Side,
        match_result: &MatchResult,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let timestamp = self
            .chain_state
            .get_time(state)
            .into_orderbook_err()?
            .as_millis() as u64;

        for fill in &match_result.fills {
            let mut maker_order = self
                .orders
                .get(&fill.order_id, state)
                .into_orderbook_err()?
                .ok_or(OrderbookError::OrderNotFound {
                    order_id: fill.order_id,
                })?;

            let (canonical_buyer, canonical_seller, buyer_side, seller_side) = match canonical_side
            {
                Side::Bid => (taker, &maker_order.owner, taker_side, maker_order.side),
                Side::Ask => (&maker_order.owner, taker, maker_order.side, taker_side),
            };
            let settlement_kind = SettlementKind::classify(buyer_side, seller_side);

            self.execute_fill(
                market_id,
                fill,
                settlement_kind,
                canonical_buyer,
                canonical_seller,
                state,
            )?;

            maker_order.remaining_quantity = maker_order
                .remaining_quantity
                .checked_sub(fill.quantity)
                .ok_or_else(|| {
                    OrderbookError::Any(anyhow::anyhow!(
                        "Underflow in remaining_quantity for order {}",
                        fill.order_id
                    ))
                })?;

            // If maker order fully filled, remove from book and user orders.
            if maker_order.remaining_quantity.is_zero() {
                maker_order.status = OrderStatus::Filled;
                self.remove_order(&maker_order, state)?;
            }

            self.orders
                .set(&fill.order_id, &maker_order, state)
                .into_orderbook_err()?;

            self.emit_event(
                state,
                Event::Trade {
                    market_id,
                    maker_order_id: fill.order_id,
                    taker_order_id,
                    price: fill.price,
                    quantity: fill.quantity,
                    buyer: canonical_buyer.to_string(),
                    seller: canonical_seller.to_string(),
                    settlement_kind,
                    timestamp,
                },
            );
        }

        if !match_result.fills.is_empty() {
            self.update_best_prices(market_id, state)?;
        }

        Ok(())
    }

    /// Get matchable price levels on the opposite side.
    ///
    /// Returns levels in matching order (best price first).
    /// If `limit_price` is None (market order), includes all levels.
    /// If `limit_price` is Some, filters to levels within the limit.
    fn get_matchable_price_levels(
        &self,
        market_id: MarketId,
        book_side: Side,
        limit_price: Option<Price>,
        incoming_side: Side,
        state: &mut impl TxState<S>,
    ) -> Result<Vec<Price>, OrderbookError> {
        let key = MarketSideKey {
            market_id,
            side: book_side,
        };
        let mut levels = self
            .price_levels
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        // Filter by limit price only if specified (not a market order)
        if let Some(limit_price) = limit_price {
            levels.retain(|&p| match incoming_side {
                Side::Bid => p <= limit_price,
                Side::Ask => p >= limit_price,
            });
        }

        match book_side {
            Side::Ask => levels.sort(),
            Side::Bid => levels.sort_by(|a, b| b.cmp(a)),
        }

        Ok(levels)
    }
}
