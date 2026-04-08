use crate::error::IntoOrderbookError;
use crate::{
    Event, Fill, MarketSideKey, Order, OrderbookError, OrderbookModule, PriceLevelKey,
    SettlementKind,
};
use market::{MarketId, PositionKey};
use shared_types::{OrderId, OutcomeSide, Price, Side};
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
    /// - Market: total_yes += qty, total_no += qty, market_collateral += qty.
    fn settle_mint_pair(
        &mut self,
        market_id: MarketId,
        fill: &Fill,
        canonical_buyer: &S::Address,
        canonical_seller: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let qty = fill.quantity;

        // Update market supply totals
        let mut market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        market.total_yes_shares = market
            .total_yes_shares
            .checked_add(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in total_yes_shares")))?;
        market.total_no_shares = market
            .total_no_shares
            .checked_add(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in total_no_shares")))?;
        self.market
            .markets
            .set(&market_id, &market, state)
            .into_orderbook_err()?;

        // Update market collateral backing (1 unit per pair)
        let current_collateral = self
            .market
            .market_collateral
            .get(&market_id, state)
            .into_orderbook_err()?
            .unwrap_or(0);
        let new_collateral = current_collateral
            .checked_add(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in market_collateral")))?;
        self.market
            .market_collateral
            .set(&market_id, &new_collateral, state)
            .into_orderbook_err()?;

        // Allocate YES shares to canonical buyer
        let buyer_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_buyer.clone(),
        };
        let mut buyer_pos = self
            .market
            .positions
            .get(&buyer_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        buyer_pos.yes_shares = buyer_pos
            .yes_shares
            .checked_add(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in yes_shares")))?;
        self.market
            .positions
            .set(&buyer_key, &buyer_pos, state)
            .into_orderbook_err()?;

        // Allocate NO shares to canonical seller
        let seller_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_seller.clone(),
        };
        let mut seller_pos = self
            .market
            .positions
            .get(&seller_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        seller_pos.no_shares = seller_pos
            .no_shares
            .checked_add(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in no_shares")))?;
        self.market
            .positions
            .set(&seller_key, &seller_pos, state)
            .into_orderbook_err()?;

        // Release collateral from both sides (consumed into backing)
        let buyer_release = fill.price.cost(qty);
        self.unlock_collateral(canonical_buyer, market_id, buyer_release, state)?;

        let seller_release = fill.price.complement().cost(qty);
        self.unlock_collateral(canonical_seller, market_id, seller_release, state)?;

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
        let qty = fill.quantity;

        // Unlock seller's YES shares using generic unlock helper
        self.unlock_shares(canonical_seller, market_id, OutcomeSide::Yes, qty, state)?;

        // Transfer YES: seller → buyer
        let seller_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_seller.clone(),
        };
        let mut seller_pos = self
            .market
            .positions
            .get(&seller_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        seller_pos.yes_shares = seller_pos.yes_shares.checked_sub(qty).ok_or_else(|| {
            OrderbookError::Any(anyhow::anyhow!("Underflow in seller yes_shares"))
        })?;
        self.market
            .positions
            .set(&seller_key, &seller_pos, state)
            .into_orderbook_err()?;

        let buyer_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_buyer.clone(),
        };
        let mut buyer_pos = self
            .market
            .positions
            .get(&buyer_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        buyer_pos.yes_shares = buyer_pos
            .yes_shares
            .checked_add(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in buyer yes_shares")))?;
        self.market
            .positions
            .set(&buyer_key, &buyer_pos, state)
            .into_orderbook_err()?;

        // Release buyer's collateral (consumed as payment)
        let buyer_release = fill.price.cost(qty);
        self.unlock_collateral(canonical_buyer, market_id, buyer_release, state)?;

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
        let qty = fill.quantity;

        // Unlock buyer's NO shares using generic unlock helper
        self.unlock_shares(canonical_buyer, market_id, OutcomeSide::No, qty, state)?;

        // Transfer NO: canonical buyer → canonical seller
        let buyer_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_buyer.clone(),
        };
        let mut buyer_pos = self
            .market
            .positions
            .get(&buyer_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        buyer_pos.no_shares = buyer_pos
            .no_shares
            .checked_sub(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Underflow in buyer no_shares")))?;
        self.market
            .positions
            .set(&buyer_key, &buyer_pos, state)
            .into_orderbook_err()?;

        let seller_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_seller.clone(),
        };
        let mut seller_pos = self
            .market
            .positions
            .get(&seller_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        seller_pos.no_shares = seller_pos
            .no_shares
            .checked_add(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Overflow in seller no_shares")))?;
        self.market
            .positions
            .set(&seller_key, &seller_pos, state)
            .into_orderbook_err()?;

        // Release canonical seller's collateral (they are BUY NO, consumed as payment)
        let seller_release = fill.price.complement().cost(qty);
        self.unlock_collateral(canonical_seller, market_id, seller_release, state)?;

        Ok(())
    }

    /// MergePair: SELL YES vs SELL NO — burn YES+NO pair, release backing collateral.
    ///
    /// - Canonical buyer (SELL NO → canonical bid): NO shares unreserved and burned.
    /// - Canonical seller (SELL YES → canonical ask): YES shares unreserved and burned.
    /// - Market: total_yes -= qty, total_no -= qty, market_collateral -= qty.
    fn settle_merge_pair(
        &mut self,
        market_id: MarketId,
        fill: &Fill,
        canonical_buyer: &S::Address,
        canonical_seller: &S::Address,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let qty = fill.quantity;

        // Unreserve shares from both sides
        self.unlock_shares(canonical_buyer, market_id, OutcomeSide::No, qty, state)?;
        self.unlock_shares(canonical_seller, market_id, OutcomeSide::Yes, qty, state)?;

        // Burn NO shares from canonical buyer (SELL NO)
        let buyer_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_buyer.clone(),
        };
        let mut buyer_pos = self
            .market
            .positions
            .get(&buyer_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        buyer_pos.no_shares = buyer_pos
            .no_shares
            .checked_sub(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Underflow in buyer no_shares")))?;
        self.market
            .positions
            .set(&buyer_key, &buyer_pos, state)
            .into_orderbook_err()?;

        // Burn YES shares from canonical seller (SELL YES)
        let seller_key: PositionKey<S> = PositionKey {
            market_id,
            address: canonical_seller.clone(),
        };
        let mut seller_pos = self
            .market
            .positions
            .get(&seller_key, state)
            .into_orderbook_err()?
            .unwrap_or_default();
        seller_pos.yes_shares = seller_pos.yes_shares.checked_sub(qty).ok_or_else(|| {
            OrderbookError::Any(anyhow::anyhow!("Underflow in seller yes_shares"))
        })?;
        self.market
            .positions
            .set(&seller_key, &seller_pos, state)
            .into_orderbook_err()?;

        // Decrease market supply totals
        let mut market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        market.total_yes_shares = market
            .total_yes_shares
            .checked_sub(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Underflow in total_yes_shares")))?;
        market.total_no_shares = market
            .total_no_shares
            .checked_sub(qty)
            .ok_or_else(|| OrderbookError::Any(anyhow::anyhow!("Underflow in total_no_shares")))?;
        self.market
            .markets
            .set(&market_id, &market, state)
            .into_orderbook_err()?;

        // Release backing collateral
        let current_collateral = self
            .market
            .market_collateral
            .get(&market_id, state)
            .into_orderbook_err()?
            .unwrap_or(0);
        let new_collateral = current_collateral.checked_sub(qty).ok_or_else(|| {
            OrderbookError::Any(anyhow::anyhow!("Underflow in market_collateral"))
        })?;
        if new_collateral == 0 {
            self.market
                .market_collateral
                .remove(&market_id, state)
                .into_orderbook_err()?;
        } else {
            self.market
                .market_collateral
                .set(&market_id, &new_collateral, state)
                .into_orderbook_err()?;
        }

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
