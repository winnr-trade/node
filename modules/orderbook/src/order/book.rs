use crate::error::IntoOrderbookError;
use crate::{
    Event, Fill, MarketSideKey, Order, OrderbookError, OrderbookModule, PriceLevelKey,
    SettlementKind,
};
use market::MarketId;
use shared_types::{OrderId, OutcomeSide, Price, Side, Size, TokenIdExt};
use sov_bank::utils::TokenHolder;
use sov_bank::{Amount, Coins, IntoPayable};
use sov_modules_api::{EventEmitter, Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    /// Add an order to the canonical book.
    pub(crate) fn add_order(
        &mut self,
        order: &Order<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let price_level_key = PriceLevelKey {
            market_id: order.market_id,
            price: order.canonical_price,
        };

        let book = match order.canonical_side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        let mut order_ids = book
            .get(&price_level_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        order_ids.push(order.id);

        book.set(&price_level_key, &order_ids, state)
            .into_orderbook_err()?;

        self.add_price_level(
            order.market_id,
            order.canonical_side,
            order.canonical_price,
            state,
        )?;
        self.add_user_order(&order.owner, order.id, state)?;
        self.update_best_prices(order.market_id, state)?;

        Ok(())
    }

    pub(crate) fn remove_order(
        &mut self,
        order: &Order<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let level_key = PriceLevelKey {
            market_id: order.market_id,
            price: order.canonical_price,
        };

        // Remove order from bids/asks book
        let book = match order.canonical_side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };
        let mut order_ids = book
            .get(&level_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        order_ids.retain(|&id| id != order.id);

        if order_ids.is_empty() {
            book.remove(&level_key, state).into_orderbook_err()?;

            self.remove_price_level(
                order.market_id,
                order.canonical_side,
                order.canonical_price,
                state,
            )?;
        } else {
            book.set(&level_key, &order_ids, state)
                .into_orderbook_err()?;
        }

        // Remove from user's order list
        self.remove_user_order(&order.owner, order.id, state)?;

        self.update_best_prices(order.market_id, state)?;

        Ok(())
    }

    /// Execute a fill: settle based on counterparty intents.
    ///
    /// The settlement path is determined by `settlement_kind`:
    /// - **MintPair**: both sides are BUYing → mint YES+NO pair, distribute.
    /// - **TransferYes**: BUY YES vs SELL YES → transfer existing YES shares.
    /// - **TransferNo**: BUY NO vs SELL NO → transfer existing NO shares.
    /// - **MergePair**: SELL YES vs SELL NO → burn YES+NO pair.
    pub(crate) fn execute_fill(
        &mut self,
        market_id: MarketId,
        fill: &Fill,
        settlement_kind: SettlementKind,
        canonical_buyer: &S::Address,
        canonical_seller: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        match settlement_kind {
            SettlementKind::MintPair => {
                self.settle_mint_pair(market_id, fill, canonical_buyer, canonical_seller, state)
            }
            SettlementKind::TransferYes => {
                self.settle_transfer_yes(market_id, fill, canonical_buyer, canonical_seller, state)
            }
            SettlementKind::TransferNo => {
                self.settle_transfer_no(market_id, fill, canonical_buyer, canonical_seller, state)
            }
            SettlementKind::MergePair => {
                self.settle_merge_pair(market_id, fill, canonical_buyer, canonical_seller, state)
            }
        }
    }

    /// MintPair: BUY YES vs BUY NO — both contribute collateral, mint new pair.
    ///
    /// - Canonical buyer (BUY YES): collateral consumed → receives YES shares.
    /// - Canonical seller (BUY NO): collateral consumed → receives NO shares.
    /// - Market: total_yes += qty, total_no += qty, market_collateral += qty * 10^decimals.
    fn settle_mint_pair(
        &mut self,
        market_id: MarketId,
        fill: &Fill,
        canonical_buyer: &S::Address,
        canonical_seller: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        let qty = fill.quantity;
        let token = &market.collateral_token;

        // Calculate total expected cost for minting the pair
        let scale = 10u128.pow(token.get_decimals() as u32);
        let expected_total_cost = Amount::from(qty.0)
            .checked_mul(Amount(scale))
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in qty scaling")))?;

        // Split total cost according to execution price: buyer pays according to fill price
        // and seller pays complement.
        let buyer_cost = fill.price.cost(qty, token);
        let seller_cost = expected_total_cost.checked_sub(buyer_cost).ok_or_else(|| {
            OrderbookError::Any(anyhow::anyhow!("Underflow in MintPair collateral split"))
        })?;
        let total_cost = buyer_cost.checked_add(seller_cost).ok_or_else(|| {
            OrderbookError::Any(anyhow::anyhow!("Overflow in MintPair collateral split"))
        })?;

        // Sanity check that the split costs sum to the expected total cost for the pair.
        if total_cost != expected_total_cost {
            return Err(OrderbookError::Any(anyhow::anyhow!(
                "MintPair collateral split mismatch: buyer={} seller={} total={} expected={}",
                buyer_cost,
                seller_cost,
                total_cost,
                expected_total_cost
            )));
        }

        self.unlock_collateral_to(
            canonical_buyer,
            None::<TokenHolder<S>>,
            market_id,
            buyer_cost,
            state,
        )?;

        self.unlock_collateral_to(
            canonical_seller,
            None::<TokenHolder<S>>,
            market_id,
            seller_cost,
            state,
        )?;

        // Mint pair from orderbook custody into orderbook holder.
        self.market
            .mint_shares_to(market_id, qty, self.id.to_payable(), state)
            .into_orderbook_err()?;

        // Distribute minted shares to counterparties.
        self.market
            .transfer_shares_from(
                market_id,
                self.id.to_payable(),
                canonical_buyer,
                qty,
                Size::ZERO,
                state,
            )
            .into_orderbook_err()?;
        self.market
            .transfer_shares_from(
                market_id,
                self.id.to_payable(),
                canonical_seller,
                Size::ZERO,
                qty,
                state,
            )
            .into_orderbook_err()?;

        Ok(())
    }

    /// TransferYes: BUY YES vs SELL YES — transfer existing YES shares.
    ///
    /// - Canonical buyer (BUY YES): collateral consumed → receives YES shares.
    /// - Canonical seller (SELL YES): YES shares unreserved and transferred.
    /// - Market totals unchanged (no minting).
    fn settle_transfer_yes(
        &mut self,
        market_id: MarketId,
        fill: &Fill,
        canonical_buyer: &S::Address,
        canonical_seller: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        let qty = fill.quantity;
        let token = &market.collateral_token;

        // Unlock and transfer YES shares: seller -> buyer.
        self.unlock_shares_to(
            canonical_seller,
            Some(canonical_buyer),
            market_id,
            OutcomeSide::Yes,
            qty,
            state,
        )?;

        // Unlock and transfer collateral: buyer -> seller
        let buyer_cost = fill.price.cost(qty, token);
        self.unlock_collateral_to(
            canonical_buyer,
            Some(canonical_seller),
            market_id,
            buyer_cost,
            state,
        )?;

        Ok(())
    }

    /// TransferNo: BUY NO vs SELL NO — transfer existing NO shares.
    ///
    /// - Canonical buyer (SELL NO → canonical bid): NO shares unreserved and transferred.
    /// - Canonical seller (BUY NO → canonical ask): collateral consumed → receives NO shares.
    /// - Market totals unchanged (no minting).
    fn settle_transfer_no(
        &mut self,
        market_id: MarketId,
        fill: &Fill,
        canonical_buyer: &S::Address,
        canonical_seller: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        let qty = fill.quantity;
        let token = &market.collateral_token;

        // Unlock and transfer NO shares: canonical buyer -> canonical seller.
        self.unlock_shares_to(
            canonical_buyer,
            Some(canonical_seller),
            market_id,
            OutcomeSide::No,
            qty,
            state,
        )?;

        // Release canonical seller's collateral (they are BUY NO, consumed as payment)
        let seller_cost = fill.price.complement().cost(qty, token);
        self.unlock_collateral_to(
            canonical_seller,
            Some(canonical_buyer),
            market_id,
            seller_cost,
            state,
        )?;

        Ok(())
    }

    /// MergePair: SELL YES vs SELL NO — burn YES+NO pair, release backing collateral.
    ///
    /// - Canonical buyer (SELL NO → canonical bid): NO shares unreserved and burned.
    /// - Canonical seller (SELL YES → canonical ask): YES shares unreserved and burned.
    /// - Market: total_yes -= qty, total_no -= qty, market_collateral -= qty * 10^decimals.
    fn settle_merge_pair(
        &mut self,
        market_id: MarketId,
        fill: &Fill,
        canonical_buyer: &S::Address,
        canonical_seller: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let qty = fill.quantity;

        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        let token = &market.collateral_token;
        let scale = 10u128.pow(token.get_decimals() as u32);
        let expected_total_payout = Amount::from(qty.0)
            .checked_mul(Amount(scale))
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in qty scaling")))?;

        // Unreserve shares from both sides
        self.unlock_shares_to(
            canonical_buyer,
            None,
            market_id,
            OutcomeSide::No,
            qty,
            state,
        )?;
        self.unlock_shares_to(
            canonical_seller,
            None,
            market_id,
            OutcomeSide::Yes,
            qty,
            state,
        )?;

        // Move the pair into orderbook custody
        self.market
            .transfer_shares_from(
                market_id,
                canonical_buyer,
                self.id.to_payable(),
                Size::ZERO,
                qty,
                state,
            )
            .into_orderbook_err()?;
        self.market
            .transfer_shares_from(
                market_id,
                canonical_seller,
                self.id.to_payable(),
                qty,
                Size::ZERO,
                state,
            )
            .into_orderbook_err()?;

        // Now burn those shares to redeem collateral from the market to orderbook custody.
        self.market
            .burn_shares_from(market_id, qty, self.id.to_payable(), state)
            .into_orderbook_err()?;

        // Distribute redeemed collateral according to execution price.
        let yes_payout = fill.price.cost(qty, token);
        let no_payout = expected_total_payout
            .checked_sub(yes_payout)
            .ok_or_else(|| {
                OrderbookError::Any(anyhow::anyhow!("Underflow in MergePair collateral split"))
            })?;
        let total_payout = yes_payout.checked_add(no_payout).ok_or_else(|| {
            OrderbookError::Any(anyhow::anyhow!("Overflow in MergePair collateral split"))
        })?;
        if total_payout != expected_total_payout {
            return Err(OrderbookError::Any(anyhow::anyhow!(
                "MergePair collateral split mismatch: yes={} no={} total={} expected={}",
                yes_payout,
                no_payout,
                total_payout,
                expected_total_payout
            )));
        }

        self.bank
            .transfer_from(
                self.id.to_payable(),
                canonical_seller,
                Coins {
                    amount: yes_payout,
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_orderbook_err()?;
        self.bank
            .transfer_from(
                self.id.to_payable(),
                canonical_buyer,
                Coins {
                    amount: no_payout,
                    token_id: market.collateral_token,
                },
                state,
            )
            .into_orderbook_err()?;

        Ok(())
    }

    fn add_price_level(
        &mut self,
        market_id: MarketId,
        side: Side,
        price: Price,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = MarketSideKey { market_id, side };
        let mut price_levels = self
            .price_levels
            .get(&key, state)
            .into_orderbook_err()?
            .unwrap_or_default();

        if !price_levels.contains(&price) {
            price_levels.push(price);
            match side {
                Side::Bid => price_levels.sort_by(|a, b| b.cmp(a)),
                Side::Ask => price_levels.sort(),
            }
            self.price_levels
                .set(&key, &price_levels, state)
                .into_orderbook_err()?;
        }

        Ok(())
    }

    fn add_user_order(
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

    fn remove_price_level(
        &mut self,
        market_id: MarketId,
        side: Side,
        price: Price,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let key = MarketSideKey { market_id, side };
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

    fn remove_user_order(
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

    pub(crate) fn update_best_prices(
        &mut self,
        market_id: MarketId,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        // Best bid
        let bid_key = MarketSideKey {
            market_id,
            side: Side::Bid,
        };
        let bid_levels = self
            .price_levels
            .get(&bid_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        if let Some(&best) = bid_levels.first() {
            self.best_bid
                .set(&market_id, &best, state)
                .into_orderbook_err()?;
        } else {
            self.best_bid
                .remove(&market_id, state)
                .into_orderbook_err()?;
        }

        // Best ask
        let ask_key = MarketSideKey {
            market_id,
            side: Side::Ask,
        };
        let ask_levels = self
            .price_levels
            .get(&ask_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        if let Some(&best) = ask_levels.first() {
            self.best_ask
                .set(&market_id, &best, state)
                .into_orderbook_err()?;
        } else {
            self.best_ask
                .remove(&market_id, state)
                .into_orderbook_err()?;
        }

        self.emit_event(
            state,
            Event::BookUpdated {
                market_id,
                best_bid: bid_levels.first().copied(),
                best_ask: ask_levels.first().copied(),
            },
        );

        Ok(())
    }
}
