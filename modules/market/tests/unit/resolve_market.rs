use crate::utils::{create_test_market, get_market, resolve_market as resolve_market_helper};
use crate::{setup, RT, S};
use market::ResolutionData;
use market::{CallMessage, MarketModule, MarketStatus};
use shared_types::{MarketId, Outcome};
use sov_test_utils::{AsUser, TransactionTestCase};

#[test]
fn test_resolve_market_success() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain tomorrow?",
        collateral,
        100_000,
    );

    // Advance time past the resolution time (~280ms per slot, need ~500 for 100s)
    runner.advance_slots(500);

    resolve_market_helper(&mut runner, &user, market_id, Outcome::Yes);

    // Verify market state after resolution
    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Resolved);
    assert_eq!(market.outcome, Some(Outcome::Yes));
}

#[test]
fn test_resolve_market_outcome_no() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will BTC hit 200k?",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    resolve_market_helper(&mut runner, &user, market_id, Outcome::No);

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Resolved);
    assert_eq!(market.outcome, Some(Outcome::No));
}

#[test]
fn test_resolve_market_outcome_invalid() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Ambiguous question",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    resolve_market_helper(&mut runner, &user, market_id, Outcome::Invalid);

    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Resolved);
    assert_eq!(market.outcome, Some(Outcome::Invalid));
}

#[test]
fn test_resolve_market_wrong_resolver_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let admin = test_data.admin;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain tomorrow?",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    // Try to resolve with admin (not the designated resolver)
    let resolve_msg = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Address {
            outcome: Outcome::Yes,
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(resolve_msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected resolve_market to fail with wrong resolver"
            );
        }),
    });

    // Market should still be active
    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
    assert_eq!(market.outcome, None);
}

#[test]
fn test_resolve_market_before_resolution_time_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain next year?",
        collateral,
        86_400_000 * 365,
    );

    // Don't advance time — try to resolve immediately
    let resolve_msg = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Address {
            outcome: Outcome::Yes,
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(resolve_msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected resolve_market to fail before resolution time"
            );
        }),
    });

    // Market should still be active
    let market = get_market(&runner, market_id);
    assert_eq!(market.status, MarketStatus::Active);
    assert_eq!(market.outcome, None);
}

#[test]
fn test_resolve_nonexistent_market_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;

    let resolve_msg = CallMessage::ResolveMarket {
        market_id: MarketId(999),
        data: ResolutionData::Address {
            outcome: Outcome::Yes,
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(resolve_msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected resolve_market to fail for nonexistent market"
            );
        }),
    });
}

#[test]
fn test_resolve_market_twice_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain tomorrow?",
        collateral,
        100_000,
    );

    runner.advance_slots(500);

    // First resolution should succeed
    resolve_market_helper(&mut runner, &user, market_id, Outcome::Yes);

    // Second resolution should fail
    let resolve_msg_2 = CallMessage::ResolveMarket {
        market_id,
        data: ResolutionData::Address {
            outcome: Outcome::No,
        },
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(resolve_msg_2),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected second resolve_market to fail"
            );
        }),
    });

    // Outcome should still be the first resolution (Yes)
    let market = get_market(&runner, market_id);
    assert_eq!(market.outcome, Some(Outcome::Yes));
}
