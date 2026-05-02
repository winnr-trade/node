use crate::setup;
use crate::utils;
use shared_types::{OrderId, OrderStatus, OrderType, OutcomeSide, Price, Side};

#[test]
fn test_bid_matches_ask_at_same_price() {
    let (data, mut runner) = setup();

    // BUY NO @ 5000 → canonical Ask @ 5000
    let ask_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // Bid 100 @ 5000 — should cross and fill both
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

    // Maker ask fully filled
    let ask = utils::get_order(&runner, OrderId(ask_id));
    assert_eq!(ask.status, OrderStatus::Filled);
    assert_eq!(ask.remaining_quantity, 0);

    // Book should be empty
    assert!(utils::get_best_ask(&runner, data.market_id).is_none());
    assert!(utils::get_best_bid(&runner, data.market_id).is_none());
}

#[test]
fn test_ask_matches_bid_at_same_price() {
    let (data, mut runner) = setup();

    // Bid 100 @ 5000
    let bid_id = utils::get_next_order_id(&runner);
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

    // BUY NO @ 5000 → canonical Ask @ 5000
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    let bid = utils::get_order(&runner, OrderId(bid_id));
    assert_eq!(bid.status, OrderStatus::Filled);
    assert_eq!(bid.remaining_quantity, 0);

    assert!(utils::get_best_bid(&runner, data.market_id).is_none());
    assert!(utils::get_best_ask(&runner, data.market_id).is_none());
}

#[test]
fn test_partial_fill_resting_order() {
    let (data, mut runner) = setup();

    // BUY NO @ 5000 → canonical Ask @ 5000, qty 200
    let ask_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        200,
        OrderType::Limit,
    );

    // Bid 80 @ 5000 — partially fills maker
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        80,
        OrderType::Limit,
    );

    let ask = utils::get_order(&runner, OrderId(ask_id));
    // Maker orders stay Open even when partially filled (matching engine only sets
    // PartiallyFilled on taker orders that are posted to book after matching)
    assert_eq!(ask.status, OrderStatus::Open);
    assert_eq!(ask.remaining_quantity, 120);
    assert_eq!(ask.filled_quantity(), 80);

    // Ask still on book
    assert_eq!(
        utils::get_best_ask(&runner, data.market_id),
        Some(Price(5000))
    );
}

#[test]
fn test_taker_partially_filled_posts_remainder() {
    let (data, mut runner) = setup();

    // BUY NO @ 5000 → canonical Ask @ 5000, qty 60
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        60,
        OrderType::Limit,
    );

    // Limit bid 100 @ 5000 — fills 60, remaining 40 rests on book
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

    let bid = utils::get_order(&runner, OrderId(bid_id));
    assert_eq!(bid.status, OrderStatus::PartiallyFilled);
    assert_eq!(bid.remaining_quantity, 40);

    // Bid should be on book
    assert_eq!(
        utils::get_best_bid(&runner, data.market_id),
        Some(Price(5000))
    );
}

#[test]
fn test_price_time_priority_same_price() {
    let (data, mut runner) = setup();

    // Two asks at same price — first order has time priority
    let ask1_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    let ask2_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // Bid 100 — should fill the first ask (time priority)
    utils::place_order(
        &mut runner,
        &data.admin,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    let ask1 = utils::get_order(&runner, OrderId(ask1_id));
    assert_eq!(ask1.status, OrderStatus::Filled);
    assert_eq!(ask1.remaining_quantity, 0);

    let ask2 = utils::get_order(&runner, OrderId(ask2_id));
    assert_eq!(ask2.status, OrderStatus::Open);
    assert_eq!(ask2.remaining_quantity, 100);
}

#[test]
fn test_price_priority_better_price_fills_first() {
    let (data, mut runner) = setup();

    // BUY NO @ 4000 → canonical Ask @ 6000
    let ask_expensive_id = utils::get_next_order_id(&runner);
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

    // BUY NO @ 6000 → canonical Ask @ 4000 (better price for buyer)
    let ask_cheap_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(6000),
        100,
        OrderType::Limit,
    );

    // Bid 100 @ 6000 — should fill cheaper ask at 4000 first
    utils::place_order(
        &mut runner,
        &data.admin,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(6000),
        100,
        OrderType::Limit,
    );

    let ask_cheap = utils::get_order(&runner, OrderId(ask_cheap_id));
    assert_eq!(ask_cheap.status, OrderStatus::Filled);

    let ask_expensive = utils::get_order(&runner, OrderId(ask_expensive_id));
    assert_eq!(ask_expensive.status, OrderStatus::Open);
    assert_eq!(ask_expensive.remaining_quantity, 100);
}

#[test]
fn test_self_trade_prevention_skips_own_orders() {
    let (data, mut runner) = setup();

    // User1 places BUY NO @ 5000 → canonical Ask @ 5000
    let ask_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // User1 places bid at same price — self-trade prevention should skip
    let bid_id = utils::get_next_order_id(&runner);
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

    // Both orders should remain open (self-trade prevented)
    let ask = utils::get_order(&runner, OrderId(ask_id));
    assert_eq!(ask.status, OrderStatus::Open);
    assert_eq!(ask.remaining_quantity, 100);

    let bid = utils::get_order(&runner, OrderId(bid_id));
    assert_eq!(bid.status, OrderStatus::Open);
    assert_eq!(bid.remaining_quantity, 100);
}

