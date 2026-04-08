use std::str::FromStr;

use crate::{RT, S};
use market::MarketModule;
use orderbook::{
    MarketSideKey, Order, OrderbookError, OrderbookModule, PriceLevelKey, UserMarketKey,
};
use shared_types::{MarketId, OrderId, OrderType, OutcomeSide, Price, Side};
use sov_bank::TokenId;
use sov_modules_api::SafeString;
use sov_modules_api::TxEffect;
use sov_test_utils::runtime::TestRunner;
use sov_test_utils::{AsUser, TestUser, TransactionTestCase};

/// Serialize an `OrderbookError` to extract its serde `error_code` tag.
fn error_code_of(err: &OrderbookError) -> String {
    let value = serde_json::to_value(err).expect("failed to serialize OrderbookError");
    value
        .get("error_code")
        .expect("serialized error missing 'error_code' field")
        .as_str()
        .expect("'error_code' is not a string")
        .to_owned()
}

/// Extract the `error_code` string from a reverted transaction receipt.
fn extract_error_code(tx_receipt: &sov_modules_api::TxEffect<S>) -> String {
    match tx_receipt {
        TxEffect::Reverted(reverted) => {
            let detail = reverted
                .reason
                .error_detail()
                .expect("failed to extract error detail");
            detail
                .get("error_code")
                .expect("error detail missing 'error_code' field")
                .as_str()
                .expect("'error_code' is not a string")
                .to_owned()
        }
        other => panic!("Expected Reverted receipt, got: {:?}", other),
    }
}

// ============================================================================
// Market helpers
// ============================================================================

pub fn get_time_ms(runner: &TestRunner<RT, S>) -> u64 {
    runner.query_state(|state| {
        runner
            .runtime()
            .chain_state
            .get_time(state)
            .expect("failed to read chain time")
            .as_millis() as u64
    })
}

pub fn set_supported_collateral_token(
    runner: &mut TestRunner<RT, S>,
    admin: &TestUser<S>,
    token_id: TokenId,
) {
    let msg = market::CallMessage::<S>::SetSupportedCollateralToken {
        token_id,
        support: true,
    };

    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "set_supported_collateral_token failed: {:?}",
                result.tx_receipt
            );
        }),
    });
}

pub fn create_test_market(
    runner: &mut TestRunner<RT, S>,
    creator: &TestUser<S>,
    resolver: &TestUser<S>,
    collateral_token: TokenId,
) -> MarketId {
    let resolution_time = get_time_ms(runner) + 1_000_000;

    let expected_id = runner.query_state(|state| {
        runner
            .runtime()
            .market
            .next_market_id
            .get(state)
            .expect("failed to read next_market_id")
            .expect("next_market_id not set")
    });

    let msg = market::CallMessage::<S>::CreateMarket {
        question: SafeString::from_str("Will this test pass?").ok().unwrap(),
        collateral_token,
        resolution_time,
        resolver: resolver.address(),
    };

    runner.execute_transaction(TransactionTestCase {
        input: creator.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "create_market failed: {:?}",
                result.tx_receipt
            );
        }),
    });

    MarketId(expected_id)
}

pub fn halt_market(runner: &mut TestRunner<RT, S>, admin: &TestUser<S>, market_id: MarketId) {
    let msg = market::CallMessage::<S>::HaltMarket { market_id };

    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "halt_market failed: {:?}",
                result.tx_receipt
            );
        }),
    });
}

// ============================================================================
// Orderbook transaction helpers
// ============================================================================

pub fn place_order(
    runner: &mut TestRunner<RT, S>,
    user: &TestUser<S>,
    market_id: MarketId,
    outcome: OutcomeSide,
    side: Side,
    price: Price,
    quantity: u64,
    order_type: OrderType,
) {
    let msg = orderbook::CallMessage::PlaceOrderNormal {
        market_id,
        outcome,
        side,
        price,
        quantity,
        order_type,
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, OrderbookModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "place_order failed: {:?}",
                result.tx_receipt
            );
        }),
    });
}

pub fn place_order_should_fail(
    runner: &mut TestRunner<RT, S>,
    user: &TestUser<S>,
    market_id: MarketId,
    outcome: OutcomeSide,
    side: Side,
    price: Price,
    quantity: u64,
    order_type: OrderType,
    expected_error: OrderbookError,
) {
    let expected = error_code_of(&expected_error);
    let msg = orderbook::CallMessage::PlaceOrderNormal {
        market_id,
        outcome,
        side,
        price,
        quantity,
        order_type,
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, OrderbookModule<S>>(msg),
        assert: Box::new(move |result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "place_order should have failed but succeeded"
            );
            let error_code = extract_error_code(&result.tx_receipt);
            assert_eq!(
                error_code, expected,
                "expected error code '{}', got '{}'",
                expected, error_code
            );
        }),
    });
}

pub fn cancel_order(runner: &mut TestRunner<RT, S>, user: &TestUser<S>, order_id: OrderId) {
    let msg = orderbook::CallMessage::CancelOrder { order_id };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, OrderbookModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "cancel_order failed: {:?}",
                result.tx_receipt
            );
        }),
    });
}

