use crate::utils::{
    create_test_market, get_market, get_market_collateral, get_position, mint_shares,
};
use crate::{setup, RT, S};
use market::{CallMessage, MarketModule};
use shared_types::{MarketId, Size};
use sov_test_utils::{AsUser, TransactionTestCase};

#[test]
fn test_redeem_shares_success() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) =
        create_test_market(&mut runner, &user, &user, "Will it rain tomorrow?", collateral, 86_400_000);

    mint_shares(&mut runner, &user, market_id, 100);

    let msg = CallMessage::RedeemShares {
        market_id,
        amount: Size(50),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "redeem_shares failed: {:?}",
                result.tx_receipt
            );
        }),
    });

    // Verify remaining position
    let position = get_position(&runner, market_id, &user).expect("position should exist");
    assert_eq!(position.yes_shares, Size(50));
    assert_eq!(position.no_shares, Size(50));

    // Verify market totals reduced
    let market = get_market(&runner, market_id);
    assert_eq!(market.total_shares, Size(50));
    assert_eq!(get_market_collateral(&runner, market_id), 50);
}

#[test]
fn test_redeem_all_shares() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) =
        create_test_market(&mut runner, &user, &user, "Will it rain tomorrow?", collateral, 86_400_000);

    mint_shares(&mut runner, &user, market_id, 100);

    let msg = CallMessage::RedeemShares {
        market_id,
        amount: Size(100),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "redeem_shares (all) failed: {:?}",
                result.tx_receipt
            );
        }),
    });

    // Position should be removed after redeeming all shares
    assert!(
        get_position(&runner, market_id, &user).is_none(),
        "position should be removed after redeeming all shares"
    );

    let market = get_market(&runner, market_id);
    assert_eq!(market.total_shares, Size::ZERO);
    assert_eq!(get_market_collateral(&runner, market_id), 0);
}

#[test]
fn test_redeem_more_than_owned_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) =
        create_test_market(&mut runner, &user, &user, "Will it rain tomorrow?", collateral, 86_400_000);

    mint_shares(&mut runner, &user, market_id, 100);

    let msg = CallMessage::RedeemShares {
        market_id,
        amount: Size(200),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected redeem_shares to fail when redeeming more than owned"
            );
        }),
    });
}

#[test]
fn test_redeem_shares_nonexistent_market_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;

    let msg = CallMessage::RedeemShares {
        market_id: MarketId(999),
        amount: Size(100),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected redeem_shares to fail for nonexistent market"
            );
        }),
    });
}

#[test]
fn test_redeem_zero_amount_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) =
        create_test_market(&mut runner, &user, &user, "Will it rain tomorrow?", collateral, 86_400_000);
    mint_shares(&mut runner, &user, market_id, 100);

    let msg = CallMessage::RedeemShares {
        market_id,
        amount: Size::ZERO,
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected redeem_shares to fail with zero amount"
            );
        }),
    });
}
