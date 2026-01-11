use crate::config::BotConfig;
use crate::price_tracker::PriceTracker;

pub mod market_maker;

use market_maker::MarketMakerStrategy;

#[derive(Debug, Clone)]
pub enum TradeSignal {
    Buy { amount: u64, reason: String },
    Sell { amount: u64, reason: String },
    PlaceBid { price: f64, size: u64 },
    PlaceAsk { price: f64, size: u64 },
    Hold,
}

pub trait Strategy: Send + Sync {
    fn generate_signal(&self, tracker: &PriceTracker) -> Option<TradeSignal>;
    fn name(&self) -> &str;
}

pub fn create_strategy(config: &BotConfig) -> anyhow::Result<Box<dyn Strategy>> {
    match config.strategy_type.to_lowercase().as_str() {
        "market_maker" => Ok(Box::new(MarketMakerStrategy::new(
            config.spread_bps,
            config.order_size,
            config.max_position_size,
        ))),
        _ => Err(anyhow::anyhow!(
            "Unknown strategy: {}. Only 'market_maker' is supported for limit orders",
            config.strategy_type
        )),
    }
}
