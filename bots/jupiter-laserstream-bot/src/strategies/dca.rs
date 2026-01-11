use super::{Strategy, TradeSignal};
use crate::price_tracker::PriceTracker;

pub struct DcaStrategy {
    amount: u64,
}

impl DcaStrategy {
    pub fn new(amount: u64) -> Self {
        Self { amount }
    }
}

impl Strategy for DcaStrategy {
    fn generate_signal(&self, tracker: &PriceTracker) -> Option<TradeSignal> {
        // DCA always buys if price data is available
        if tracker.current_price().is_some() {
            Some(TradeSignal::Buy {
                amount: self.amount,
                reason: "DCA: Regular scheduled buy".to_string(),
            })
        } else {
            None
        }
    }
    
    fn name(&self) -> &str {
        "DCA (Dollar Cost Average)"
    }
}
