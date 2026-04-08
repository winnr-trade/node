use crate::setup;
use crate::utils;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};

#[test]
fn test_cancel_all_for_market() {
    let (data, mut runner) = setup();

    // Place 3 orders — 2 Yes, 1 No
    let id1 = utils::get_next_order_id(&runner);
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

    let id2 = utils::get_next_order_id(&runner);
    // BUY NO @ 4000 → canonical Ask @ 6000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(4000),
        100,
        OrderType::Limit,
    );

    let id3 = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(3000),
        100,
        OrderType::Limit,
    );

    // Cancel all orders for this market (no outcome filter)
    utils::cancel_all_orders(&mut runner, &data.user1, data.market_id, None);

    // All orders should be cancelled
    let o1 = utils::get_order(&runner, OrderId(id1));
    assert_eq!(o1.status, OrderStatus::Cancelled);

    let o2 = utils::get_order(&runner, OrderId(id2));
    assert_eq!(o2.status, OrderStatus::Cancelled);

    let o3 = utils::get_order(&runner, OrderId(id3));
    assert_eq!(o3.status, OrderStatus::Cancelled);

    // Book should be empty
    assert!(utils::get_best_bid(&runner, data.market_id).is_none());
    assert!(utils::get_best_ask(&runner, data.market_id).is_none());
}

#[test]
fn test_cancel_all_filters_by_outcome() {
    let (data, mut runner) = setup();

    // Yes bid
    let yes_id = utils::get_next_order_id(&runner);
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

    // No bid
    let no_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(4000),
        100,
        OrderType::Limit,
    );

    // Cancel only Yes orders
    utils::cancel_all_orders(
        &mut runner,
        &data.user1,
        data.market_id,
        Some(OutcomeSide::Yes),
    );

    // Yes order cancelled, No order still open
    let yes = utils::get_order(&runner, OrderId(yes_id));
    assert_eq!(yes.status, OrderStatus::Cancelled);

    let no = utils::get_order(&runner, OrderId(no_id));
    assert_eq!(no.status, OrderStatus::Open);
}

#[test]
fn test_cancel_all_no_orders_succeeds() {
    let (data, mut runner) = setup();

    // No orders placed — cancel_all should succeed (no-op)
    utils::cancel_all_orders(&mut runner, &data.user1, data.market_id, None);
}

#[test]
fn test_cancel_all_does_not_affect_other_users() {
    let (data, mut runner) = setup();

    // user1 places order
    let u1_id = utils::get_next_order_id(&runner);
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

    // user2 places BUY NO @ 4000 → canonical Ask @ 6000
    let u2_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(4000),
        100,
        OrderType::Limit,
    );

    // user1 cancels all
    utils::cancel_all_orders(&mut runner, &data.user1, data.market_id, None);

    let u1_order = utils::get_order(&runner, OrderId(u1_id));
    assert_eq!(u1_order.status, OrderStatus::Cancelled);

    // user2's order should be untouched
    let u2_order = utils::get_order(&runner, OrderId(u2_id));
    assert_eq!(u2_order.status, OrderStatus::Open);
}

#[test]
fn test_cancel_all_unlocks_bid_collateral() {
    let (data, mut runner) = setup();

    // Place 2 bids, locking collateral
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
    // Price(3000) * 100 / 10000 = 30
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(3000),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_locked_collateral(&runner, &data.user1, data.market_id),
        80
    );

    utils::cancel_all_orders(&mut runner, &data.user1, data.market_id, None);

    assert_eq!(
        utils::get_locked_collateral(&runner, &data.user1, data.market_id),
        0
    );
}
