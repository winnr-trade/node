use super::*;

#[test]
fn test_update_price_feed_with_valid_data() {
    let (test_data, mut runner) = setup();

    let feed_id = test_data_feed_id();

    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                update_data: test_update_data(),
            },
        ),
        assert: Box::new(move |result, state| {
            assert!(
                result.tx_receipt.is_successful(),
                "failed to update price feed: {:?}",
                result.tx_receipt
            );

            let pyth = PythModule::<S>::default();
            let stored = pyth
                .price_updates
                .get(
                    &pyth::PriceFeedKey {
                        feed_id: feed_id.clone(),
                        publish_time: TEST_DATA_PUBLISH_TIME,
                    },
                    state,
                )
                .unwrap()
                .expect("price update should be stored");

            assert_eq!(stored.price, 7713314144433);
            assert_eq!(stored.expo, -8);
            assert_eq!(stored.publish_time, TEST_DATA_PUBLISH_TIME);
        }),
    });
}

#[test]
fn test_resubmit_same_update_data() {
    let (test_data, mut runner) = setup();

    // First submission
    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                update_data: test_update_data(),
            },
        ),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful());
        }),
    });

    let feed_id = test_data_feed_id();

    // Second submission of the same data should also succeed (overwrite)
    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                update_data: test_update_data(),
            },
        ),
        assert: Box::new(move |result, state| {
            assert!(result.tx_receipt.is_successful());

            let pyth = PythModule::<S>::default();
            let stored = pyth
                .price_updates
                .get(
                    &pyth::PriceFeedKey {
                        feed_id: feed_id.clone(),
                        publish_time: TEST_DATA_PUBLISH_TIME,
                    },
                    state,
                )
                .unwrap()
                .expect("price update should still exist after resubmission");
            assert_eq!(stored.price, 7713314144433);
        }),
    });
}

#[test]
fn test_nonexistent_feed_returns_none() {
    let (test_data, mut runner) = setup();

    // Submit valid data so there's at least one price stored
    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                update_data: test_update_data(),
            },
        ),
        assert: Box::new(move |_result, state| {
            let pyth = PythModule::<S>::default();
            let result = pyth
                .price_updates
                .get(
                    &pyth::PriceFeedKey {
                        feed_id: test_feed_id(0xFF),
                        publish_time: 9999,
                    },
                    state,
                )
                .unwrap();
            assert!(result.is_none(), "nonexistent feed should return None");
        }),
    });
}
