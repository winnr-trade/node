use crate::utils::{create_test_market, get_market, get_market_collateral};
use crate::{setup, RT, S};
use market::{CallMessage, MarketModule, MarketStatus, Resolver};
use shared_types::{MarketId, Size};
use sov_modules_api::SafeString;
use sov_test_utils::{AsUser, TransactionTestCase};
use std::str::FromStr;

#[test]
fn test_create_market_success() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, resolution_time) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    // Verify all market properties are correctly set
    let market = get_market(&runner, market_id);
    assert_eq!(market.id, MarketId(0));
    assert_eq!(market.question.to_string(), "Will it rain tomorrow?");
    assert_eq!(market.creator, user.address());
    assert_eq!(market.collateral_token, collateral);
    assert_eq!(market.resolution_time, resolution_time);
    assert_eq!(market.status(), MarketStatus::Active);
    assert_eq!(market.outcome, None);
    assert_eq!(market.resolver, Resolver::Address(user.address()));
    assert_eq!(market.total_shares, Size::ZERO);
    assert!(
        market.created_at > 0,
        "created_at should be set to current time"
    );
    assert_eq!(get_market_collateral(&runner, market_id), 0);
}

#[test]
fn test_create_market_resolution_time_in_past_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;

    let create_market_msg = CallMessage::CreateMarket {
        question: SafeString::from_str("Will it rain?").ok().unwrap(),
        collateral_token: test_data.collateral_token_id,
        resolution_time: 0,
        resolver: Resolver::Address(user.address()),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(create_market_msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected create_market to fail with resolution time in the past"
            );
        }),
    });
}

#[test]
fn test_create_multiple_markets() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let admin = test_data.admin;
    let collateral = test_data.collateral_token_id;

    create_test_market(
        &mut runner,
        &user,
        &user,
        "Will BTC hit 100k?",
        collateral,
        86_400_000,
    );
    create_test_market(
        &mut runner,
        &admin,
        &user,
        "Will ETH hit 10k?",
        collateral,
        172_800_000,
    );
    create_test_market(
        &mut runner,
        &user,
        &admin,
        "Will SOL hit 1k?",
        collateral,
        259_200_000,
    );

    // Verify each market has the correct id and distinct properties
    let m1 = get_market(&runner, MarketId(0));
    assert_eq!(m1.id, MarketId(0));
    assert_eq!(m1.question.to_string(), "Will BTC hit 100k?");
    assert_eq!(m1.creator, user.address());
    assert_eq!(m1.resolver, Resolver::Address(user.address()));

    let m2 = get_market(&runner, MarketId(1));
    assert_eq!(m2.id, MarketId(1));
    assert_eq!(m2.question.to_string(), "Will ETH hit 10k?");
    assert_eq!(m2.creator, admin.address());
    assert_eq!(m2.resolver, Resolver::Address(user.address()));

    let m3 = get_market(&runner, MarketId(2));
    assert_eq!(m3.id, MarketId(2));
    assert_eq!(m3.question.to_string(), "Will SOL hit 1k?");
    assert_eq!(m3.creator, user.address());
    assert_eq!(m3.resolver, Resolver::Address(admin.address()));
}
