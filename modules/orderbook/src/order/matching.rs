use crate::error::IntoOrderbookError;
use crate::{
    BookSideKey, Event, Fill, MatchResult, OrderbookError, OrderbookModule, PriceLevelKey,
};
use market::MarketId;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};
use sov_modules_api::{EventEmitter, Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    /// Match an incoming order against the book.
    pub(crate) fn match_order(
        &mut self,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
        quantity: u64,
        order_type: &OrderType,
        taker: &S::Address,
        taker_order_id: OrderId,
        state: &mut impl TxState<S>,
    ) -> Result<MatchResult, OrderbookError> {
        let mut result = MatchResult {
            remaining: quantity,
            ..Default::default()
        };

        // Get matchable price levels from opposite side
        let opposite_side = side.opposite();
        let levels =
            self.get_matchable_levels(market_id, outcome, opposite_side, price, side, state)?;

        let config = self
            .config
            .get(state)
            .into_orderbook_err()?
            .unwrap_or_default();

        for level_price in levels {
            if result.remaining == 0 {
                break;
            }

            let level_key = PriceLevelKey {
                market_id,
                outcome,
                price: level_price,
            };
            let order_ids = match opposite_side {
                Side::Bid => self.bids.get(&level_key, state).into_orderbook_err()?,
                Side::Ask => self.asks.get(&level_key, state).into_orderbook_err()?,
            }
            .unwrap_or_default();

            let mut remaining_at_level = Vec::new();

            for maker_order_id in order_ids {
                if result.remaining == 0 {
                    remaining_at_level.push(maker_order_id);
                    continue;
                }

                let mut maker_order = match self
                    .orders
                    .get(&maker_order_id, state)
                    .into_orderbook_err()?
                {
                    Some(o) => o,
                    None => continue,
                };

                // Self-trade prevention
                if maker_order.owner == *taker {
                    remaining_at_level.push(maker_order_id);
                    continue;
                }

                // Calculate fill
                let fill_qty = result.remaining.min(maker_order.remaining_quantity);
                let fill_price = maker_order.price;

                // Calculate fees
                let notional = fill_price.cost(fill_qty);
                let maker_fee = (notional * config.maker_fee_bps as u64) / 10000;
                let taker_fee = (notional * config.taker_fee_bps as u64) / 10000;

                result.fills.push(Fill {
                    maker_order_id,
                    taker_order_id,
                    price: fill_price,
                    quantity: fill_qty,
                    maker_fee,
                    taker_fee,
                });

                // Update maker order
                maker_order.remaining_quantity -= fill_qty;
                if maker_order.remaining_quantity == 0 {
                    maker_order.status = OrderStatus::Filled;
                    self.remove_user_order(&maker_order.owner, maker_order_id, state)?;
                } else {
                    remaining_at_level.push(maker_order_id);
                }
                self.orders
                    .set(&maker_order_id, &maker_order, state)
                    .into_orderbook_err()?;

                result.remaining -= fill_qty;
                result.total_filled += fill_qty;

                // Emit trade event
                self.emit_event(
                    state,
                    Event::Trade {
                        market_id,
                        outcome,
                        maker_order_id,
                        taker_order_id,
                        price: fill_price,
                        quantity: fill_qty,
                        maker: maker_order.owner.to_string(),
                        taker: taker.to_string(),
                    },
                );
            }

            // Update price level
            if remaining_at_level.is_empty() {
                match opposite_side {
                    Side::Bid => self.bids.remove(&level_key, state).into_orderbook_err()?,
                    Side::Ask => self.asks.remove(&level_key, state).into_orderbook_err()?,
                };
                self.remove_price_level(market_id, outcome, opposite_side, level_price, state)?;
            } else {
                match opposite_side {
                    Side::Bid => self
                        .bids
                        .set(&level_key, &remaining_at_level, state)
                        .into_orderbook_err()?,
                    Side::Ask => self
                        .asks
                        .set(&level_key, &remaining_at_level, state)
                        .into_orderbook_err()?,
                };
            }
        }

        // Determine if should post
        result.should_post = match order_type {
            OrderType::Limit => true,
            OrderType::PostOnly => result.total_filled == 0,
            OrderType::ImmediateOrCancel | OrderType::Market => false,
            OrderType::FillOrKill => {
                if result.remaining > 0 {
                    return Err(OrderbookError::FillOrKillNotFilled {
                        requested: quantity,
                        available: result.total_filled,
                    });
                }
                false
            }
        };

        // Update best prices
        self.update_best_prices(market_id, outcome, state)?;

        Ok(result)
    }

    fn get_matchable_levels(
        &self,
        market_id: MarketId,
        outcome: OutcomeSide,
        book_side: Side,
        limit_price: Price,
        incoming_side: Side,
        state: &mut impl TxState<S>,
    ) -> Result<Vec<Price>, OrderbookError> {
        let key = BookSideKey {
            market_id,
            outcome,
            side: book_side,
        };
        let mut levels = self
            .price_levels
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        levels.retain(|&p| match incoming_side {
            Side::Bid => p <= limit_price,
            Side::Ask => p >= limit_price,
        });

        // Sort for matching order
        match book_side {
            Side::Ask => levels.sort(),
            Side::Bid => levels.sort_by(|a, b| b.cmp(a)),
        }

        Ok(levels)
    }
}