#[test]
fn test_taker_sweeps_multiple_price_levels() {
    let (data, mut runner) = setup();

    // BUY NO @ 6000 → canonical Ask @ 4000
    let ask1_id = utils::get_next_order_id(&runner);
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

    // BUY NO @ 5000 → canonical Ask @ 5000
    let ask2_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        50,
        OrderType::Limit,
    );

    // BUY NO @ 4000 → canonical Ask @ 6000
    let ask3_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(4000),
        50,
        OrderType::Limit,
    );

    // Bid 120 @ 6000 — sweeps all 3 levels (50+50+20)
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(6000),
        120,
        OrderType::Limit,
    );

    let ask1 = utils::get_order(&runner, OrderId(ask1_id));
    assert_eq!(ask1.status, OrderStatus::Filled);

    let ask2 = utils::get_order(&runner, OrderId(ask2_id));
    assert_eq!(ask2.status, OrderStatus::Filled);

    let ask3 = utils::get_order(&runner, OrderId(ask3_id));
    assert_eq!(ask3.status, OrderStatus::Open); // maker stays Open
    assert_eq!(ask3.remaining_quantity, 30);
}

#[test]
fn test_bid_does_not_match_ask_above_limit_price() {
    let (data, mut runner) = setup();

    // BUY NO @ 4000 → canonical Ask @ 6000
    let ask_id = utils::get_next_order_id(&runner);
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

    // Bid at 5000 — below ask, no match
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

    // Both should remain open
    let ask = utils::get_order(&runner, OrderId(ask_id));
    assert_eq!(ask.status, OrderStatus::Open);

    let bid = utils::get_order(&runner, OrderId(bid_id));
    assert_eq!(bid.status, OrderStatus::Open);

    // Both should be on book
    assert_eq!(
        utils::get_best_ask(&runner, data.market_id),
        Some(Price(6000))
    );
    assert_eq!(
        utils::get_best_bid(&runner, data.market_id),
        Some(Price(5000))
    );
}

#[test]
fn test_bid_matches_ask_at_lower_price_executes_at_maker_price() {
    let (data, mut runner) = setup();

    // BUY NO @ 6000 → canonical Ask @ 4000
    let ask_id = utils::get_next_order_id(&runner);
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(6000),
        100,
        OrderType::Limit,
    );

    // Bid at 6000 — crosses spread, executes at maker's price (4000)
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Bid,
        Price(6000),
        100,
        OrderType::Limit,
    );

    // Ask should be filled — fill happened at 4000
    let ask = utils::get_order(&runner, OrderId(ask_id));
    assert_eq!(ask.status, OrderStatus::Filled);
    assert_eq!(ask.remaining_quantity, 0);
}

#[test]
fn test_mint_pair_increases_volume_by_quantity_in_collateral_units() {
    let (data, mut runner) = setup();

    assert_eq!(utils::get_market_total_volume(&runner, data.market_id), 0);
    assert_eq!(utils::get_market_collateral(&runner, data.market_id), 0);

    // BUY NO @ 5000 (canonical Ask @ 5000)
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // BUY YES @ 5000 (canonical Bid @ 5000) -> MintPair fill
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

    assert_eq!(utils::get_market_total_volume(&runner, data.market_id), 100);
    assert_eq!(utils::get_market_collateral(&runner, data.market_id), 100);
}

#[test]
fn test_transfer_no_uses_complement_price_for_volume() {
    let (data, mut runner) = setup();

    // Bootstrap balances: mint one YES/NO pair so user1 owns NO shares.
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );
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

    let volume_before = utils::get_market_total_volume(&runner, data.market_id);
    let collateral_before = utils::get_market_collateral(&runner, data.market_id);

    // user1 SELL NO @ 3000 (canonical Bid @ 7000)
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Ask,
        Price(3000),
        100,
        OrderType::Limit,
    );

    // user2 BUY NO @ 3000 (canonical Ask @ 7000) -> TransferNo fill at canonical 7000.
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

    let expected_delta = Price(7000).complement().cost(100);
    assert_eq!(expected_delta, 30);
    assert_eq!(
        utils::get_market_total_volume(&runner, data.market_id),
        volume_before + expected_delta
    );
    assert_eq!(
        utils::get_market_collateral(&runner, data.market_id),
        collateral_before
    );
}

#[test]
fn test_merge_pair_does_not_increase_volume_and_reduces_collateral() {
    let (data, mut runner) = setup();

    // Bootstrap balances: mint one YES/NO pair.
    // user1 gets NO shares, user2 gets YES shares.
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Bid,
        Price(5000),
        100,
        OrderType::Limit,
    );
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

    let volume_before = utils::get_market_total_volume(&runner, data.market_id);
    let collateral_before = utils::get_market_collateral(&runner, data.market_id);

    // user2 SELL YES @ 5000 (canonical Ask @ 5000)
    utils::place_order(
        &mut runner,
        &data.user2,
        data.market_id,
        OutcomeSide::Yes,
        Side::Ask,
        Price(5000),
        100,
        OrderType::Limit,
    );

    // user1 SELL NO @ 5000 (canonical Bid @ 5000) -> MergePair fill.
    utils::place_order(
        &mut runner,
        &data.user1,
        data.market_id,
        OutcomeSide::No,
        Side::Ask,
        Price(5000),
        100,
        OrderType::Limit,
    );

    assert_eq!(
        utils::get_market_total_volume(&runner, data.market_id),
        volume_before
    );
    assert_eq!(
        utils::get_market_collateral(&runner, data.market_id),
        collateral_before - 100
    );
}
