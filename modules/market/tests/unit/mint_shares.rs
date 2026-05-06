use crate::utils::{
    create_test_market, get_market, get_market_collateral, get_position,
    mint_shares as mint_shares_helper,
};
use crate::{setup, RT, S};
use market::{CallMessage, MarketModule};
use shared_types::{MarketId, Size};
use sov_test_utils::{AsUser, TransactionTestCase};

#[test]
fn test_mint_shares_success() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    mint_shares_helper(&mut runner, &user, market_id, 100);

    // Verify market totals
    let market = get_market(&runner, market_id);
    assert_eq!(market.total_shares, Size(100));

    // Verify user position
    let position = get_position(&runner, market_id, &user).expect("position should exist");
    assert_eq!(position.yes_shares, Size(100));
    assert_eq!(position.no_shares, Size(100));

    // Verify collateral tracking
    assert_eq!(get_market_collateral(&runner, market_id), 100_000_000);
}

#[test]
fn test_mint_shares_multiple_times() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    mint_shares_helper(&mut runner, &user, market_id, 100);
    mint_shares_helper(&mut runner, &user, market_id, 200);
    mint_shares_helper(&mut runner, &user, market_id, 50);

    // Verify cumulative totals
    let market = get_market(&runner, market_id);
    assert_eq!(market.total_shares, Size(350));

    let position = get_position(&runner, market_id, &user).expect("position should exist");
    assert_eq!(position.yes_shares, Size(350));
    assert_eq!(position.no_shares, Size(350));

    assert_eq!(get_market_collateral(&runner, market_id), 350_000_000);
}

#[test]
fn test_mint_shares_zero_amount_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    let msg = CallMessage::MintShares {
        market_id,
        amount: Size::ZERO,
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected mint_shares to fail with zero amount"
            );
        }),
    });
}

#[test]
fn test_mint_shares_nonexistent_market_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;

    let msg = CallMessage::MintShares {
        market_id: MarketId(999),
        amount: Size(100),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected mint_shares to fail for nonexistent market"
            );
        }),
    });
}

#[test]
fn test_mint_shares_different_users() {
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
        86_400_000,
    );

    mint_shares_helper(&mut runner, &user, market_id, 100);
    mint_shares_helper(&mut runner, &admin, market_id, 200);

    // Verify each user has their own position
    let user_pos = get_position(&runner, market_id, &user).expect("user position should exist");
    assert_eq!(user_pos.yes_shares, Size(100));
    assert_eq!(user_pos.no_shares, Size(100));

    let admin_pos = get_position(&runner, market_id, &admin).expect("admin position should exist");
    assert_eq!(admin_pos.yes_shares, Size(200));
    assert_eq!(admin_pos.no_shares, Size(200));

    // Verify combined market totals
    let market = get_market(&runner, market_id);
    assert_eq!(market.total_shares, Size(300));
    assert_eq!(get_market_collateral(&runner, market_id), 300_000_000);
}
