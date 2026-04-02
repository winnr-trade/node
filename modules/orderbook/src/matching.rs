//! Matching engine implementation.

use crate::error::IntoOrderbookError;
use crate::event::{CancelReason, Event};
use crate::keys::{BookKey, BookSideKey, PriceLevelKey, UserMarketKey};
use crate::types::{Fill, MatchResult, Order};
use crate::{OrderbookError, OrderbookModule};
use shared_types::{
    MarketId, MarketStatus, OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side,
};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};
use tracing::debug;

impl<S: Spec> OrderbookModule<S> {
    /// Place a new order.
    // fn _place_order(
    //     &mut self,
    //     sender: &S::Address,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     side: Side,
    //     price: Price,
    //     quantity: u64,
    //     order_type: OrderType,
    //     context: &Context<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     // Validate
    //     self.validate_order(&price, quantity, &order_type, state)?;
    //     self.verify_market_active(market_id, state)?;

    //     // Generate order ID
    //     let order_id = self.next_order_id(state)?;

    //     // Check PostOnly before matching
    //     if order_type == OrderType::PostOnly
    //         && self
    //             .would_match(market_id, outcome, side, price, state)
    //             .into_orderbook_err()?
    //     {
    //         return Err(OrderbookError::PostOnlyWouldMatch);
    //     }

    //     // Lock collateral for bids
    //     if side == Side::Bid {
    //         let required_amount = price.cost(quantity);
    //         self.lock_collateral(sender, market_id, required_amount, state)?;
    //     }

    //     // Run matching
    //     let match_result = self.match_order(
    //         market_id,
    //         outcome,
    //         side,
    //         price,
    //         quantity,
    //         &order_type,
    //         sender,
    //         order_id,
    //         state,
    //     )?;

    //     // Execute fills
    //     for fill in &match_result.fills {
    //         self.execute_fill(market_id, outcome, side, fill, sender, state)?;
    //     }

    //     // Get current time for order creation
    //     let created_at = self
    //         .chain_state
    //         .get_time(state)
    //         .into_orderbook_err()?
    //         .as_millis() as u64;

    //     // Post remaining to book if applicable
    //     if match_result.should_post && match_result.remaining > 0 {
    //         let order = Order {
    //             id: order_id,
    //             market_id,
    //             outcome,
    //             side,
    //             price,
    //             original_quantity: quantity,
    //             remaining_quantity: match_result.remaining,
    //             owner: sender.clone(),
    //             order_type,
    //             created_at,
    //             status: if match_result.total_filled > 0 {
    //                 OrderStatus::PartiallyFilled
    //             } else {
    //                 OrderStatus::Open
    //             },
    //         };

    //         self.orders
    //             .set(&order_id, &order, state)
    //             .into_orderbook_err()?;
    //         self.add_to_book(&order, state)?;
    //         self.add_user_order(sender, order_id, state)?;

    //         debug!(order_id = %order_id, "Order posted to book");
    //     }

    //     // Emit order placed event
    //     self.emit_event(
    //         state,
    //         Event::OrderPlaced {
    //             order_id,
    //             market_id,
    //             outcome,
    //             side,
    //             price,
    //             quantity,
    //             order_type,
    //             owner: sender.to_string(),
    //         },
    //     );

    //     // Emit fill event if any
    //     if match_result.total_filled > 0 {
    //         let avg_price = if match_result.total_filled > 0 {
    //             match_result
    //                 .fills
    //                 .iter()
    //                 .map(|f| f.price.0 * f.quantity)
    //                 .sum::<u64>()
    //                 / match_result.total_filled
    //         } else {
    //             0
    //         };

    //         self.emit_event(
    //             state,
    //             Event::OrderFilled {
    //                 order_id,
    //                 filled_quantity: match_result.total_filled,
    //                 remaining_quantity: match_result.remaining,
    //                 average_price: avg_price,
    //             },
    //         );
    //     }

    //     Ok(())
    // }

