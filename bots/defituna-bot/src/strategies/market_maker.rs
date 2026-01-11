use super::{Strategy, TradeSignal};
use crate::price_tracker::PriceTracker;
use tracing::info;

/// Market maker strategy that places both bid and ask orders
/// around the current market price with a defined spread
pub struct MarketMakerStrategy {
    spread_bps: u16,
    order_size: u64,
    max_position_size: u64,
    current_position: u64,
}

impl MarketMakerStrategy {  
    pub fn new(spread_bps: u16, order_size: u64, max_position_size: u64) -> Self {
        Self {
            spread_bps,
            order_size,
            max_position_size,
            current_position: 0,
        }
    }

    fn calculate_bid_ask_prices(&self, mid_price: f64) -> (f64, f64) {
        let spread_factor = self.spread_bps as f64 / 10000.0;
        let half_spread = mid_price * spread_factor / 2.0;

        let bid_price = mid_price - half_spread;
        let ask_price = mid_price + half_spread;

        (bid_price, ask_price)
    }

    fn can_place_bid(&self) -> bool {
        self.current_position < self.max_position_size
    }

    fn can_place_ask(&self) -> bool {
        self.current_position > 0
    }
}

impl Strategy for MarketMakerStrategy {
    fn generate_signal(&self, tracker: &PriceTracker) -> Option<TradeSignal> {
        let current_price = tracker.current_price()?;
        let (bid_price, ask_price) = self.calculate_bid_ask_prices(current_price);

        info!(
            "Market making: mid=${:.4}, bid=${:.4}, ask=${:.4}, spread={}bps",
            current_price, bid_price, ask_price, self.spread_bps
        );

        // Simple market making: place both orders if we can
        if self.can_place_bid() {
            return Some(TradeSignal::PlaceBid {
                price: bid_price,
                size: self.order_size,
            });
        }

        if self.can_place_ask() {
            return Some(TradeSignal::PlaceAsk {
                price: ask_price,
                size: self.order_size,
            });
        }

        Some(TradeSignal::Hold)
    }

    fn name(&self) -> &str {
        "Market Maker"
    }
}
