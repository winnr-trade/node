use crate::call::MAX_MEMO_BYTES;
use crate::error::IntoOrderbookError;
use crate::event::CancelReason;
use crate::order::canonical::CanonicalOrder;
use crate::{Event, MatchResult, Order, OrderRequest, OrderbookError, OrderbookModule};
use agent_wallet::{SCOPE_CANCEL_ALL_ORDERS, SCOPE_CANCEL_ORDER, SCOPE_PLACE_ORDER};
use market::MarketId;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Side, Size};
use shielded_pool::ProofBytes;
use sov_bank::Amount;
use sov_modules_api::{Context, EventEmitter, HexHash, SafeVec, Spec, TxState};
use tracing::debug;

impl<S: Spec> OrderbookModule<S> {
    pub(crate) fn place_order_normal(
        &mut self,
        order_request: OrderRequest,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let owner = self
            .agent_wallet
            .resolve_principal_or_self(ctx.sender(), SCOPE_PLACE_ORDER, state)
            .into_orderbook_err()?;
        self.place_order(order_request, &owner, state)
    }

    pub(crate) fn place_order_stealth(
        &mut self,
        order_request: OrderRequest,
        proof: ProofBytes,
        root: HexHash,
        commitment: HexHash,
        nullifier: HexHash,
        note_memo: SafeVec<u8, MAX_MEMO_BYTES>,
        stealth_memo: SafeVec<u8, MAX_MEMO_BYTES>,
        stealth_address: &S::Address,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        if order_request.side != Side::Bid {
            return Err(OrderbookError::StealthOrderMustBeBid);
        }

        let market = self
            .market
            .markets
            .get(&order_request.market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound {
                market_id: order_request.market_id,
            })?;
        let canonical = CanonicalOrder::normalize(
            order_request.outcome,
            order_request.side,
            order_request.price,
        );
        let required_collateral =
            canonical.required_collateral(order_request.quantity, &market.collateral_token);

        self.shielded_pool
            .withdraw_to(
                proof,
                root,
                commitment,
                nullifier,
                required_collateral,
                note_memo,
                stealth_address,
                ctx,
                state,
            )
            .into_orderbook_err()?;

        self.emit_event(
            state,
            Event::StealthOrderMemo {
                commitment,
                stealth_address: stealth_address.to_string(),
                memo: stealth_memo.as_ref().to_vec(),
            },
        );

        self.place_order(order_request, stealth_address, state)
    }

