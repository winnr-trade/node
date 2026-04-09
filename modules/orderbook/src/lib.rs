//! Orderbook Module
//!
//! Unified canonical YES-space order book for binary prediction markets.
//! All orders are normalized to YES representation before entering the book:
//! - BUY NO @ p  → SELL YES @ (1-p)
//! - SELL NO @ p → BUY YES @ (1-p)
//!
//! Settlement uses four paths based on the original intents of both counterparties:
//! - MintPair: BUY YES vs BUY NO — mint new YES+NO pair from collateral.
//! - TransferYes: BUY YES vs SELL YES — transfer existing YES shares.
//! - TransferNo: BUY NO vs SELL NO — transfer existing NO shares.
//! - MergePair: SELL YES vs SELL NO — burn YES+NO pair, release backing collateral.

mod call;
mod error;
mod event;
mod genesis;
mod keys;
mod order;
mod types;

#[cfg(feature = "native")]
mod query;
#[cfg(feature = "native")]
pub use query::*;

pub use call::CallMessage;
pub use error::OrderbookError;
pub use event::Event;
pub use genesis::OrderbookGenesisConfig;
pub use keys::*;
use shielded_pool::ShieldedPoolModule;
pub use types::*;

// Re-export shared types
pub use shared_types::{MarketId, OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};

use market::MarketModule;
use sov_modules_api::{
    Context, GenesisState, Module, ModuleId, ModuleInfo, ModuleRestApi, Spec, StateMap, StateValue,
    TxState,
};

/// Orderbook Module — unified canonical YES-space CLOB.
#[derive(Clone, ModuleInfo, ModuleRestApi)]
pub struct OrderbookModule<S: Spec> {
    /// Module identifier
    #[id]
    pub id: ModuleId,

    /// Fee configuration
    #[state]
    pub config: StateValue<FeeConfig>,

    /// Next order ID counter
    #[state]
    pub next_order_id: StateValue<u64>,

    /// All orders by ID (stored in canonical form)
    #[state]
    pub orders: StateMap<OrderId, Order<S>>,

    /// User's open order IDs
    #[state]
    pub user_orders: StateMap<S::Address, Vec<OrderId>>,

    /// Locked collateral per user per market (for BUY orders)
    #[state]
    pub locked_collateral: StateMap<UserMarketKey<S>, u64>,

    /// Locked YES/NO shares per user per market (for SELL orders)
    #[state]
    pub locked_shares: StateMap<UserMarketKey<S>, LockedShares>,

    // ========================================================================
    // UNIFIED CANONICAL ORDER BOOK (YES-space)
    // ========================================================================
    /// Canonical YES bids at each price level
    #[state]
    pub bids: StateMap<PriceLevelKey, Vec<OrderId>>,

    /// Canonical YES asks at each price level
    #[state]
    pub asks: StateMap<PriceLevelKey, Vec<OrderId>>,

    /// Best canonical bid per market
    #[state]
    pub best_bid: StateMap<MarketId, Price>,

    /// Best canonical ask per market
    #[state]
    pub best_ask: StateMap<MarketId, Price>,

    /// Active price levels per market side (sorted)
    #[state]
    pub price_levels: StateMap<MarketSideKey, Vec<Price>>,

    // ========================================================================
    // MODULE DEPENDENCIES
    // ========================================================================
    /// Bank module for token transfers
    #[module]
    pub bank: sov_bank::Bank<S>,

    /// Chain state module for accessing chain information
    #[module]
    pub chain_state: sov_chain_state::ChainState<S>,

    /// Market module for position tracking & synthetic minting
    #[module]
    pub market: MarketModule<S>,

    /// Shielded pool module for stealth orders
    #[module]
    pub shielded_pool: ShieldedPoolModule<S>,
}

impl<S: Spec> Module for OrderbookModule<S> {
    type Spec = S;
    type Config = OrderbookGenesisConfig;
    type CallMessage = CallMessage<S>;
    type Event = Event;
    type Error = OrderbookError;

    fn genesis(
        &mut self,
        _header: &<S::Da as sov_modules_api::DaSpec>::BlockHeader,
        config: &Self::Config,
        state: &mut impl GenesisState<S>,
    ) -> anyhow::Result<()> {
        self.init_module(_header, config, state)
    }

    fn call(
        &mut self,
        msg: Self::CallMessage,
        ctx: &Context<S>,
        state: &mut impl TxState<S>,
    ) -> Result<(), Self::Error> {
        match msg {
            CallMessage::PlaceOrderNormal {
                market_id,
                outcome,
                side,
                price,
                quantity,
                order_type,
            } => self.place_order_normal(
                OrderRequest {
                    market_id,
                    outcome,
                    side,
                    price,
                    quantity,
                    order_type,
                },
                ctx,
                state,
            ),

            CallMessage::PlaceOrderStealth {
                proof,
                commitment,
                nullifier,
                stealth_address,
                market_id,
                token_id,
                outcome,
                side,
                price,
                quantity,
                order_type,
            } => self.place_order_stealth(
                OrderRequest {
                    market_id,
                    outcome,
                    side,
                    price,
                    quantity,
                    order_type,
                },
                proof,
                commitment,
                nullifier,
                &stealth_address,
                token_id,
                ctx,
                state,
            ),

            CallMessage::CancelOrder { order_id } => self.cancel_order(order_id, ctx, state),

            CallMessage::CancelAllOrders { market_id, outcome } => {
                self.cancel_all_orders(market_id, outcome, ctx, state)
            }
        }
    }
}
