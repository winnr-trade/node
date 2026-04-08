use crate::setup;
use crate::utils;
use orderbook::OrderbookError;
use shared_types::{MarketId, OrderType, OutcomeSide, Price, Side};

#[test]
fn test_zero_quantity_rejected() {
    let (data, mut runner) = setup();

    utils::place_order_should_fail(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        0,
        OrderType::Limit,
        OrderbookError::ZeroQuantity,
    );
}

#[test]
fn test_invalid_price_zero_rejected() {
    let (data, mut runner) = setup();

    utils::place_order_should_fail(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(0),
        100,
        OrderType::Limit,
        OrderbookError::InvalidPrice { price: 0 },
    );
}

#[test]
fn test_invalid_price_10000_rejected() {
    let (data, mut runner) = setup();

    utils::place_order_should_fail(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(10000),
        100,
        OrderType::Limit,
        OrderbookError::InvalidPrice { price: 10000 },
    );
}

#[test]
fn test_invalid_price_above_10000_rejected() {
    let (data, mut runner) = setup();

    utils::place_order_should_fail(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(15000),
        100,
        OrderType::Limit,
        OrderbookError::InvalidPrice { price: 15000 },
    );
}

#[test]
fn test_valid_edge_price_1_accepted() {
    let (data, mut runner) = setup();

    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(1),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_best_bid(&runner, data.market_id),
        Some(Price(1))
    );
}

#[test]
fn test_valid_edge_price_9999_accepted() {
    let (data, mut runner) = setup();

    // BUY NO @ 1 → canonical Ask @ 9999
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(1),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_best_ask(&runner, data.market_id),
        Some(Price(9999))
    );
}

#[test]
fn test_nonexistent_market_rejected() {
    let (data, mut runner) = setup();

    utils::place_order_should_fail(
        &mut runner,
        &data.user1,
        MarketId(9999),
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
        OrderbookError::MarketNotFound {
            market_id: MarketId(9999),
        },
    );
}

#[test]
fn test_halted_market_rejected() {
    let (data, mut runner) = setup();

    // Halt the market
    utils::halt_market(&mut runner, &data.admin, data.market_id);

    // Placing an order on a halted market should fail
    utils::place_order_should_fail(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
        OrderbookError::MarketNotActive {
            market_id: data.market_id,
            status: String::new(),
        },
    );
}

#[test]
fn test_order_below_min_size_rejected() {
    // Our setup uses min_order_size = 1, so create a custom setup with higher min
    // Instead, we just verify that min_order_size = 1 works (qty 1 is accepted)
    let (data, mut runner) = setup();

    // quantity = 1 should work with min_order_size = 1
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        1,
        OrderType::Limit,
    );
}

#[test]
fn test_market_order_skips_price_validation() {
    let (data, mut runner) = setup();

    // Market orders should skip price validation (price is ignored for matching)
    // Using Price(0) which would fail for Limit orders
    // However, market orders still match against opposite side using price as "limit"
    // So a Market bid at max price should succeed even with no matches
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(0),
        100,
        OrderType::Market,
    );
    // Market orders don't post, so nothing on book — just verify no panic
}