    /// Place a new order.
    // pub(crate) fn place_order_normal(
    //     &mut self,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     side: Side,
    //     price: Price,
    //     quantity: u64,
    //     order_type: OrderType,
    //     context: &Context<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     self.place_order(
    //         context.sender(),
    //         market_id,
    //         outcome,
    //         side,
    //         price,
    //         quantity,
    //         order_type,
    //         context,
    //         state,
    //     )
    // }

    // pub(crate) fn place_order_stealth(
    //     &mut self,
    //     stealth_address: &S::Address,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     side: Side,
    //     price: Price,
    //     quantity: u64,
    //     order_type: OrderType,
    //     context: &Context<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     self.place_order(
    //         sender, market_id, outcome, side, price, quantity, order_type, context, state,
    //     )
    // }

    /// Match an incoming order against the book.
    // pub(crate) fn match_order(
    //     &mut self,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     side: Side,
    //     price: Price,
    //     quantity: u64,
    //     order_type: &OrderType,
    //     taker: &S::Address,
    //     taker_order_id: OrderId,
    //     state: &mut impl TxState<S>,
    // ) -> Result<MatchResult, OrderbookError> {
    //     let mut result = MatchResult {
    //         remaining: quantity,
    //         ..Default::default()
    //     };

    //     // Get matchable price levels from opposite side
    //     let opposite_side = side.opposite();
    //     let levels =
    //         self.get_matchable_levels(market_id, outcome, opposite_side, price, side, state)?;

    //     let config = self
    //         .config
    //         .get(state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();

    //     for level_price in levels {
    //         if result.remaining == 0 {
    //             break;
    //         }

    //         let level_key = PriceLevelKey {
    //             market_id,
    //             outcome,
    //             price: level_price,
    //         };
    //         let order_ids = match opposite_side {
    //             Side::Bid => self.bids.get(&level_key, state).into_orderbook_err()?,
    //             Side::Ask => self.asks.get(&level_key, state).into_orderbook_err()?,
    //         }
    //         .unwrap_or_default();

    //         let mut remaining_at_level = Vec::new();

    //         for maker_order_id in order_ids {
    //             if result.remaining == 0 {
    //                 remaining_at_level.push(maker_order_id);
    //                 continue;
    //             }

    //             let mut maker_order = match self
    //                 .orders
    //                 .get(&maker_order_id, state)
    //                 .into_orderbook_err()?
    //             {
    //                 Some(o) => o,
    //                 None => continue,
    //             };

    //             // Self-trade prevention
    //             if maker_order.owner == *taker {
    //                 remaining_at_level.push(maker_order_id);
    //                 continue;
    //             }

    //             // Calculate fill
    //             let fill_qty = result.remaining.min(maker_order.remaining_quantity);
    //             let fill_price = maker_order.price;

    //             // Calculate fees
    //             let notional = fill_price.cost(fill_qty);
    //             let maker_fee = (notional * config.maker_fee_bps as u64) / 10000;
    //             let taker_fee = (notional * config.taker_fee_bps as u64) / 10000;

    //             result.fills.push(Fill {
    //                 maker_order_id,
    //                 taker_order_id,
    //                 price: fill_price,
    //                 quantity: fill_qty,
    //                 maker_fee,
    //                 taker_fee,
    //             });

    //             // Update maker order
    //             maker_order.remaining_quantity -= fill_qty;
    //             if maker_order.remaining_quantity == 0 {
    //                 maker_order.status = OrderStatus::Filled;
    //                 self.remove_user_order(&maker_order.owner, maker_order_id, state)?;
    //             } else {
    //                 remaining_at_level.push(maker_order_id);
    //             }
    //             self.orders
    //                 .set(&maker_order_id, &maker_order, state)
    //                 .into_orderbook_err()?;

    //             result.remaining -= fill_qty;
    //             result.total_filled += fill_qty;

    //             // Emit trade event
    //             self.emit_event(
    //                 state,
    //                 Event::Trade {
    //                     market_id,
    //                     outcome,
    //                     maker_order_id,
    //                     taker_order_id,
    //                     price: fill_price,
    //                     quantity: fill_qty,
    //                     maker: maker_order.owner.to_string(),
    //                     taker: taker.to_string(),
    //                 },
    //             );
    //         }

