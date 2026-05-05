use crate::utils::{
    create_test_market, get_market_collateral, get_position, mint_shares,
    resolve_market as resolve_market_helper,
};
use crate::{setup, RT, S};
use market::{CallMessage, MarketModule};
use shared_types::{MarketId, Outcome, Size};
use sov_test_utils::{AsUser, TransactionTestCase};

#[test]
fn test_claim_winnings_yes_outcome() {
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

    mint_shares(&mut runner, &user, market_id, 100);

    // Advance time and resolve
    runner.advance_slots(500);
    resolve_market_helper(&mut runner, &user, market_id, Outcome::Yes);

    // Claim winnings
    let msg = CallMessage::ClaimWinnings { market_id };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "claim_winnings failed: {:?}",
                result.tx_receipt
            );
        }),
    });

    // Position should be removed after claiming
    assert!(
        get_position(&runner, market_id, &user).is_none(),
        "position should be removed after claiming winnings"
    );

    // Market collateral should be reduced by payout
    assert_eq!(get_market_collateral(&runner, market_id), 0);
}

#[test]
fn test_claim_winnings_no_outcome() {
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

    mint_shares(&mut runner, &user, market_id, 100);

    runner.advance_slots(500);
    resolve_market_helper(&mut runner, &user, market_id, Outcome::No);

    let msg = CallMessage::ClaimWinnings { market_id };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "claim_winnings (No outcome) failed: {:?}",
                result.tx_receipt
            );
        }),
    });

    assert!(
        get_position(&runner, market_id, &user).is_none(),
        "position should be removed after claiming"
    );
    assert_eq!(get_market_collateral(&runner, market_id), 0);
}

#[test]
fn test_claim_winnings_unresolved_market_fails() {
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

    mint_shares(&mut runner, &user, market_id, 100);

    let msg = CallMessage::ClaimWinnings { market_id };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected claim_winnings to fail on unresolved market"
            );
        }),
    });

    // Position should still exist
    let position = get_position(&runner, market_id, &user).expect("position should still exist");
    assert_eq!(position.yes_shares, Size(100));
    assert_eq!(position.no_shares, Size(100));
}

#[test]
fn test_claim_winnings_nonexistent_market_fails() {
    let (test_data, mut runner) = setup();
    let user = test_data.user;

    let msg = CallMessage::ClaimWinnings {
        market_id: MarketId(999),
    };

    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected claim_winnings to fail for nonexistent market"
            );
        }),
    });
}

#[test]
fn test_claim_winnings_twice_fails() {
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

    mint_shares(&mut runner, &user, market_id, 100);

    runner.advance_slots(500);
    resolve_market_helper(&mut runner, &user, market_id, Outcome::Yes);

    // First claim should succeed
    let msg1 = CallMessage::ClaimWinnings { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg1),
        assert: Box::new(|result, _state| {
            assert!(
                result.tx_receipt.is_successful(),
                "first claim_winnings should succeed"
            );
        }),
    });

    // Second claim should fail (position already removed)
    let msg2 = CallMessage::ClaimWinnings { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: user.create_plain_message::<RT, MarketModule<S>>(msg2),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected second claim_winnings to fail"
            );
        }),
    });
}

#[test]
fn test_claim_winnings_no_shares_fails() {
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
    mint_shares(&mut runner, &user, market_id, 100);

    runner.advance_slots(500);
    resolve_market_helper(&mut runner, &user, market_id, Outcome::Yes);

    // Admin has no shares, claiming should fail
    let msg = CallMessage::ClaimWinnings { market_id };
    runner.execute_transaction(TransactionTestCase {
        input: admin.create_plain_message::<RT, MarketModule<S>>(msg),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "expected claim_winnings to fail for user with no shares"
            );
        }),
    });
}
