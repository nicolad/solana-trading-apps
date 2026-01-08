use super::{Strategy, TradeSignal};
use crate::price_tracker::PriceTracker;
use tracing::info;

pub struct MomentumStrategy {
    amount: u64,
    min_movement: f64,
    lookback_minutes: usize,
}

impl MomentumStrategy {
    pub fn new(amount: u64, min_movement: f64, lookback_minutes: usize) -> Self {
        Self {
            amount,
            min_movement,
            lookback_minutes,
        }
    }
}

impl Strategy for MomentumStrategy {
    fn generate_signal(&self, tracker: &PriceTracker) -> Option<TradeSignal> {
        // Ensure we have sufficient data
        if !tracker.has_sufficient_data(self.lookback_minutes) {
            return None;
        }
        
        let current_price = tracker.current_price()?;
        let avg_price = tracker.moving_average(self.lookback_minutes)?;
        
        let change = (current_price - avg_price) / avg_price;
        
        info!("Momentum check: current=${:.2}, avg=${:.2}, change={:.2}%",
              current_price, avg_price, change * 100.0);
        
        // Buy if price is rising above threshold
        if change > self.min_movement {
            Some(TradeSignal::Buy {
                amount: self.amount,
                reason: format!(
                    "Momentum: Price up {:.2}% vs {}min avg",
                    change * 100.0,
                    self.lookback_minutes
                ),
            })
        }
        // Sell if price is falling below threshold
        else if change < -self.min_movement {
            Some(TradeSignal::Sell {
                amount: self.amount,
                reason: format!(
                    "Momentum: Price down {:.2}% vs {}min avg",
                    change * 100.0,
                    self.lookback_minutes
                ),
            })
        }
        else {
            Some(TradeSignal::Hold)
        }
    }
    
    fn name(&self) -> &str {
        "Momentum"
    }
}
