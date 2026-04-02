use crate::setup;
use crate::utils;
use orderbook::OrderbookError;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};

#[test]
fn test_cancel_open_order() {
    let (data, mut runner) = setup();

    let order_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // Verify order is open
    let order = utils::get_order(&runner, OrderId(order_id));
    assert_eq!(order.status, OrderStatus::Open);

    utils::cancel_order(&mut runner, &data.user1, OrderId(order_id));

    // Verify order is cancelled
    let order = utils::get_order(&runner, OrderId(order_id));
    assert_eq!(order.status, OrderStatus::Cancelled);
    assert_eq!(order.remaining_quantity, 0);
}

#[test]
fn test_cancel_partially_filled_order() {
    let (data, mut runner) = setup();

    // Ask 200
    let ask_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        200,
        OrderType::Limit,
    );

    // Bid 50 — partially fills ask
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        50,
        OrderType::Limit,
    );

    let ask = utils::get_order(&runner, OrderId(ask_id));
    // Maker orders stay Open even when partially filled
    assert_eq!(ask.status, OrderStatus::Open);
    assert_eq!(ask.remaining_quantity, 150);

    // Cancel the partially filled (but status=Open) ask
    utils::cancel_order(&mut runner, &data.user1, OrderId(ask_id));

    let ask = utils::get_order(&runner, OrderId(ask_id));
    assert_eq!(ask.status, OrderStatus::Cancelled);
    assert_eq!(ask.remaining_quantity, 0);
}

#[test]
fn test_cancel_wrong_owner_rejected() {
    let (data, mut runner) = setup();

    let order_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // user2 tries to cancel user1's order
    utils::cancel_order_should_fail(
        &mut runner,
        &data.user2,
        OrderId(order_id),
        OrderbookError::NotOrderOwner {
            order_id: OrderId(order_id),
            owner: String::new(),
            sender: String::new(),
        },
    );

    // Order should still be open
    let order = utils::get_order(&runner, OrderId(order_id));
    assert_eq!(order.status, OrderStatus::Open);
}

#[test]
fn test_cancel_nonexistent_order_rejected() {
    let (data, mut runner) = setup();

    utils::cancel_order_should_fail(
        &mut runner,
        &data.user1,
        OrderId(99999),
        OrderbookError::OrderNotFound {
            order_id: OrderId(99999),
        },
    );
}

#[test]
fn test_cancel_bid_unlocks_collateral() {
    let (data, mut runner) = setup();

    let order_id = utils::get_next_order_id(&runner);
    // Price(5000) * 100 / 10000 = 50
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_locked_collateral(&runner, &data.user1, data.market_id),
        50
    );

    utils::cancel_order(&mut runner, &data.user1, OrderId(order_id));

    assert_eq!(
        utils::get_locked_collateral(&runner, &data.user1, data.market_id),
        0
    );
}

#[test]
fn test_cancel_partially_filled_bid_unlocks_remaining_collateral() {
    let (data, mut runner) = setup();

    // Ask 40 @ 5000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        40,
        OrderType::Limit,
    );

    // Bid 100 @ 5000 — fills 40, remaining 60 on book
    let bid_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // Locked = Price(5000).cost(100) = 50 initially locked
    // After matching, remaining_quantity=60. Cancel should unlock cost of 60 shares.
    // Price(5000).cost(60) = 30
    let locked_before = utils::get_locked_collateral(&runner, &data.user2, data.market_id);
    assert_eq!(locked_before, 50); // full amount still locked

    utils::cancel_order(&mut runner, &data.user2, OrderId(bid_id));

    // After cancel: locked was 50, unlock 30 (cost of remaining 60 shares) => 20
    let locked_after = utils::get_locked_collateral(&runner, &data.user2, data.market_id);
    assert_eq!(locked_after, 20);
}

#[test]
fn test_cancel_removes_from_book() {
    let (data, mut runner) = setup();

    let order_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes),
        Some(Price(5000))
    );

    utils::cancel_order(&mut runner, &data.user1, OrderId(order_id));

    // Book should be empty
    assert!(utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes).is_none());
    assert!(
        utils::get_bids_at_price(&runner, data.market_id, OutcomeSide::Yes, Price(5000)).is_empty()
    );
}

#[test]
fn test_cancel_removes_from_user_orders() {
    let (data, mut runner) = setup();

    let order_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_user_orders(&runner, &data.user1),
        vec![OrderId(order_id)]
    );

    utils::cancel_order(&mut runner, &data.user1, OrderId(order_id));

    assert!(utils::get_user_orders(&runner, &data.user1).is_empty());
}

#[test]
fn test_cancel_updates_best_price_to_next_level() {
    let (data, mut runner) = setup();

    // 2 bids at different prices
    let bid1_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(4000),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes),
        Some(Price(5000))
    );

    // Cancel the best bid — next best should be 4000
    utils::cancel_order(&mut runner, &data.user1, OrderId(bid1_id));

    assert_eq!(
        utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes),
        Some(Price(4000))
    );
}
