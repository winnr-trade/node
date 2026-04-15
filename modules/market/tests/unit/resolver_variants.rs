use crate::utils::{create_test_market, create_test_market_with_resolver, get_market};
use crate::{setup, RT, S};
use market::{CallMessage, MarketModule, MarketStatus, ResolutionData, Resolver};
use shared_types::Outcome;
use sov_modules_api::HexHash;
use sov_test_utils::{AsUser, TransactionTestCase};

// ============================================================================
// CREATE with different resolver variants
// ============================================================================

#[test]
fn test_create_market_with_pyth_resolver() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let feed_id = HexHash::new([0xAB; 32]);
    let resolver = Resolver::Pyth {
        feed_id,
        lower_bound: Some(1000),
        upper_bound: Some(2000),
    };

    let (market_id, _) = create_test_market_with_resolver(
        &mut runner,
        &user,
        resolver.clone(),
        "Will BTC be between $1000 and $2000?",
        collateral,
        86_400_000,
    );

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
    assert_eq!(market.resolver, resolver);
}

#[test]
fn test_create_market_with_pyth_resolver_unbounded() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let resolver = Resolver::Pyth {
        feed_id: HexHash::from([0x01; 32]),
        lower_bound: None,
        upper_bound: Some(50000),
    };

    let (market_id, _) = create_test_market_with_resolver(
        &mut runner,
        &user,
        resolver.clone(),
        "Will BTC be under $50k?",
        collateral,
        86_400_000,
    );

    let market = get_market(&runner, market_id);
    assert_eq!(market.resolver, resolver);
}

#[test]
fn test_create_market_with_optimistic_resolver() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market_with_resolver(
        &mut runner,
        &user,
        Resolver::Optimistic {},
        "Will it snow in July?",
        collateral,
        86_400_000,
    );

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
    assert_eq!(market.resolver, Resolver::Optimistic {});
}

// ============================================================================
// Resolve stub errors
// ============================================================================

#[test]
fn test_resolve_pyth_market_returns_not_implemented() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market_with_resolver(
        &mut runner,
        &user,
        Resolver::Pyth {
            feed_id: HexHash::from([0xAB; 32]),
            lower_bound: Some(1000),
            upper_bound: Some(2000),
        },
        "Will BTC be in range?",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    let msg = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Pyth {
            price_update_data: vec![1, 2, 3],
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected Pyth resolution to fail (not yet implemented)"
            );
        }),
    });

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
}

#[test]
fn test_resolve_optimistic_market_returns_not_implemented() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market_with_resolver(
        &mut runner,
        &user,
        Resolver::Optimistic {},
        "Will it snow in July?",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    // Any ResolutionData variant should fail on an Optimistic market
    let msg = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Address {
            outcome: Outcome::Yes,
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected Optimistic resolution to fail (not yet implemented)"
            );
        }),
    });

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
}

// ============================================================================
// Mismatched resolver type + resolution data
// ============================================================================

#[test]
fn test_resolve_address_market_with_pyth_data_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain?",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    // Send Pyth data to an Address-resolver market
    let msg = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Pyth {
            price_update_data: vec![1, 2, 3],
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected mismatch between Address resolver and Pyth data to fail"
            );
        }),
    });

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
}

#[test]
fn test_resolve_pyth_market_with_address_data_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market_with_resolver(
        &mut runner,
        &user,
        Resolver::Pyth {
            feed_id: HexHash::from([0xAB; 32]),
            lower_bound: Some(1000),
            upper_bound: Some(2000),
        },
        "Will BTC be in range?",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    // Send Address data to a Pyth-resolver market
    let msg = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Address {
            outcome: Outcome::Yes,
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected mismatch between Pyth resolver and Address data to fail"
            );
        }),
    });

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
}
