use crate::error::IntoOrderbookError;
use crate::{BookKey, OrderbookError, OrderbookModule};
use market::{MarketId, MarketStatus};
use shared_types::{OrderId, OrderType, OutcomeSide, Price, Side};
use sov_modules_api::{Spec, TxState};

impl<S: Spec> OrderbookModule<S> {
    pub(crate) fn validate_order(
        &self,
        price: &Price,
        quantity: u64,
        order_type: &OrderType,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let config = self
            .config
            .get(state)
            .into_orderbook_err()?
            .unwrap_or_default();

        if quantity == 0 {
            return Err(OrderbookError::ZeroQuantity);
        }

        if quantity < config.min_order_size {
            return Err(OrderbookError::OrderTooSmall {
                size: quantity,
                minimum: config.min_order_size,
            });
        }

        if *order_type != OrderType::Market && !price.is_valid() {
            return Err(OrderbookError::InvalidPrice { price: price.0 });
        }

        Ok(())
    }

    pub(crate) fn verify_market_active(
        &self,
        market_id: MarketId,
        state: &mut impl TxState<S>,
    ) -> Result<(), OrderbookError> {
        let market = self
            .market
            .markets
            .get(&market_id, state)
            .into_orderbook_err()?
            .ok_or(OrderbookError::MarketNotFound { market_id })?;

        if market.status != MarketStatus::Active {
            return Err(OrderbookError::MarketNotActive {
                market_id,
                status: format!("{:?}", market.status),
            });
        }

        Ok(())
    }

    pub(crate) fn would_match(
        &self,
        market_id: MarketId,
        outcome: OutcomeSide,
        side: Side,
        price: Price,
        state: &mut impl TxState<S>,
    ) -> Result<bool, OrderbookError> {
        let book_key = BookKey { market_id, outcome };

        match side {
            Side::Bid => {
                if let Some(best_ask) = self.best_ask.get(&book_key, state).into_orderbook_err()? {
                    return Ok(price >= best_ask);
                }
            }
            Side::Ask => {
                if let Some(best_bid) = self.best_bid.get(&book_key, state).into_orderbook_err()? {
                    return Ok(price <= best_bid);
                }
            }
        }

        Ok(false)
    }

    pub(crate) fn next_order_id(
        &mut self,
        state: &mut impl TxState<S>,
    ) -> Result<OrderId, OrderbookError> {
        let id = self
            .next_order_id
            .get(state)
            .into_orderbook_err()?
            .ok_or_else(|| anyhow::anyhow!("Module not initialized"))?;
        self.next_order_id
            .set(&(id + 1), state)
            .into_orderbook_err()?;
        Ok(OrderId(id))
    }
}
