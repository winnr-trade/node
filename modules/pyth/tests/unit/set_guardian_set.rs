use super::*;

#[test]
fn test_admin_can_set_guardian_set() {
    let (test_data, mut runner) = setup();

    let new_keys: Vec<[u8; 20]> = vec![[1u8; 20], [2u8; 20], [3u8; 20]];

    runner.execute_transaction(TransactionTestCase {
        input: test_data.admin.create_plain_message::<RT, PythModule<S>>(
            CallMessage::SetGuardianSet {
                keys: new_keys.clone(),
                expiry: 9999,
            },
        ),
        assert: Box::new(move |result, state| {
            assert!(
                result.tx_receipt.is_successful(),
                "admin should be able to set guardian set: {:?}",
                result.tx_receipt
            );

            let pyth = PythModule::<S>::default();
            let gs = pyth
                .guardian_set
                .get(state)
                .unwrap()
                .expect("guardian set should exist");
            assert_eq!(gs.keys.len(), 3);
            assert_eq!(gs.keys[0], [1u8; 20]);
            assert_eq!(gs.keys[1], [2u8; 20]);
            assert_eq!(gs.keys[2], [3u8; 20]);
            assert_eq!(gs.expiry, 9999);
        }),
    });
}

#[test]
fn test_non_admin_cannot_set_guardian_set() {
    let (test_data, mut runner) = setup();

    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::SetGuardianSet {
                keys: vec![[5u8; 20]],
                expiry: 0,
            },
        ),
        assert: Box::new(|result, _state| {
            assert!(
                !result.tx_receipt.is_successful(),
                "non-admin should not be able to set guardian set"
            );
        }),
    });
}

#[test]
fn test_admin_can_update_guardian_set_multiple_times() {
    let (test_data, mut runner) = setup();

    // First update
    runner.execute_transaction(TransactionTestCase {
        input: test_data.admin.create_plain_message::<RT, PythModule<S>>(
            CallMessage::SetGuardianSet {
                keys: vec![[1u8; 20]],
                expiry: 100,
            },
        ),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful());
        }),
    });

    // Second update replaces the first
    runner.execute_transaction(TransactionTestCase {
        input: test_data.admin.create_plain_message::<RT, PythModule<S>>(
            CallMessage::SetGuardianSet {
                keys: vec![[9u8; 20], [8u8; 20]],
                expiry: 200,
            },
        ),
        assert: Box::new(move |result, state| {
            assert!(result.tx_receipt.is_successful());

            let pyth = PythModule::<S>::default();
            let gs = pyth
                .guardian_set
                .get(state)
                .unwrap()
                .expect("guardian set should exist");
            assert_eq!(gs.keys.len(), 2);
            assert_eq!(gs.keys[0], [9u8; 20]);
            assert_eq!(gs.expiry, 200);
        }),
    });
}

#[test]
fn test_set_empty_guardian_set() {
    let (test_data, mut runner) = setup();

    runner.execute_transaction(TransactionTestCase {
        input: test_data.admin.create_plain_message::<RT, PythModule<S>>(
            CallMessage::SetGuardianSet {
                keys: vec![],
                expiry: 0,
            },
        ),
        assert: Box::new(move |result, state| {
            assert!(result.tx_receipt.is_successful());

            let pyth = PythModule::<S>::default();
            let gs = pyth
                .guardian_set
                .get(state)
                .unwrap()
                .expect("guardian set should exist");
            assert_eq!(gs.keys.len(), 0);
            assert_eq!(gs.expiry, 0);
        }),
    });
}