pub fn cancel_order_should_fail(
    runner: &mut TestRunner<RT, S>,
    user: &TestUser<S>,
    order_id: OrderId,
    expected_error: OrderbookError,
) {
    let expected = error_code_of(&expected_error);
    let msg = orderbook::CallMessage::CancelOrder { order_id };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, OrderbookModule<S>>(msg),
        assert: Box::new(move |result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "cancel_order should have failed but succeeded"
            );
            let error_code = extract_error_code(&result.tx_receipt);
            assert_eq!(
                error_code, expected,
                "expected error code '{}', got '{}'",
                expected, error_code
            );
        }),
    });
}

pub fn cancel_all_orders(
    runner: &mut TestRunner<RT, S>,
    user: &TestUser<S>,
    market_id: MarketId,
    outcome: Option<OutcomeSide>,
) {
    let msg = orderbook::CallMessage::CancelAllOrders { market_id, outcome };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, OrderbookModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "cancel_all_orders failed: {:?}",
                result.tx_receipt
            );
        }),
    });
}

// ============================================================================
// State query helpers
// ============================================================================

pub fn get_order(runner: &TestRunner<RT, S>, order_id: OrderId) -> Order<S> {
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .orders
            .get(&order_id, state)
            .expect("failed to read order state")
            .unwrap_or_else(|| panic!("order {:?} not found in state", order_id))
    })
}

pub fn try_get_order(runner: &TestRunner<RT, S>, order_id: OrderId) -> Option<Order<S>> {
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .orders
            .get(&order_id, state)
            .expect("failed to read order state")
    })
}

pub fn get_next_order_id(runner: &TestRunner<RT, S>) -> u64 {
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .next_order_id
            .get(state)
            .expect("failed to read next_order_id")
            .expect("next_order_id not set")
    })
}

/// Get the best canonical bid for a market.
pub fn get_best_bid(runner: &TestRunner<RT, S>, market_id: MarketId) -> Option<Price> {
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .best_bid
            .get(&market_id, state)
            .expect("failed to read best_bid")
    })
}

/// Get the best canonical ask for a market.
pub fn get_best_ask(runner: &TestRunner<RT, S>, market_id: MarketId) -> Option<Price> {
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .best_ask
            .get(&market_id, state)
            .expect("failed to read best_ask")
    })
}

/// Get canonical price levels for a market side.
pub fn get_price_levels(runner: &TestRunner<RT, S>, market_id: MarketId, side: Side) -> Vec<Price> {
    let key = MarketSideKey { market_id, side };
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .price_levels
            .get(&key, state)
            .expect("failed to read price_levels")
            .unwrap_or_default()
    })
}

pub fn get_bids_at_price(
    runner: &TestRunner<RT, S>,
    market_id: MarketId,
    price: Price,
) -> Vec<OrderId> {
    let key = PriceLevelKey { market_id, price };
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .bids
            .get(&key, state)
            .expect("failed to read bids")
            .unwrap_or_default()
    })
}

pub fn get_asks_at_price(
    runner: &TestRunner<RT, S>,
    market_id: MarketId,
    price: Price,
) -> Vec<OrderId> {
    let key = PriceLevelKey { market_id, price };
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .asks
            .get(&key, state)
            .expect("failed to read asks")
            .unwrap_or_default()
    })
}

pub fn get_user_orders(runner: &TestRunner<RT, S>, user: &TestUser<S>) -> Vec<OrderId> {
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .user_orders
            .get(&user.address(), state)
            .expect("failed to read user_orders")
            .unwrap_or_default()
    })
}

pub fn get_locked_collateral(
    runner: &TestRunner<RT, S>,
    user: &TestUser<S>,
    market_id: MarketId,
) -> u64 {
    let key = UserMarketKey {
        address: user.address(),
        market_id,
    };
    runner.query_state(|state| {
        runner
            .runtime()
            .orderbook
            .locked_collateral
            .get(&key, state)
            .expect("failed to read locked_collateral")
            .unwrap_or(0)
    })
}

/// Get user's YES shares in a market.
pub fn get_yes_shares(runner: &TestRunner<RT, S>, user: &TestUser<S>, market_id: MarketId) -> u64 {
    let key = market::PositionKey::<S> {
        market_id,
        address: user.address(),
    };
    runner.query_state(|state| {
        runner
            .runtime()
            .market
            .positions
            .get(&key, state)
            .expect("failed to read position")
            .map(|p| p.yes_shares)
            .unwrap_or(0)
    })
}

/// Get user's NO shares in a market.
pub fn get_no_shares(runner: &TestRunner<RT, S>, user: &TestUser<S>, market_id: MarketId) -> u64 {
    let key = market::PositionKey::<S> {
        market_id,
        address: user.address(),
    };
    runner.query_state(|state| {
        runner
            .runtime()
            .market
            .positions
            .get(&key, state)
            .expect("failed to read position")
            .map(|p| p.no_shares)
            .unwrap_or(0)
    })
}
