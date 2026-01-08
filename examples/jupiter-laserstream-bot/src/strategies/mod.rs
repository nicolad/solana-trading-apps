use crate::config::BotConfig;
use crate::price_tracker::PriceTracker;

pub mod dca;
pub mod momentum;
pub mod mean_reversion;

use dca::DcaStrategy;
use momentum::MomentumStrategy;
use mean_reversion::MeanReversionStrategy;

#[derive(Debug, Clone)]
pub enum TradeSignal {
    Buy { amount: u64, reason: String },
    Sell { amount: u64, reason: String },
    Hold,
}

pub trait Strategy: Send + Sync {
    fn generate_signal(&self, tracker: &PriceTracker) -> Option<TradeSignal>;
    fn name(&self) -> &str;
}

pub fn create_strategy(config: &BotConfig) -> anyhow::Result<Box<dyn Strategy>> {
    match config.strategy_type.to_lowercase().as_str() {
        "dca" => Ok(Box::new(DcaStrategy::new(config.trade_amount))),
        "momentum" => Ok(Box::new(MomentumStrategy::new(
            config.trade_amount,
            config.min_price_movement,
            config.lookback_minutes,
        ))),
        "mean_reversion" => Ok(Box::new(MeanReversionStrategy::new(
            config.trade_amount,
            config.min_price_movement,
            config.lookback_minutes,
        ))),
        _ => Err(anyhow::anyhow!("Unknown strategy: {}", config.strategy_type)),
    }
}
