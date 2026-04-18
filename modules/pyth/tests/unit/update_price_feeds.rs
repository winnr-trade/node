use super::*;

#[test]
fn test_update_single_price_feed() {
    let (test_data, mut runner) = setup();

    let feed_id = test_feed_id(0xAA);
    let update = make_price_update(feed_id.clone(), 7713314144433, 1776450157);

    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                updates: vec![update.clone()],
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
                        publish_time: 1776450157,
                    },
                    state,
                )
                .unwrap()
                .expect("price update should be stored");

            assert_eq!(stored.price, 7713314144433);
            assert_eq!(stored.conf, 1000);
            assert_eq!(stored.expo, -8);
            assert_eq!(stored.publish_time, 1776450157);
        }),
    });
}

#[test]
fn test_update_multiple_price_feeds() {
    let (test_data, mut runner) = setup();

    let feed_a = test_feed_id(0x01);
    let feed_b = test_feed_id(0x02);
    let update_a = make_price_update(feed_a.clone(), 100_000, 1000);
    let update_b = make_price_update(feed_b.clone(), 200_000, 1000);

    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                updates: vec![update_a, update_b],
            },
        ),
        assert: Box::new(move |result, state| {
            assert!(result.tx_receipt.is_successful());

            let pyth = PythModule::<S>::default();

            let stored_a = pyth
                .price_updates
                .get(
                    &pyth::PriceFeedKey {
                        feed_id: feed_a.clone(),
                        publish_time: 1000,
                    },
                    state,
                )
                .unwrap()
                .expect("feed A should be stored");
            assert_eq!(stored_a.price, 100_000);

            let stored_b = pyth
                .price_updates
                .get(
                    &pyth::PriceFeedKey {
                        feed_id: feed_b.clone(),
                        publish_time: 1000,
                    },
                    state,
                )
                .unwrap()
                .expect("feed B should be stored");
            assert_eq!(stored_b.price, 200_000);
        }),
    });
}

#[test]
fn test_update_same_feed_different_timestamps() {
    let (test_data, mut runner) = setup();

    let feed_id = test_feed_id(0x10);
    let update1 = make_price_update(feed_id.clone(), 50_000, 1000);
    let update2 = make_price_update(feed_id.clone(), 55_000, 2000);

    // Submit first update
    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                updates: vec![update1],
            },
        ),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful());
        }),
    });

    let feed_id_clone = feed_id.clone();

    // Submit second update at different timestamp
    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                updates: vec![update2],
            },
        ),
        assert: Box::new(move |result, state| {
            assert!(result.tx_receipt.is_successful());

            let pyth = PythModule::<S>::default();

            // Both timestamps should have their own entries
            let at_1000 = pyth
                .price_updates
                .get(
                    &pyth::PriceFeedKey {
                        feed_id: feed_id.clone(),
                        publish_time: 1000,
                    },
                    state,
                )
                .unwrap()
                .expect("update at t=1000 should exist");
            assert_eq!(at_1000.price, 50_000);

            let at_2000 = pyth
                .price_updates
                .get(
                    &pyth::PriceFeedKey {
                        feed_id: feed_id_clone.clone(),
                        publish_time: 2000,
                    },
                    state,
                )
                .unwrap()
                .expect("update at t=2000 should exist");
            assert_eq!(at_2000.price, 55_000);
        }),
    });
}

#[test]
fn test_overwrite_same_feed_same_timestamp() {
    let (test_data, mut runner) = setup();

    let feed_id = test_feed_id(0x20);
    let update1 = make_price_update(feed_id.clone(), 100, 5000);

    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                updates: vec![update1],
            },
        ),
        assert: Box::new(|result, _state| {
            assert!(result.tx_receipt.is_successful());
        }),
    });

    // Overwrite with different price at same timestamp
    let update2 = make_price_update(feed_id.clone(), 999, 5000);

    runner.execute_transaction(TransactionTestCase {
        input: test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds {
                updates: vec![update2],
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
                        publish_time: 5000,
                    },
                    state,
                )
                .unwrap()
                .expect("should exist");
            assert_eq!(stored.price, 999, "should be overwritten with latest value");
        }),
    });
}

#[test]
fn test_nonexistent_feed_returns_none() {
    let (_test_data, mut runner) = setup();

    runner.execute_transaction(TransactionTestCase {
        input: _test_data.user.create_plain_message::<RT, PythModule<S>>(
            CallMessage::UpdatePriceFeeds { updates: vec![] },
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