    /// Cancel an order.
    pub(crate) fn cancel_order(
        &mut self,
        order_id: OrderId,
        context: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let sender = self
            .agent_wallet
            .resolve_principal_or_self(context.sender(), SCOPE_CANCEL_ORDER, state)
            .into_orderbook_err()?;

        let mut order = self
            .orders
            .get(&order_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::OrderNotFound { order_id })?;

        // Verify ownership
        if order.owner != sender {
            return Err(OrderbookError::NotOrderOwner {
                order_id,
                owner: order.owner.to_string(),
                sender: sender.to_string(),
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
        self.remove_order(&order, state)?;

        // Release locked resources based on original intent
        match order.side {
            Side::Bid => {
                // BUY order: unlock collateral
                let market = self
                    .market
                    .markets
                    .get(&order.market_id, state)
                    .into_orderbook_err()?
                    .ok_or(OrderbookError::MarketNotFound {
                        market_id: order.market_id,
                    })?;
                let locked = order.locked_collateral(&market.collateral_token);
                self.unlock_collateral_to(
                    &order.owner,
                    Some(&order.owner),
                    order.market_id,
                    locked,
                    state,
                )?;
            }
            Side::Ask => {
                // SELL order: unreserve shares
                self.unlock_shares_to(
                    &order.owner,
                    None,
                    order.market_id,
                    order.outcome,
                    unfilled,
                    Amount::ZERO,
                    Amount::ZERO,
                    state,
                )?;
            }
        }

        // Update order
        order.status = OrderStatus::Cancelled;
        order.remaining_quantity = Size::ZERO;
        self.orders
            .set(&order_id, &order, state)
            .into_orderbook_err()?;

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
        let owner = self
            .agent_wallet
            .resolve_principal_or_self(context.sender(), SCOPE_CANCEL_ALL_ORDERS, state)
            .into_orderbook_err()?;

        let order_ids = self
            .user_orders
            .get(&owner, state)
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

        // Load market to get collateral decimals
        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        let collateral_token = market.collateral_token;

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
            order_type,
            &collateral_token,
            state,
        )?;

        let should_post = Self::should_post(
            &order_type,
            match_result.total_quantity_filled,
            match_result.remaining_quantity,
            quantity,
        )?;

        // Generate order ID only once we know the order will proceed.
        let order_id = self.next_order_id(state)?;

        // Lock resources based on original intent
        match side {
            Side::Bid => {
                // BUY order: lock collateral at limit price
                let required_collateral =
                    canonical_order.required_collateral(quantity, &collateral_token);
                self.lock_collateral_from(sender, market_id, required_collateral, state)?;
            }
            Side::Ask => {
                // SELL order: reserve shares of the outcome being sold
                self.lock_shares_from(sender, market_id, outcome, quantity, state)?;
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
                let total_required_collateral =
                    canonical_order.required_collateral(quantity, &collateral_token);
                let total_used_collateral =
                    match_result.fills.iter().fold(Amount::ZERO, |acc, f| {
                        let cost = match canonical_order.side {
                            Side::Bid => f.price.cost(f.quantity, &collateral_token),
                            Side::Ask => f.price.complement().cost(f.quantity, &collateral_token),
                        };
                        acc.saturating_add(cost)
                    });

                if should_post && !match_result.remaining_quantity.is_zero() {
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
                    let required_remaining_collateral = canonical_order
                        .required_collateral(match_result.remaining_quantity, &collateral_token);

                    if remaining_collateral > required_remaining_collateral {
                        self.unlock_collateral_to(
                            sender,
                            Some(sender),
                            market_id,
                            remaining_collateral.saturating_sub(required_remaining_collateral),
                            state,
                        )?;
                    }
                } else if !match_result.total_quantity_filled.is_zero() {
                    // Fully filled or IOC/Market with partial fills
                    let remaining_collateral =
                        total_required_collateral.saturating_sub(total_used_collateral);
                    if remaining_collateral > 0u128 {
                        self.unlock_collateral_to(
                            sender,
                            Some(sender),
                            market_id,
                            remaining_collateral,
                            state,
                        )?;
                    }
                } else if !match_result.remaining_quantity.is_zero() {
                    // Not posted (IOC/Market) with no fills
                    self.unlock_collateral_to(
                        sender,
                        Some(sender),
                        market_id,
                        total_required_collateral,
                        state,
                    )?;
                }
            }
            Side::Ask => {
                // SELL: unlock unfilled shares if not posting
                if should_post && !match_result.remaining_quantity.is_zero() {
                    self.post_order(
                        order_id,
                        &order_request,
                        &canonical_order,
                        &match_result,
                        sender,
                        state,
                    )?;
                    // Shares for remaining qty stay reserved (for the resting order)
                } else if !match_result.remaining_quantity.is_zero() {
                    // IOC/Market/no fills: unlock unfilled shares
                    let to_unreserve = match_result.remaining_quantity;
                    self.unlock_shares_to(
                        sender,
                        None,
                        market_id,
                        outcome,
                        to_unreserve,
                        Amount::ZERO,
                        Amount::ZERO,
                        state,
                    )?;
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
        if !match_result.total_quantity_filled.is_zero() {
            let avg_price = match_result
                .fills
                .iter()
                .map(|f| f.price.0 * f.quantity.0)
                .sum::<u64>()
                / match_result.total_quantity_filled.0;

            self.emit_event(
                state,
                Event::OrderFilled {
                    order_id,
                    filled_quantity: match_result.total_quantity_filled,
                    remaining_quantity: match_result.remaining_quantity,
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
            remaining_quantity: match_result.remaining_quantity,
            owner: sender.clone(),
            order_type: *order_type,
            created_at,
            status: if !match_result.total_quantity_filled.is_zero() {
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
