//! Order normalization to canonical YES-space representation.
//!
//! All orders are transformed into YES-space before entering the book:
//! - YES orders keep their side and price unchanged.
//! - NO orders are flipped:
//!   - BUY NO @ p  → SELL YES @ (BASIS - p)
//!   - SELL NO @ p → BUY YES @ (BASIS - p)

use shared_types::{OutcomeSide, Price, Side};

/// Canonical representation of an order in YES-space.
pub struct CanonicalOrder {
    /// Bid = buying YES shares, Ask = selling YES shares.
    pub side: Side,
    /// Price in YES-space basis points (1..=9999).
    pub price: Price,
}

impl CanonicalOrder {
    /// Normalize an incoming order to canonical YES-space.
    ///
    /// Returns the canonical side and price.
    pub fn normalize(outcome: OutcomeSide, side: Side, price: Price) -> Self {
        match outcome {
            OutcomeSide::Yes => Self { side, price },
            OutcomeSide::No => Self {
                side: side.opposite(),
                price: price.complement(),
            },
        }
    }

    /// Calculate collateral required for this canonical order and quantity.
    ///
    /// - Canonical bid (buying YES): locks `canonical_price * qty / BASIS`
    /// - Canonical ask (selling YES): locks `(BASIS - canonical_price) * qty / BASIS`
    ///
    /// The sum of both sides = qty, which is exactly the cost to mint one YES+NO pair per unit.
    pub fn required_collateral(&self, quantity: u64) -> u64 {
        match self.side {
            Side::Bid => self.price.cost(quantity),
            Side::Ask => self.price.complement().cost(quantity),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yes_bid_passthrough() {
        let c = CanonicalOrder::normalize(OutcomeSide::Yes, Side::Bid, Price(6000));
        assert_eq!(c.side, Side::Bid);
        assert_eq!(c.price, Price(6000));
    }

    #[test]
    fn test_yes_ask_passthrough() {
        let c = CanonicalOrder::normalize(OutcomeSide::Yes, Side::Ask, Price(6000));
        assert_eq!(c.side, Side::Ask);
        assert_eq!(c.price, Price(6000));
    }

    #[test]
    fn test_buy_no_becomes_sell_yes() {
        // BUY NO @ 4000 → SELL YES @ 6000
        let c = CanonicalOrder::normalize(OutcomeSide::No, Side::Bid, Price(4000));
        assert_eq!(c.side, Side::Ask);
        assert_eq!(c.price, Price(6000));
    }

    #[test]
    fn test_sell_no_becomes_buy_yes() {
        // SELL NO @ 4000 → BUY YES @ 6000
        let c = CanonicalOrder::normalize(OutcomeSide::No, Side::Ask, Price(4000));
        assert_eq!(c.side, Side::Bid);
        assert_eq!(c.price, Price(6000));
    }

    #[test]
    fn test_complement_sum_is_basis() {
        for p in [1, 100, 2500, 5000, 7500, 9999] {
            let price = Price(p);
            assert_eq!(price.0 + price.complement().0, Price::BASIS);
        }
    }

    #[test]
    fn test_collateral_bid_plus_ask_equals_quantity() {
        let price = Price(6000);
        let qty = 1000;
        let bid_col = CanonicalOrder {
            side: Side::Bid,
            price,
        }
        .required_collateral(qty);
        let ask_col = CanonicalOrder {
            side: Side::Ask,
            price,
        }
        .required_collateral(qty);
        // bid(6000*1000/10000=600) + ask(4000*1000/10000=400) = 1000 = qty
        assert_eq!(bid_col + ask_col, qty);
    }
}
