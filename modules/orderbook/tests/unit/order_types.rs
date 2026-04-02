use crate::setup;
use crate::utils;
use orderbook::OrderbookError;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};

#[test]
fn test_post_only_posts_when_no_opposing_orders() {
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
        OrderType::PostOnly,
    );

    // Should rest on book since no asks exist
    let order = utils::get_order(&runner, OrderId(next_id));
    assert_eq!(order.status, OrderStatus::Open);
    assert_eq!(order.remaining_quantity, 100);
    assert_eq!(
        utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes),
        Some(Price(5000))
    );
}

#[test]
fn test_post_only_rejected_when_would_match() {
    let (data, mut runner) = setup();

    // Place an ask at 5000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // PostOnly bid at 5000 would cross the spread — should fail
    utils::place_order_should_fail(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::PostOnly,
        OrderbookError::PostOnlyWouldMatch,
    );

    // Ask should still be on book, unchanged
    assert_eq!(
        utils::get_best_ask(&runner, data.market_id, OutcomeSide::Yes),
        Some(Price(5000))
    );
}

#[test]
fn test_post_only_bid_below_best_ask_posts() {
    let (data, mut runner) = setup();

    // Ask at 6000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(6000),
        100,
        OrderType::Limit,
    );

    // PostOnly bid at 5000 (below best ask 6000) — should succeed
    let next_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::PostOnly,
    );

    let order = utils::get_order(&runner, OrderId(next_id));
    assert_eq!(order.status, OrderStatus::Open);
}

#[test]
fn test_ioc_fills_partial_and_remainder_not_posted() {
    let (data, mut runner) = setup();

    // Place ask of 50 at 5000
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        50,
        OrderType::Limit,
    );

    // IOC bid of 100 at 5000 — should fill 50, remainder 50 is NOT posted
    let ioc_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::ImmediateOrCancel,
    );

    // IOC order should NOT be on book (not posted)
    let ioc_order = utils::try_get_order(&runner, OrderId(ioc_id));
    assert!(
        ioc_order.is_none(),
        "IOC order should not be posted to book"
    );

    // No bids should be on the book
    assert!(utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes).is_none());
}

#[test]
fn test_ioc_no_match_nothing_posted() {
    let (data, mut runner) = setup();

    // No opposing orders on book
    let ioc_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::ImmediateOrCancel,
    );

    // Order should not be on book
    let order = utils::try_get_order(&runner, OrderId(ioc_id));
    assert!(order.is_none(), "IOC with no match should not post");
    assert!(utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes).is_none());
}

#[test]
fn test_fok_fills_fully() {
    let (data, mut runner) = setup();

    // Place ask of 100 at 5000
    let ask_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // FOK bid of 100 at 5000 — all 100 available, should fully fill
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::FillOrKill,
    );

    // Maker ask should be fully filled
    let ask_order = utils::get_order(&runner, OrderId(ask_id));
    assert_eq!(ask_order.status, OrderStatus::Filled);
    assert_eq!(ask_order.remaining_quantity, 0);
}

#[test]
fn test_fok_rejected_when_insufficient_liquidity() {
    let (data, mut runner) = setup();

    // Only 50 available
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        50,
        OrderType::Limit,
    );

    // FOK bid of 100 — only 50 available, should fail
    utils::place_order_should_fail(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::FillOrKill,
        OrderbookError::FillOrKillNotFilled {
            requested: 100,
            available: 50,
        },
    );

    // The maker ask should be unchanged (FOK is atomic)
    assert_eq!(
        utils::get_best_ask(&runner, data.market_id, OutcomeSide::Yes),
        Some(Price(5000))
    );
}

#[test]
fn test_fok_rejected_when_no_liquidity() {
    let (data, mut runner) = setup();

    // No opposing orders at all
    utils::place_order_should_fail(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::FillOrKill,
        OrderbookError::FillOrKillNotFilled {
            requested: 100,
            available: 0,
        },
    );
}

#[test]
fn test_market_order_fills_and_does_not_post() {
    let (data, mut runner) = setup();

    // Place ask at 5000
    let ask_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // Market bid — fills against ask, remainder not posted
    let market_id_order = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(9999), // Market orders use max price for bids
        100,
        OrderType::Market,
    );

    // Ask should be fully filled
    let ask = utils::get_order(&runner, OrderId(ask_id));
    assert_eq!(ask.status, OrderStatus::Filled);

    // Market order should not be posted (even if there was unfilled qty)
    let market_order = utils::try_get_order(&runner, OrderId(market_id_order));
    assert!(
        market_order.is_none(),
        "Market order should not rest on book"
    );
}

#[test]
fn test_market_order_partial_fill_not_posted() {
    let (data, mut runner) = setup();

    // Only 30 available
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        30,
        OrderType::Limit,
    );

    // Market bid for 100 — fills 30, remaining 70 discarded
    let market_order_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(9999),
        100,
        OrderType::Market,
    );

    // No bid on book
    assert!(utils::get_best_bid(&runner, data.market_id, OutcomeSide::Yes).is_none());
    assert!(utils::try_get_order(&runner, OrderId(market_order_id)).is_none());
}
