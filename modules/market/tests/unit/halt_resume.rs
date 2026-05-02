use crate::utils::{create_test_market, get_market, mint_shares};
use crate::{setup, RT, S};
use market::{CallMessage, MarketModule, MarketStatus};
use shared_types::MarketId;
use sov_test_utils::{AsUser, TransactionTestCase};

#[test]
fn test_halt_market_by_admin() {
    let (test_data, mut runner) = setup();
    let admin = test_data.admin;
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &admin,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    let msg = CallMessage::HaltMarket { market_id };

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

    // Verify market status is Halted
    let market = get_market(&runner, market_id);
    assert_eq!(market.status(), MarketStatus::Halted);
}

#[test]
fn test_halt_market_unauthorized_fails() {
    let (test_data, mut runner) = setup();
    let admin = test_data.admin;
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &admin,
        &admin,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    // User (not admin or resolver) tries to halt
    let msg = CallMessage::HaltMarket { market_id };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected halt_market to fail for unauthorized user"
            );
        }),
    });

    // Market should still be Active
    let market = get_market(&runner, market_id);
    assert_eq!(market.status(), MarketStatus::Active);
}

#[test]
fn test_resume_market_after_halt() {
    let (test_data, mut runner) = setup();
    let admin = test_data.admin;
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &admin,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    // Halt
    let halt_msg = CallMessage::HaltMarket { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(halt_msg),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful(), "halt failed");
        }),
    });

    assert_eq!(
        get_market(&runner, market_id).status(),
        MarketStatus::Halted
    );

    // Resume
    let resume_msg = CallMessage::ResumeMarket { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(resume_msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "resume_market failed: {:?}",
                result.tx_receipt
            );
        }),
    });

    // Verify market is Active again
    assert_eq!(
        get_market(&runner, market_id).status(),
        MarketStatus::Active
    );
}

#[test]
fn test_resume_market_unauthorized_fails() {
    let (test_data, mut runner) = setup();
    let admin = test_data.admin;
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &admin,
        &admin,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    // Admin halts
    let halt_msg = CallMessage::HaltMarket { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(halt_msg),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful(), "halt failed");
        }),
    });

    // User (not admin) tries to resume
    let resume_msg = CallMessage::ResumeMarket { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(resume_msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected resume_market to fail for unauthorized user"
            );
        }),
    });

    // Market should still be Halted
    assert_eq!(
        get_market(&runner, market_id).status(),
        MarketStatus::Halted
    );
}

#[test]
fn test_mint_on_halted_market_fails() {
    let (test_data, mut runner) = setup();
    let admin = test_data.admin;
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &admin,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    let halt_msg = CallMessage::HaltMarket { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(halt_msg),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful(), "halt failed");
        }),
    });

    // Try to mint on halted market
    let mint_msg = CallMessage::MintShares {
        market_id,
        amount: 100,
    };
    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(mint_msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected mint_shares to fail on halted market"
            );
        }),
    });

    // Market totals should be unchanged
    let market = get_market(&runner, market_id);
    assert_eq!(market.total_shares, 0);
}

#[test]
fn test_mint_after_resume_succeeds() {
    let (test_data, mut runner) = setup();
    let admin = test_data.admin;
    let user = test_data.user;
    let collateral = test_data.collateral_token_id;

    let (market_id, _) = create_test_market(
        &mut runner,
        &admin,
        &user,
        "Will it rain tomorrow?",
        collateral,
        86_400_000,
    );

    // Halt
    let halt_msg = CallMessage::HaltMarket { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(halt_msg),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful(), "halt failed");
        }),
    });

    // Resume
    let resume_msg = CallMessage::ResumeMarket { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(resume_msg),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful(), "resume failed");
        }),
    });

    // Mint should work after resume
    mint_shares(&mut runner, &user, market_id, 100);

    // Verify shares were minted
    let market = get_market(&runner, market_id);
    assert_eq!(market.total_shares, 100);
}

#[test]
fn test_halt_nonexistent_market_fails() {
    let (test_data, mut runner) = setup();
    let admin = test_data.admin;

    let msg = CallMessage::HaltMarket {
        market_id: MarketId(999),
    };

    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected halt_market to fail for nonexistent market"
            );
        }),
    });
}