    //         // Update price level
    //         if remaining_at_level.is_empty() {
    //             match opposite_side {
    //                 Side::Bid => self.bids.remove(&level_key, state).into_orderbook_err()?,
    //                 Side::Ask => self.asks.remove(&level_key, state).into_orderbook_err()?,
    //             };
    //             self.remove_price_level(market_id, outcome, opposite_side, level_price, state)?;
    //         } else {
    //             match opposite_side {
    //                 Side::Bid => self
    //                     .bids
    //                     .set(&level_key, &remaining_at_level, state)
    //                     .into_orderbook_err()?,
    //                 Side::Ask => self
    //                     .asks
    //                     .set(&level_key, &remaining_at_level, state)
    //                     .into_orderbook_err()?,
    //             };
    //         }
    //     }

    //     // Determine if should post
    //     result.should_post = match order_type {
    //         OrderType::Limit => true,
    //         OrderType::PostOnly => result.total_filled == 0,
    //         OrderType::ImmediateOrCancel | OrderType::Market => false,
    //         OrderType::FillOrKill => {
    //             if result.remaining > 0 {
    //                 return Err(OrderbookError::FillOrKillNotFilled {
    //                     requested: quantity,
    //                     available: result.total_filled,
    //                 });
    //             }
    //             false
    //         }
    //     };

    //     // Update best prices
    //     self.update_best_prices(market_id, outcome, state)?;

    //     Ok(result)
    // }

    // /// Cancel an order.
    // pub(crate) fn cancel_order(
    //     &mut self,
    //     order_id: OrderId,
    //     context: &Context<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let mut order = self
    //         .orders
    //         .get(&order_id, state)
    //         .into_orderbook_err()?
    //         .ok_or(OrderbookError::OrderNotFound { order_id })?;

    //     // Verify ownership
    //     if order.owner != *context.sender() {
    //         return Err(OrderbookError::NotOrderOwner {
    //             order_id,
    //             owner: order.owner.to_string(),
    //             sender: context.sender().to_string(),
    //         });
    //     }

    //     // Must be open
    //     if order.status != OrderStatus::Open && order.status != OrderStatus::PartiallyFilled {
    //         return Err(OrderbookError::OrderNotCancellable {
    //             order_id,
    //             status: format!("{:?}", order.status),
    //         });
    //     }

    //     let unfilled = order.remaining_quantity;

    //     // Remove from book
    //     self.remove_from_book(&order, state)?;

    //     // Unlock collateral
    //     if order.side == Side::Bid {
    //         let locked = order.price.cost(order.remaining_quantity);
    //         self.unlock_collateral(&order.owner, order.market_id, locked, state)?;
    //     }

    //     // Update order
    //     order.status = OrderStatus::Cancelled;
    //     order.remaining_quantity = 0;
    //     self.orders
    //         .set(&order_id, &order, state)
    //         .into_orderbook_err()?;

    //     // Remove from user orders
    //     self.remove_user_order(&order.owner, order_id, state)?;

    //     self.emit_event(
    //         state,
    //         Event::OrderCancelled {
    //             order_id,
    //             reason: CancelReason::UserRequested,
    //             unfilled_quantity: unfilled,
    //         },
    //     );

    //     Ok(())
    // }

    // /// Cancel all orders for a market.
    // pub(crate) fn cancel_all_orders(
    //     &mut self,
    //     market_id: MarketId,
    //     outcome: Option<OutcomeSide>,
    //     context: &Context<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let order_ids = self
    //         .user_orders
    //         .get(context.sender(), state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();

    //     for order_id in order_ids {
    //         if let Some(order) = self.orders.get(&order_id, state).into_orderbook_err()? {
    //             if order.market_id == market_id {
    //                 if outcome.is_none() || outcome == Some(order.outcome) {
    //                     // Ignore errors for individual cancels
    //                     let _ = self.cancel_order(order_id, context, state);
    //                 }
    //             }
    //         }
    //     }

    //     Ok(())
    // }

