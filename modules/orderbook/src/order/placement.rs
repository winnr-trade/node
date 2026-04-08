use crate::error::IntoOrderbookError;
use crate::order::canonical::CanonicalOrder;
use crate::{Event, MatchResult, Order, OrderRequest, OrderbookError, OrderbookModule};
use market::MarketId;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};
use sov_bank::TokenId;
use sov_modules_api::{Context, EventEmitter, HexHash, SafeVec, Spec, TxState};
use tracing::debug;

impl<S: Spec> OrderbookModule<S> {
    pub(crate) fn place_order_normal(
        &mut self,
        order_request: OrderRequest,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        self.place_order(order_request, ctx.sender(), state)
    }

    pub(crate) fn place_order_stealth(
        &mut self,
        order_request: OrderRequest,
        proof: SafeVec<u8>,
        commitment: HexHash,
        nullifier: HexHash,
        stealth_address: &S::Address,
        token_id: TokenId,
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
                order_request.quantity,
                stealth_address,
                ctx,
                state,
            )
            .ok()
            .unwrap();

        self.place_order(order_request, &*stealth_address, state)
    }

    /// Place a new order: normalize to canonical YES-space, match, settle, post.
    fn place_order(
        &mut self,
        order_request: OrderRequest,
        sender: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let OrderRequest {
            market_id,
            outcome,
            side,
            price,
            quantity,
            order_type,
        } = order_request;

        // Normalize to canonical YES-space
        let canonical_order = CanonicalOrder::normalize(outcome, side, price);

        // Validate (using canonical price for range check)
        self.validate_order(&order_request, &canonical_order, state)?;

        // Check PostOnly before matching
        if order_type == OrderType::PostOnly
            && self
                .would_match(
                    market_id,
                    canonical_order.side,
                    canonical_order.price,
                    state,
                )
                .into_orderbook_err()?
        {
            return Err(OrderbookError::PostOnlyWouldMatch);
        }

        // Compute matches in canonical space before any stateful execution so FOK can be enforced
        let match_result = self.compute_matches(
            market_id,
            canonical_order.side,
            canonical_order.price,
            quantity,
            sender,
            state,
        )?;

        let should_post = Self::should_post(
            &order_type,
            match_result.total_filled,
            match_result.remaining,
            quantity,
        )?;

        // Generate order ID only once we know the order will proceed.
        let order_id = self.next_order_id(state)?;

        // Lock resources based on original intent
        match side {
            Side::Bid => {
                // BUY order: lock collateral at limit price
                let required_collateral = canonical_order.required_collateral(quantity);
                self.lock_collateral(sender, market_id, required_collateral, state)?;
            }
            Side::Ask => {
                // SELL order: reserve shares of the outcome being sold
                self.lock_shares(sender, market_id, outcome, quantity, state)?;
            }
        }

        // Execution matches and update book
        self.execute_matches(
            market_id,
            canonical_order.side,
            sender,
            order_id,
            side,
            &match_result,
            state,
        )?;

        // Post remaining to book and refund excess locked resources
        match side {
            Side::Bid => {
                // BUY: refund price improvement collateral
                let total_required_collateral = canonical_order.required_collateral(quantity);
                let total_used_collateral: u64 = match_result
                    .fills
                    .iter()
                    .map(|f| match canonical_order.side {
                        Side::Bid => f.price.cost(f.quantity),
                        Side::Ask => f.price.complement().cost(f.quantity),
                    })
                    .sum();

                if should_post && match_result.remaining > 0 {
                    self.post_order(
                        order_id,
                        &order_request,
                        &canonical_order,
                        &match_result,
                        sender,
                        state,
                    )?;

                    // Refund excess collateral (case when when better prices were matched) after partial fills
                    let remaining_collateral =
                        total_required_collateral.saturating_sub(total_used_collateral);
                    let required_remaining_collateral =
                        canonical_order.required_collateral(match_result.remaining);

                    if remaining_collateral > required_remaining_collateral {
                        self.unlock_collateral(
                            sender,
                            market_id,
                            remaining_collateral - required_remaining_collateral,
                            state,
                        )?;
                    }
                } else if match_result.total_filled > 0 {
                    // Fully filled or IOC/Market with partial fills
                    let remaining_collateral =
                        total_required_collateral.saturating_sub(total_used_collateral);
                    if remaining_collateral > 0 {
                        self.unlock_collateral(sender, market_id, remaining_collateral, state)?;
                    }
                } else if match_result.remaining > 0 {
                    // Not posted (IOC/Market) with no fills
                    self.unlock_collateral(sender, market_id, total_required_collateral, state)?;
                }
            }
            Side::Ask => {
                // SELL: unlock unfilled shares if not posting
                if should_post && match_result.remaining > 0 {
                    self.post_order(
                        order_id,
                        &order_request,
                        &canonical_order,
                        &match_result,
                        sender,
                        state,
                    )?;
                    // Shares for remaining qty stay reserved (for the resting order)
                } else if match_result.remaining > 0 {
                    // IOC/Market/no fills: unlock unfilled shares
                    let to_unreserve = match_result.remaining;
                    self.unlock_shares(sender, market_id, outcome, to_unreserve, state)?;
                }
            }
        }

        // Emit order placed event
        self.emit_event(
            state,
            Event::OrderPlaced {
                order_id,
                market_id,
                outcome,
                side,
                canonical_side: canonical_order.side,
                canonical_price: canonical_order.price,
                quantity,
                order_type,
                owner: sender.to_string(),
            },
        );

        // Emit fill event if any
        if match_result.total_filled > 0 {
            let avg_price = match_result
                .fills
                .iter()
                .map(|f| f.price.0 * f.quantity)
                .sum::<u64>()
                / match_result.total_filled;

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

    /// Post remaining quantity to the book as a resting order.
    fn post_order(
        &mut self,
        order_id: OrderId,
        order_request: &OrderRequest,
        canonical: &CanonicalOrder,
        match_result: &MatchResult,
        sender: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let OrderRequest {
            market_id,
            outcome,
            side,
            quantity,
            order_type,
            ..
        } = order_request;

        // Get current time for order creation
        let created_at = self
            .chain_state
            .get_time(state)
            .into_orderbook_err()?
            .as_millis() as u64;

        let order = Order {
            id: order_id,
            market_id: *market_id,
            outcome: *outcome,
            side: *side,
            canonical_side: canonical.side,
            canonical_price: canonical.price,
            original_quantity: *quantity,
            remaining_quantity: match_result.remaining,
            owner: sender.clone(),
            order_type: *order_type,
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
        self.add_order(&order, state)?;

        debug!(order_id = %order_id, "Order posted to canonical book");
        Ok(())
    }
}
