use crate::error::IntoOrderbookError;
use crate::{
    BookKey, BookSideKey, Event, Fill, Order, OrderbookError, OrderbookModule, PriceLevelKey,
};
use market::MarketId;
use shared_types::{OrderId, OutcomeSide, Price, Side};
use sov_modules_api::{EventEmitter, Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    pub(crate) fn add_to_book(
        &mut self,
        order: &Order<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let level_key = PriceLevelKey {
            market_id: order.market_id,
            outcome: order.outcome,
            price: order.price,
        };

        let mut order_ids = match order.side {
            Side::Bid => self.bids.get(&level_key, state).into_orderbook_err()?,
            Side::Ask => self.asks.get(&level_key, state).into_orderbook_err()?,
        }
        .unwrap_or_default();

        order_ids.push(order.id);

        match order.side {
            Side::Bid => self
                .bids
                .set(&level_key, &order_ids, state)
                .into_orderbook_err()?,
            Side::Ask => self
                .asks
                .set(&level_key, &order_ids, state)
                .into_orderbook_err()?,
        };

        self.add_price_level(
            order.market_id,
            order.outcome,
            order.side,
            order.price,
            state,
        )?;
        self.update_best_prices(order.market_id, order.outcome, state)?;

        Ok(())
    }

    pub(crate) fn add_price_level(
        &mut self,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = BookSideKey {
            market_id,
            outcome,
            side,
        };
        let mut levels = self
            .price_levels
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        if !levels.contains(&price) {
            levels.push(price);
            match side {
                Side::Bid => levels.sort_by(|a, b| b.cmp(a)),
                Side::Ask => levels.sort(),
            }
            self.price_levels
                .set(&key, &levels, state)
                .into_orderbook_err()?;
        }

        Ok(())
    }

    pub(crate) fn remove_price_level(
        &mut self,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = BookSideKey {
            market_id,
            outcome,
            side,
        };
        let mut levels = self
            .price_levels
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        levels.retain(|&p| p != price);

        if levels.is_empty() {
            self.price_levels.remove(&key, state).into_orderbook_err()?;
        } else {
            self.price_levels
                .set(&key, &levels, state)
                .into_orderbook_err()?;
        }

        Ok(())
    }

    pub(crate) fn update_best_prices(
        &mut self,
        market_id: MarketId,
        outcome: OutcomeSide,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let book_key = BookKey { market_id, outcome };

        // Best bid
        let bid_key = BookSideKey {
            market_id,
            outcome,
            side: Side::Bid,
        };
        let bid_levels = self
            .price_levels
            .get(&bid_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        if let Some(&best) = bid_levels.first() {
            self.best_bid
                .set(&book_key, &best, state)
                .into_orderbook_err()?;
        } else {
            self.best_bid
                .remove(&book_key, state)
                .into_orderbook_err()?;
        }

        // Best ask
        let ask_key = BookSideKey {
            market_id,
            outcome,
            side: Side::Ask,
        };
        let ask_levels = self
            .price_levels
            .get(&ask_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        if let Some(&best) = ask_levels.first() {
            self.best_ask
                .set(&book_key, &best, state)
                .into_orderbook_err()?;
        } else {
            self.best_ask
                .remove(&book_key, state)
                .into_orderbook_err()?;
        }

        self.emit_event(
            state,
            Event::BookUpdated {
                market_id,
                outcome,
                best_bid: bid_levels.first().copied(),
                best_ask: ask_levels.first().copied(),
            },
        );

        Ok(())
    }

    pub(crate) fn add_user_order(
        &mut self,
        user: &S::Address,
        order_id: OrderId,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let mut orders = self
            .user_orders
            .get(user, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        orders.push(order_id);
        self.user_orders
            .set(user, &orders, state)
            .into_orderbook_err()?;
        Ok(())
    }

    pub(crate) fn remove_user_order(
        &mut self,
        user: &S::Address,
        order_id: OrderId,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let mut orders = self
            .user_orders
            .get(user, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        orders.retain(|&id| id != order_id);

        if orders.is_empty() {
            self.user_orders.remove(user, state).into_orderbook_err()?;
        } else {
            self.user_orders
                .set(user, &orders, state)
                .into_orderbook_err()?;
        }

        Ok(())
    }

    pub(crate) fn execute_fill(
        &mut self,
        _market_id: MarketId,
        _outcome: OutcomeSide,
        _taker_side: Side,
        _fill: &Fill,
        _taker: &S::Address,
        _state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        // TODO: Implement actual share and collateral transfers
        // This requires integration with prediction_market positions
        // and bank module for collateral
        Ok(())
    }

    pub(crate) fn remove_from_book(
        &mut self,
        order: &Order<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let level_key = PriceLevelKey {
            market_id: order.market_id,
            outcome: order.outcome,
            price: order.price,
        };

        let mut order_ids = match order.side {
            Side::Bid => self.bids.get(&level_key, state).into_orderbook_err()?,
            Side::Ask => self.asks.get(&level_key, state).into_orderbook_err()?,
        }
        .unwrap_or_default();

        order_ids.retain(|&id| id != order.id);

        if order_ids.is_empty() {
            match order.side {
                Side::Bid => self.bids.remove(&level_key, state).into_orderbook_err()?,
                Side::Ask => self.asks.remove(&level_key, state).into_orderbook_err()?,
            };
            self.remove_price_level(
                order.market_id,
                order.outcome,
                order.side,
                order.price,
                state,
            )?;
        } else {
            match order.side {
                Side::Bid => self
                    .bids
                    .set(&level_key, &order_ids, state)
                    .into_orderbook_err()?,
                Side::Ask => self
                    .asks
                    .set(&level_key, &order_ids, state)
                    .into_orderbook_err()?,
            };
        }

        self.update_best_prices(order.market_id, order.outcome, state)?;

        Ok(())
    }
}