    // /// Amend an order (cancel and replace).
    // pub(crate) fn amend_order(
    //     &mut self,
    //     order_id: OrderId,
    //     new_price: Option<Price>,
    //     new_quantity: Option<u64>,
    //     context: &Context<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let order = self
    //         .orders
    //         .get(&order_id, state)
    //         .into_orderbook_err()?
    //         .ok_or(OrderbookError::OrderNotFound { order_id })?;

    //     // Cancel existing
    //     self.cancel_order(order_id, context, state)?;

    //     // Place new with amended params
    //     self.place_order(
    //         order.market_id,
    //         order.outcome,
    //         order.side,
    //         new_price.unwrap_or(order.price),
    //         new_quantity.unwrap_or(order.remaining_quantity),
    //         order.order_type,
    //         context,
    //         state,
    //     )
    // }

    // ========================================================================
    // HELPERS
    // ========================================================================

    // fn next_order_id(&mut self, state: &mut impl TxState<S>) -> Result<OrderId, OrderbookError> {
    //     let id = self
    //         .next_order_id
    //         .get(state)
    //         .into_orderbook_err()?
    //         .ok_or_else(|| anyhow::anyhow!("Module not initialized"))?;
    //     self.next_order_id
    //         .set(&(id + 1), state)
    //         .into_orderbook_err()?;
    //     Ok(OrderId(id))
    // }

    // fn validate_order(
    //     &self,
    //     price: &Price,
    //     quantity: u64,
    //     order_type: &OrderType,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let config = self
    //         .config
    //         .get(state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();

    //     if quantity == 0 {
    //         return Err(OrderbookError::ZeroQuantity);
    //     }

    //     if quantity < config.min_order_size {
    //         return Err(OrderbookError::OrderTooSmall {
    //             size: quantity,
    //             minimum: config.min_order_size,
    //         });
    //     }

    //     if *order_type != OrderType::Market && !price.is_valid() {
    //         return Err(OrderbookError::InvalidPrice { price: price.0 });
    //     }

    //     Ok(())
    // }

    // fn verify_market_active(
    //     &self,
    //     market_id: MarketId,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let market = self
    //         .market
    //         .markets
    //         .get(&market_id, state)
    //         .into_orderbook_err()?
    //         .ok_or(OrderbookError::MarketNotFound { market_id })?;

    //     if market.status != MarketStatus::Active {
    //         return Err(OrderbookError::MarketNotActive {
    //             market_id,
    //             status: format!("{:?}", market.status),
    //         });
    //     }

    //     Ok(())
    // }

    // fn would_match(
    //     &self,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     side: Side,
    //     price: Price,
    //     state: &mut impl TxState<S>,
    // ) -> Result<bool, OrderbookError> {
    //     let book_key = BookKey { market_id, outcome };

    //     match side {
    //         Side::Bid => {
    //             if let Some(best_ask) = self.best_ask.get(&book_key, state).into_orderbook_err()? {
    //                 return Ok(price >= best_ask);
    //             }
    //         }
    //         Side::Ask => {
    //             if let Some(best_bid) = self.best_bid.get(&book_key, state).into_orderbook_err()? {
    //                 return Ok(price <= best_bid);
    //             }
    //         }
    //     }

    //     Ok(false)
    // }

    // fn get_matchable_levels(
    //     &self,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     book_side: Side,
    //     limit_price: Price,
    //     incoming_side: Side,
    //     state: &mut impl TxState<S>,
    // ) -> Result<Vec<Price>, OrderbookError> {
    //     let key = BookSideKey {
    //         market_id,
    //         outcome,
    //         side: book_side,
    //     };
    //     let mut levels = self
    //         .price_levels
    //         .get(&key, state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();

    //     levels.retain(|&p| match incoming_side {
    //         Side::Bid => p <= limit_price,
    //         Side::Ask => p >= limit_price,
    //     });

    //     // Sort for matching order
    //     match book_side {
    //         Side::Ask => levels.sort(),
    //         Side::Bid => levels.sort_by(|a, b| b.cmp(a)),
    //     }

    //     Ok(levels)
    // }

