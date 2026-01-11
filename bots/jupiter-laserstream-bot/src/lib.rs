// Library modules for jupiter-laserstream-bot
// This allows binaries and tests to access shared code

pub mod config;
pub mod executor;
pub mod jupiter_client;
pub mod laserstream_client;
pub mod metrics;
pub mod price_tracker;
pub mod strategies;
pub mod swap_parser;

// Re-export commonly used types for easier testing
pub use config::BotConfig;
pub use executor::TradeExecutor;
pub use jupiter_client::JupiterClient;
pub use laserstream_client::LaserStreamClient;
pub use price_tracker::PriceTracker;
pub use strategies::{Strategy, TradeSignal};
