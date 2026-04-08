use crate::setup;
use crate::utils;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};

#[test]
fn test_place_limit_bid_order() {
    let (data, mut runner) = setup();

    let next_id = utils::get_next_order_id(&runner);
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

    // Verify the order was created with correct fields
    let order = utils::get_order(&runner, OrderId(next_id));
    assert_eq!(order.id, OrderId(next_id));
    assert_eq!(order.market_id, data.market_id);
    assert_eq!(order.outcome, OutcomeSide::Yes);
    assert_eq!(order.side, Side::Bid);
    // YES Bid => canonical Bid @ same price
    assert_eq!(order.canonical_side, Side::Bid);
    assert_eq!(order.canonical_price, Price(5000));
    assert_eq!(order.original_quantity, 100);
    assert_eq!(order.remaining_quantity, 100);
    assert_eq!(order.owner, data.user1.address());
    assert_eq!(order.order_type, OrderType::Limit);
    assert_eq!(order.status, OrderStatus::Open);
}

#[test]
fn test_place_limit_ask_order() {
    let (data, mut runner) = setup();

    let next_id = utils::get_next_order_id(&runner);
    // BUY NO @ 6000 → canonical Ask @ 4000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(6000),
        50,
        OrderType::Limit,
    );

    let order = utils::get_order(&runner, OrderId(next_id));
    assert_eq!(order.side, Side::Bid);
    assert_eq!(order.outcome, OutcomeSide::No);
    // BUY NO @ 6000 => canonical SELL YES @ (10000-6000) = canonical Ask @ 4000
    assert_eq!(order.canonical_side, Side::Ask);
    assert_eq!(order.canonical_price, Price(4000));
    assert_eq!(order.original_quantity, 50);
    assert_eq!(order.remaining_quantity, 50);
    assert_eq!(order.status, OrderStatus::Open);
}

#[test]
fn test_place_bid_updates_best_bid() {
    let (data, mut runner) = setup();

    // Initially no best bid
    assert!(utils::get_best_bid(&runner, data.market_id).is_none());

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
        utils::get_best_bid(&runner, data.market_id),
        Some(Price(4000))
    );

    // Place a higher bid — best_bid should update
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

    assert_eq!(
        utils::get_best_bid(&runner, data.market_id),
        Some(Price(5000))
    );
}

#[test]
fn test_place_ask_updates_best_ask() {
    let (data, mut runner) = setup();

    assert!(utils::get_best_ask(&runner, data.market_id).is_none());

    // BUY NO @ 3000 → canonical Ask @ 7000
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
        utils::get_best_ask(&runner, data.market_id),
        Some(Price(7000))
    );

    // BUY NO @ 4000 → canonical Ask @ 6000 (better ask)
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

    assert_eq!(
        utils::get_best_ask(&runner, data.market_id),
        Some(Price(6000))
    );
}

#[test]
fn test_place_bid_locks_collateral() {
    let (data, mut runner) = setup();

    assert_eq!(
        utils::get_locked_collateral(&runner, &data.user1, data.market_id),
        0
    );

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

    // Another bid: Price(3000) * 200 / 10000 = 60. Total locked = 50 + 60 = 110
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(3000),
        200,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_locked_collateral(&runner, &data.user1, data.market_id),
        110
    );
}

#[test]
fn test_place_ask_locks_collateral() {
    let (data, mut runner) = setup();

    // BUY NO @ 4000 → canonical Ask @ 6000, locks 4000 * 100 / 10000 = 40
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

    assert_eq!(
        utils::get_locked_collateral(&runner, &data.user1, data.market_id),
        40
    );
}

#[test]
fn test_place_orders_at_multiple_price_levels() {
    let (data, mut runner) = setup();

    // Place bids at 3 different prices
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(3000),
        100,
        OrderType::Limit,
    );
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

    // Price levels should be sorted descending for bids
    let levels = utils::get_price_levels(&runner, data.market_id, Side::Bid);
    assert_eq!(levels, vec![Price(5000), Price(4000), Price(3000)]);
}

#[test]
fn test_order_id_increments_sequentially() {
    let (data, mut runner) = setup();

    let first_id = utils::get_next_order_id(&runner);

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

    assert_eq!(utils::get_next_order_id(&runner), first_id + 1);

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

    assert_eq!(utils::get_next_order_id(&runner), first_id + 2);
}

#[test]
fn test_place_order_adds_to_user_orders() {
    let (data, mut runner) = setup();

    assert!(utils::get_user_orders(&runner, &data.user1).is_empty());

    let id1 = utils::get_next_order_id(&runner);
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

    let user_orders = utils::get_user_orders(&runner, &data.user1);
    assert_eq!(user_orders, vec![OrderId(id1)]);

    let id2 = utils::get_next_order_id(&runner);
    // BUY NO @ 7000 → canonical Ask @ 3000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(7000),
        50,
        OrderType::Limit,
    );

    let user_orders = utils::get_user_orders(&runner, &data.user1);
    assert_eq!(user_orders, vec![OrderId(id1), OrderId(id2)]);
}

#[test]
fn test_place_order_adds_to_bid_price_level() {
    let (data, mut runner) = setup();

    let id = utils::get_next_order_id(&runner);
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

    let bids = utils::get_bids_at_price(&runner, data.market_id, Price(5000));
    assert_eq!(bids, vec![OrderId(id)]);
}

#[test]
fn test_place_order_adds_to_ask_price_level() {
    let (data, mut runner) = setup();

    let id = utils::get_next_order_id(&runner);
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

    let asks = utils::get_asks_at_price(&runner, data.market_id, Price(6000));
    assert_eq!(asks, vec![OrderId(id)]);
}

#[test]
fn test_yes_and_no_orders_share_canonical_book() {
    let (data, mut runner) = setup();

    // BUY YES @ 3000 → canonical bid @ 3000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(3000),
        100,
        OrderType::Limit,
    );

    // BUY NO @ 3000 → canonical SELL YES @ (10000-3000) = canonical ask @ 7000
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(3000),
        100,
        OrderType::Limit,
    );

    // Single canonical book: best bid = 3000, best ask = 7000 (no crossing)
    assert_eq!(
        utils::get_best_bid(&runner, data.market_id),
        Some(Price(3000))
    );
    assert_eq!(
        utils::get_best_ask(&runner, data.market_id),
        Some(Price(7000))
    );
}
