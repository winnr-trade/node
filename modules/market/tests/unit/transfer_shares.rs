use crate::utils::{create_test_market, get_position, mint_shares};
use crate::{setup, RT, S};
use market::{CallMessage, MarketModule};
use shared_types::Size;
use sov_test_utils::{AsUser, TransactionTestCase};

#[test]
fn test_transfer_shares_success() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let admin = test_data.admin;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will transfer work?",
        collateral,
        86_400_000,
    );

    mint_shares(&mut runner, &user, market_id, 100);

    let msg = CallMessage::TransferShares {
        market_id,
        to: admin.address(),
        yes_amount: Size(40),
        no_amount: Size(30),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "transfer_shares failed: {:?}",
                result.tx_receipt
            );
        }),
    });

    let user_position =
        get_position(&runner, market_id, &user).expect("user position should exist");
    assert_eq!(user_position.yes_shares, Size(60));
    assert_eq!(user_position.no_shares, Size(70));

    let admin_position =
        get_position(&runner, market_id, &admin).expect("admin position should exist");
    assert_eq!(admin_position.yes_shares, Size(40));
    assert_eq!(admin_position.no_shares, Size(30));
}

#[test]
fn test_transfer_shares_insufficient_balance_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let admin = test_data.admin;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will insufficient transfer fail?",
        collateral,
        86_400_000,
    );

    mint_shares(&mut runner, &user, market_id, 50);

    let msg = CallMessage::TransferShares {
        market_id,
        to: admin.address(),
        yes_amount: Size(100),
        no_amount: Size::ZERO,
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected transfer_shares to fail when amount exceeds balance"
            );
        }),
    });
}

#[test]
fn test_transfer_shares_zero_amount_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;
    let admin = test_data.admin;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &user,
        &user,
        "Will zero transfer fail?",
        collateral,
        86_400_000,
    );

    mint_shares(&mut runner, &user, market_id, 50);

    let msg = CallMessage::TransferShares {
        market_id,
        to: admin.address(),
        yes_amount: Size::ZERO,
        no_amount: Size::ZERO,
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected transfer_shares to fail with zero transfer amounts"
            );
        }),
    });
}