    // fn add_to_book(
    //     &mut self,
    //     order: &Order<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let level_key = PriceLevelKey {
    //         market_id: order.market_id,
    //         outcome: order.outcome,
    //         price: order.price,
    //     };

    //     let mut order_ids = match order.side {
    //         Side::Bid => self.bids.get(&level_key, state).into_orderbook_err()?,
    //         Side::Ask => self.asks.get(&level_key, state).into_orderbook_err()?,
    //     }
    //     .unwrap_or_default();

    //     order_ids.push(order.id);

    //     match order.side {
    //         Side::Bid => self
    //             .bids
    //             .set(&level_key, &order_ids, state)
    //             .into_orderbook_err()?,
    //         Side::Ask => self
    //             .asks
    //             .set(&level_key, &order_ids, state)
    //             .into_orderbook_err()?,
    //     };

    //     self.add_price_level(
    //         order.market_id,
    //         order.outcome,
    //         order.side,
    //         order.price,
    //         state,
    //     )?;
    //     self.update_best_prices(order.market_id, order.outcome, state)?;

    //     Ok(())
    // }

    // fn remove_from_book(
    //     &mut self,
    //     order: &Order<S>,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let level_key = PriceLevelKey {
    //         market_id: order.market_id,
    //         outcome: order.outcome,
    //         price: order.price,
    //     };

    //     let mut order_ids = match order.side {
    //         Side::Bid => self.bids.get(&level_key, state).into_orderbook_err()?,
    //         Side::Ask => self.asks.get(&level_key, state).into_orderbook_err()?,
    //     }
    //     .unwrap_or_default();

    //     order_ids.retain(|&id| id != order.id);

    //     if order_ids.is_empty() {
    //         match order.side {
    //             Side::Bid => self.bids.remove(&level_key, state).into_orderbook_err()?,
    //             Side::Ask => self.asks.remove(&level_key, state).into_orderbook_err()?,
    //         };
    //         self.remove_price_level(
    //             order.market_id,
    //             order.outcome,
    //             order.side,
    //             order.price,
    //             state,
    //         )?;
    //     } else {
    //         match order.side {
    //             Side::Bid => self
    //                 .bids
    //                 .set(&level_key, &order_ids, state)
    //                 .into_orderbook_err()?,
    //             Side::Ask => self
    //                 .asks
    //                 .set(&level_key, &order_ids, state)
    //                 .into_orderbook_err()?,
    //         };
    //     }

    //     self.update_best_prices(order.market_id, order.outcome, state)?;

    //     Ok(())
    // }

    // fn add_price_level(
    //     &mut self,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     side: Side,
    //     price: Price,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let key = BookSideKey {
    //         market_id,
    //         outcome,
    //         side,
    //     };
    //     let mut levels = self
    //         .price_levels
    //         .get(&key, state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();

    //     if !levels.contains(&price) {
    //         levels.push(price);
    //         match side {
    //             Side::Bid => levels.sort_by(|a, b| b.cmp(a)),
    //             Side::Ask => levels.sort(),
    //         }
    //         self.price_levels
    //             .set(&key, &levels, state)
    //             .into_orderbook_err()?;
    //     }

    //     Ok(())
    // }

    // pub(crate) fn remove_price_level(
    //     &mut self,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     side: Side,
    //     price: Price,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let key = BookSideKey {
    //         market_id,
    //         outcome,
    //         side,
    //     };
    //     let mut levels = self
    //         .price_levels
    //         .get(&key, state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();
    //     levels.retain(|&p| p != price);

    //     if levels.is_empty() {
    //         self.price_levels.remove(&key, state).into_orderbook_err()?;
    //     } else {
    //         self.price_levels
    //             .set(&key, &levels, state)
    //             .into_orderbook_err()?;
    //     }

    //     Ok(())
    // }

    // pub(crate) fn update_best_prices(
    //     &mut self,
    //     market_id: MarketId,
    //     outcome: OutcomeSide,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let book_key = BookKey { market_id, outcome };

