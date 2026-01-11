use super::{Strategy, TradeSignal};
use crate::price_tracker::PriceTracker;
use tracing::info;

pub struct MeanReversionStrategy {
    amount: u64,
    oversold_threshold: f64,
    overbought_threshold: f64,
    lookback_minutes: usize,
}

impl MeanReversionStrategy {
    pub fn new(amount: u64, threshold: f64, lookback_minutes: usize) -> Self {
        Self {
            amount,
            oversold_threshold: -threshold,  // Buy when below mean
            overbought_threshold: threshold, // Sell when above mean
            lookback_minutes,
        }
    }
}

impl Strategy for MeanReversionStrategy {
    fn generate_signal(&self, tracker: &PriceTracker) -> Option<TradeSignal> {
        // Ensure we have sufficient data
        if !tracker.has_sufficient_data(self.lookback_minutes) {
            return None;
        }
        
        let current_price = tracker.current_price()?;
        let avg_price = tracker.moving_average(self.lookback_minutes)?;
        
        let deviation = (current_price - avg_price) / avg_price;
        
        info!("Mean reversion check: current=${:.2}, avg=${:.2}, deviation={:.2}%",
              current_price, avg_price, deviation * 100.0);
        
        // Buy when price is below average (oversold)
        if deviation < self.oversold_threshold {
            Some(TradeSignal::Buy {
                amount: self.amount,
                reason: format!(
                    "Mean Reversion: Price {:.2}% below {}min average (oversold)",
                    deviation.abs() * 100.0,
                    self.lookback_minutes
                ),
            })
        }
        // Sell when price is above average (overbought)
        else if deviation > self.overbought_threshold {
            Some(TradeSignal::Sell {
                amount: self.amount,
                reason: format!(
                    "Mean Reversion: Price {:.2}% above {}min average (overbought)",
                    deviation * 100.0,
                    self.lookback_minutes
                ),
            })
        }
        else {
            Some(TradeSignal::Hold)
        }
    }
    
    fn name(&self) -> &str {
        "Mean Reversion"
    }
}
