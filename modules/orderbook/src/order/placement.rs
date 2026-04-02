use crate::error::IntoOrderbookError;
use crate::{Event, Order, OrderbookError, OrderbookModule};
use market::MarketId;
use shared_types::{OrderStatus, OrderType, OutcomeSide, Price, Side};
use sov_bank::TokenId;
use sov_modules_api::{Context, EventEmitter, HexHash, Spec, TxState};
use tracing::debug;

impl<S: Spec> OrderbookModule<S> {
    pub(crate) fn place_order_normal(
        &mut self,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
        quantity: u64,
        order_type: OrderType,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        self.place_order(
            ctx.sender(),
            market_id,
            outcome,
            side,
            price,
            quantity,
            order_type,
            ctx,
            state,
        )
    }

    pub(crate) fn place_order_stealth(
        &mut self,
        proof: Vec<u8>,
        commitment: HexHash,
        nullifier: HexHash,
        stealth_address: &S::Address,
        market_id: MarketId,
        token_id: TokenId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
        quantity: u64,
        order_type: OrderType,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        // Withdraw from shielded pool to a stealth address
        self.shielded_pool
            .withdraw_to(
                proof,
                commitment,
                nullifier,
                token_id,
                quantity,
                stealth_address,
                ctx,
                state,
            )
            .ok()
            .unwrap();

        self.place_order(
            &*stealth_address,
            market_id,
            outcome,
            side,
            price,
            quantity,
            order_type,
            ctx,
            state,
        )
    }

    /// Place a new order on behalf of the sender.
    fn place_order(
        &mut self,
        sender: &S::Address,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
        quantity: u64,
        order_type: OrderType,
        _ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        // Validate
        self.validate_order(&price, quantity, &order_type, state)?;
        self.verify_market_active(market_id, state)?;

        // Generate order ID
        let order_id = self.next_order_id(state)?;

        // Check PostOnly before matching
        if order_type == OrderType::PostOnly
            && self
                .would_match(market_id, outcome, side, price, state)
                .into_orderbook_err()?
        {
            return Err(OrderbookError::PostOnlyWouldMatch);
        }

        // Lock collateral for bids
        if side == Side::Bid {
            let required_amount = price.cost(quantity);
            self.lock_collateral(sender, market_id, required_amount, state)?;
        }

        // Run matching
        let match_result = self.match_order(
            market_id,
            outcome,
            side,
            price,
            quantity,
            &order_type,
            sender,
            order_id,
            state,
        )?;

        // Execute fills
        for fill in &match_result.fills {
            self.execute_fill(market_id, outcome, side, fill, sender, state)?;
        }

        // Get current time for order creation
        let created_at = self
            .chain_state
            .get_time(state)
            .into_orderbook_err()?
            .as_millis() as u64;

        // Post remaining to book if applicable
        if match_result.should_post && match_result.remaining > 0 {
            let order = Order {
                id: order_id,
                market_id,
                outcome,
                side,
                price,
                original_quantity: quantity,
                remaining_quantity: match_result.remaining,
                owner: sender.clone(),
                order_type,
                created_at,
                status: if match_result.total_filled > 0 {
                    OrderStatus::PartiallyFilled
                } else {
                    OrderStatus::Open
                },
            };

            self.orders
                .set(&order_id, &order, state)
                .into_orderbook_err()?;
            self.add_to_book(&order, state)?;
            self.add_user_order(sender, order_id, state)?;

            debug!(order_id = %order_id, "Order posted to book");
        }

        // Emit order placed event
        self.emit_event(
            state,
            Event::OrderPlaced {
                order_id,
                market_id,
                outcome,
                side,
                price,
                quantity,
                order_type,
                owner: sender.to_string(),
            },
        );

        // Emit fill event if any
        if match_result.total_filled > 0 {
            let avg_price = if match_result.total_filled > 0 {
                match_result
                    .fills
                    .iter()
                    .map(|f| f.price.0 * f.quantity)
                    .sum::<u64>()
                    / match_result.total_filled
            } else {
                0
            };

            self.emit_event(
                state,
                Event::OrderFilled {
                    order_id,
                    filled_quantity: match_result.total_filled,
                    remaining_quantity: match_result.remaining,
                    average_price: avg_price,
                },
            );
        }

        Ok(())
    }
}