    //     // Best bid
    //     let bid_key = BookSideKey {
    //         market_id,
    //         outcome,
    //         side: Side::Bid,
    //     };
    //     let bid_levels = self
    //         .price_levels
    //         .get(&bid_key, state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();
    //     if let Some(&best) = bid_levels.first() {
    //         self.best_bid
    //             .set(&book_key, &best, state)
    //             .into_orderbook_err()?;
    //     } else {
    //         self.best_bid
    //             .remove(&book_key, state)
    //             .into_orderbook_err()?;
    //     }

    //     // Best ask
    //     let ask_key = BookSideKey {
    //         market_id,
    //         outcome,
    //         side: Side::Ask,
    //     };
    //     let ask_levels = self
    //         .price_levels
    //         .get(&ask_key, state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();
    //     if let Some(&best) = ask_levels.first() {
    //         self.best_ask
    //             .set(&book_key, &best, state)
    //             .into_orderbook_err()?;
    //     } else {
    //         self.best_ask
    //             .remove(&book_key, state)
    //             .into_orderbook_err()?;
    //     }

    //     self.emit_event(
    //         state,
    //         Event::BookUpdated {
    //             market_id,
    //             outcome,
    //             best_bid: bid_levels.first().copied(),
    //             best_ask: ask_levels.first().copied(),
    //         },
    //     );

    //     Ok(())
    // }

    // fn lock_collateral(
    //     &mut self,
    //     user: &S::Address,
    //     market_id: MarketId,
    //     amount: u64,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let key = UserMarketKey {
    //         address: user.clone(),
    //         market_id,
    //     };
    //     let current = self
    //         .locked_collateral
    //         .get(&key, state)
    //         .into_orderbook_err()?
    //         .unwrap_or(0);
    //     self.locked_collateral
    //         .set(&key, &(current + amount), state)
    //         .into_orderbook_err()?;
    //     Ok(())
    // }

    // fn unlock_collateral(
    //     &mut self,
    //     user: &S::Address,
    //     market_id: MarketId,
    //     amount: u64,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let key = UserMarketKey {
    //         address: user.clone(),
    //         market_id,
    //     };
    //     let current = self
    //         .locked_collateral
    //         .get(&key, state)
    //         .into_orderbook_err()?
    //         .unwrap_or(0);
    //     let new_val = current.saturating_sub(amount);

    //     if new_val == 0 {
    //         self.locked_collateral
    //             .remove(&key, state)
    //             .into_orderbook_err()?;
    //     } else {
    //         self.locked_collateral
    //             .set(&key, &new_val, state)
    //             .into_orderbook_err()?;
    //     }

    //     Ok(())
    // }

    // pub(crate) fn add_user_order(
    //     &mut self,
    //     user: &S::Address,
    //     order_id: OrderId,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let mut orders = self
    //         .user_orders
    //         .get(user, state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();
    //     orders.push(order_id);
    //     self.user_orders
    //         .set(user, &orders, state)
    //         .into_orderbook_err()?;
    //     Ok(())
    // }

    // pub(crate) fn remove_user_order(
    //     &mut self,
    //     user: &S::Address,
    //     order_id: OrderId,
    //     state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     let mut orders = self
    //         .user_orders
    //         .get(user, state)
    //         .into_orderbook_err()?
    //         .unwrap_or_default();
    //     orders.retain(|&id| id != order_id);

    //     if orders.is_empty() {
    //         self.user_orders.remove(user, state).into_orderbook_err()?;
    //     } else {
    //         self.user_orders
    //             .set(user, &orders, state)
    //             .into_orderbook_err()?;
    //     }

    //     Ok(())
    // }

    // fn execute_fill(
    //     &mut self,
    //     _market_id: MarketId,
    //     _outcome: OutcomeSide,
    //     _taker_side: Side,
    //     _fill: &Fill,
    //     _taker: &S::Address,
    //     _state: &mut impl TxState<S>,
    // ) -> Result<(), OrderbookError> {
    //     // TODO: Implement actual share and collateral transfers
    //     // This requires integration with prediction_market positions
    //     // and bank module for collateral
    //     Ok(())
    // }
}
