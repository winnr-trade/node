use std::str::FromStr;

use crate::{RT, S};
use market::ResolutionData;
use market::{CallMessage, Market, MarketModule, Position, PositionKey, Resolver};
use shared_types::{MarketId, Outcome};
use sov_bank::utils::TokenHolder;
use sov_bank::TokenId;
use sov_modules_api::da::Time;
use sov_modules_api::SafeString;
use sov_test_utils::runtime::TestRunner;
use sov_test_utils::{AsUser, TestUser, TransactionTestCase};

pub fn get_time(runner: &TestRunner<RT, S>) -> Time {
    runner.query_state(|state| {
        runner
            .runtime()
            .chain_state
            .get_time(state)
            .expect("failed to read chain time")
    })
}

pub fn get_time_ms(runner: &TestRunner<RT, S>) -> u64 {
    get_time(runner).as_millis() as u64
}

/// Helper: create a market and assert success. Returns the MarketId and resolution_time.
pub fn create_test_market(
    runner: &mut TestRunner<RT, S>,
    creator: &TestUser<S>,
    resolver: &TestUser<S>,
    question: &str,
    collateral_token: TokenId,
    resolution_time_offset_ms: u64,
) -> (MarketId, u64) {
    let resolution_time = get_time_ms(runner) + resolution_time_offset_ms;

    // Read the next_market_id before creation so we know which ID will be assigned
    let expected_id = runner.query_state(|state| {
        runner
            .runtime()
            .market
            .next_market_id
            .get(state)
            .expect("failed to read next_market_id")
            .expect("next_market_id not set")
    });

    let msg = CallMessage::CreateMarket {
        question: SafeString::from_str(question).ok().unwrap(),
        collateral_token,
        resolution_time,
        resolver: Resolver::Address(resolver.address()),
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

    (MarketId(expected_id), resolution_time)
}

/// Helper: mint shares on a market and assert success.
pub fn mint_shares(
    runner: &mut TestRunner<RT, S>,
    user: &TestUser<S>,
    market_id: MarketId,
    amount: u64,
) {
    let msg = CallMessage::MintShares { market_id, amount };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "mint_shares failed: {:?}",
                result.tx_receipt
            );
        }),
    });
}

/// Helper: resolve a market and assert success.
pub fn resolve_market(
    runner: &mut TestRunner<RT, S>,
    resolver: &TestUser<S>,
    market_id: MarketId,
    outcome: Outcome,
) {
    let msg = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Address { outcome },
    };

    runner.execute_transaction(TransactionTestCase {
        input: resolver.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "resolve_market failed: {:?}",
                result.tx_receipt
            );
        }),
    });
}

/// Query a market from state by its ID.
pub fn get_market(runner: &TestRunner<RT, S>, market_id: MarketId) -> Market<S> {
    runner.query_state(|state| {
        runner
            .runtime()
            .market
            .markets
            .get(&market_id, state)
            .expect("failed to read market state")
            .unwrap_or_else(|| panic!("market {:?} not found in state", market_id))
    })
}

/// Query a user's position in a market. Returns `None` if no position exists.
pub fn get_position(
    runner: &TestRunner<RT, S>,
    market_id: MarketId,
    user: &TestUser<S>,
) -> Option<Position> {
    runner.query_state(|state| {
        let key = PositionKey {
            market_id,
            owner: TokenHolder::User(user.address()),
        };
        runner
            .runtime()
            .market
            .positions
            .get(&key, state)
            .expect("failed to read position state")
    })
}

/// Query total collateral held for a market.
pub fn get_market_collateral(runner: &TestRunner<RT, S>, market_id: MarketId) -> u64 {
    runner.query_state(|state| {
        runner
            .runtime()
            .market
            .market_collateral
            .get(&market_id, state)
            .expect("failed to read market_collateral state")
            .unwrap_or(0)
    })
}

/// Helper: create a market with an explicit resolver and assert success.
pub fn create_test_market_with_resolver(
    runner: &mut TestRunner<RT, S>,
    creator: &TestUser<S>,
    resolver: Resolver<S>,
    question: &str,
    collateral_token: TokenId,
    resolution_time_offset_ms: u64,
) -> (MarketId, u64) {
    let resolution_time = get_time_ms(runner) + resolution_time_offset_ms;

    let expected_id = runner.query_state(|state| {
        runner
            .runtime()
            .market
            .next_market_id
            .get(state)
            .expect("failed to read next_market_id")
            .expect("next_market_id not set")
    });

    let msg = CallMessage::CreateMarket {
        question: SafeString::from_str(question).ok().unwrap(),
        collateral_token,
        resolution_time,
        resolver,
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

    (MarketId(expected_id), resolution_time)
}
