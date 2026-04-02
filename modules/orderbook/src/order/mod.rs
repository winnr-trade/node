use crate::{error::IntoOrderbookError, event::CancelReason, UserMarketKey};
use market::MarketId;
use shared_types::{OrderId, OrderStatus, OutcomeSide, Side};
use sov_modules_api::{Context, EventEmitter, Spec, TxState};

pub mod book;
pub mod helpers;
pub mod matching;
pub mod placement;

use crate::{Event, OrderbookError, OrderbookModule};

impl<S: Spec> OrderbookModule<S> {
    /// Cancel an order.
    pub(crate) fn cancel_order(
        &mut self,
        order_id: OrderId,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let mut order = self
            .orders
            .get(&order_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::OrderNotFound { order_id })?;

        // Verify ownership
        if order.owner != *context.sender() {
            return Err(OrderbookError::NotOrderOwner {
                order_id,
                owner: order.owner.to_string(),
                sender: context.sender().to_string(),
            });
        }

        // Must be open
        if order.status != OrderStatus::Open && order.status != OrderStatus::PartiallyFilled {
            return Err(OrderbookError::OrderNotCancellable {
                order_id,
                status: format!("{:?}", order.status),
            });
        }

        let unfilled = order.remaining_quantity;

        // Remove from book
        self.remove_from_book(&order, state)?;

        // Unlock collateral
        if order.side == Side::Bid {
            let locked = order.price.cost(order.remaining_quantity);
            self.unlock_collateral(&order.owner, order.market_id, locked, state)?;
        }

        // Update order
        order.status = OrderStatus::Cancelled;
        order.remaining_quantity = 0;
        self.orders
            .set(&order_id, &order, state)
            .into_orderbook_err()?;

        // Remove from user orders
        self.remove_user_order(&order.owner, order_id, state)?;

        self.emit_event(
            state,
            Event::OrderCancelled {
                order_id,
                reason: CancelReason::UserRequested,
                unfilled_quantity: unfilled,
            },
        );

        Ok(())
    }

    /// Cancel all orders for a market.
    pub(crate) fn cancel_all_orders(
        &mut self,
        market_id: MarketId,
        outcome: Option<OutcomeSide>,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let order_ids = self
            .user_orders
            .get(context.sender(), state)
            .into_orderbook_err()?
            .unwrap_or_default();

        for order_id in order_ids {
            if let Some(order) = self.orders.get(&order_id, state).into_orderbook_err()? {
                if order.market_id == market_id {
                    if outcome.is_none() || outcome == Some(order.outcome) {
                        // Ignore errors for individual cancels
                        let _ = self.cancel_order(order_id, context, state);
                    }
                }
            }
        }

        Ok(())
    }

    pub(crate) fn lock_collateral(
        &mut self,
        user: &S::Address,
        market_id: MarketId,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = UserMarketKey {
            address: user.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or(0);
        self.locked_collateral
            .set(&key, &(current + amount), state)
            .into_orderbook_err()?;
        Ok(())
    }

    pub(crate) fn unlock_collateral(
        &mut self,
        user: &S::Address,
        market_id: MarketId,
        amount: u64,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = UserMarketKey {
            address: user.clone(),
            market_id,
        };
        let current = self
            .locked_collateral
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or(0);
        let new_val = current.saturating_sub(amount);

        if new_val == 0 {
            self.locked_collateral
                .remove(&key, state)
                .into_orderbook_err()?;
        } else {
            self.locked_collateral
                .set(&key, &new_val, state)
                .into_orderbook_err()?;
        }

        Ok(())
    }
}
