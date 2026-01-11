use anyhow::Result;
use dotenv::dotenv;
use std::time::Duration;
use tracing::{error, info, warn};

mod config;
mod executor;
mod jupiter_client;
mod laserstream_client;
mod metrics;
mod price_tracker;
mod strategies;
mod swap_parser;

use config::BotConfig;
use executor::TradeExecutor;
use jupiter_client::JupiterClient;
use laserstream_client::LaserStreamClient;
use price_tracker::PriceTracker;
use strategies::create_strategy;
use swap_parser::get_token_decimals;

/// Bot state to track cooldowns and processed slots
struct BotState {
    cooldown_until: Option<chrono::DateTime<chrono::Utc>>,
    last_slot: Option<u64>,
}

impl BotState {
    fn new() -> Self {
        Self {
            cooldown_until: None,
            last_slot: None,
        }
    }

    fn is_in_cooldown(&self) -> bool {
        self.cooldown_until
            .map_or(false, |until| chrono::Utc::now() < until)
    }

    fn clear_cooldown(&mut self) {
        info!("✅ Cooldown period ended");
        self.cooldown_until = None;
    }

    fn set_cooldown(&mut self, minutes: u64) {
        self.cooldown_until = Some(
            chrono::Utc::now() + chrono::Duration::minutes(minutes as i64)
        );
        info!("⏰ Cooldown until: {}", self.cooldown_until.unwrap());
    }

    fn should_skip_slot(&self, slot: u64) -> bool {
        self.last_slot.map_or(false, |s| s >= slot)
    }

    fn update_slot(&mut self, slot: u64) {
        self.last_slot = Some(slot);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    info!("🚀 Starting Jupiter LaserStream Trading Bot");

    let config = BotConfig::from_env()?;
    info!(
        "Loaded config: strategy={}, pair={}/{}",
        config.strategy_type, config.base_token, config.quote_token
    );

    // Initialize all components
    let (laserstream, mut price_tracker, strategy, executor, metrics, jupiter_client, quote_decimals) =
        initialize_components(&config).await?;

    let mut state = BotState::new();
    let poll_interval = Duration::from_secs(config.poll_interval_seconds);

    info!("✅ Bot is running! Monitoring slot updates...");

    // Main event loop
    loop {
        if let Err(e) = process_slot_update(
            &laserstream,
            &mut price_tracker,
            &strategy,
            &executor,
            &metrics,
            &jupiter_client,
            &config,
            &mut state,
            quote_decimals,
        )
        .await
        {
            warn!("Error processing slot update: {}", e);
        }

        tokio::time::sleep(poll_interval).await;
    }
}

async fn initialize_components(
    config: &BotConfig,
) -> Result<(
    LaserStreamClient,
    PriceTracker,
    Box<dyn strategies::Strategy>,
    TradeExecutor,
    std::sync::Arc<metrics::Metrics>,
    JupiterClient,
    u8,
)> {
    let mut price_tracker = PriceTracker::new(config.lookback_minutes);
    let strategy = create_strategy(&config)?;
    let executor = TradeExecutor::new(&config).await?;
    let metrics = metrics::init_metrics();
    let jupiter_client = JupiterClient::new();

    let quote_decimals = get_token_decimals(&config.quote_mint);

    // Connect and verify LaserStream container
    info!(
        "Connecting to LaserStream container at {}",
        config.laserstream_url
    );
    let laserstream = LaserStreamClient::new(&config.laserstream_url);

    if !laserstream.health_check().await? {
        error!("LaserStream container is not healthy");
        return Err(anyhow::anyhow!("LaserStream container health check failed"));
    }
    info!("✅ LaserStream container is healthy");

    info!("Starting LaserStream subscription...");
    laserstream.start().await?;
    info!("✅ LaserStream subscription started");

    Ok((
        laserstream,
        price_tracker,
        strategy,
        executor,
        metrics,
        jupiter_client,
        quote_decimals,
    ))
}

async fn process_slot_update(
    laserstream: &LaserStreamClient,
    price_tracker: &mut PriceTracker,
    strategy: &Box<dyn strategies::Strategy>,
    executor: &TradeExecutor,
    metrics: &std::sync::Arc<metrics::Metrics>,
    jupiter_client: &JupiterClient,
    config: &BotConfig,
    state: &mut BotState,
    quote_decimals: u8,
) -> Result<()> {
    let update = match laserstream.get_latest().await? {
        Some(update) => update,
        None => {
            // Log periodically that we're polling but no data yet
            static mut POLL_COUNT: u64 = 0;
            unsafe {
                POLL_COUNT += 1;
                if POLL_COUNT % 30 == 0 {
                    info!("⏱️  Polling LaserStream container... (no data yet - needs valid HELIUS_API_KEY)");
                }
            }
            return Ok(());
        }
    };

    // Skip if we've already processed this slot
    if state.should_skip_slot(update.slot) {
        return Ok(());
    }

    state.update_slot(update.slot);
    info!("📊 New slot: {} at {}", update.slot, update.timestamp);

    // Fetch and update price data
    update_price_data(
        jupiter_client,
        price_tracker,
        metrics,
        config,
        quote_decimals,
    )
    .await;

    // Check cooldown status
    if state.is_in_cooldown() {
        return Ok(());
    } else if state.cooldown_until.is_some() {
        state.clear_cooldown();
    }

    // Generate and execute trading signals
    if let Some(signal) = strategy.generate_signal(&price_tracker) {
        info!("📊 Signal: {:?}", signal);

        match executor.execute_trade(&signal, &config).await {
            Ok(signature) => {
                info!("✅ Trade executed: {}", signature);
                metrics.record_trade(true);
                state.set_cooldown(config.cooldown_minutes);
            }
            Err(e) => {
                error!("❌ Trade failed: {}", e);
                metrics.record_trade(false);
            }
        }
    }

    Ok(())
}

async fn update_price_data(
    jupiter_client: &JupiterClient,
    price_tracker: &mut PriceTracker,
    metrics: &std::sync::Arc<metrics::Metrics>,
    config: &BotConfig,
    quote_decimals: u8,
) {
    let price = match jupiter_client
        .get_price(&config.base_mint, &config.quote_mint)
        .await
    {
        Ok(price) => price,
        Err(e) => {
            error!("❌ Failed to fetch price from Jupiter: {}", e);
            return;
        }
    };

    let timestamp = chrono::Utc::now().timestamp();
    let quote_amount = 100_000_000; // 100 USDC (6 decimals)

    // Try to get volume estimate from quote
    let volume = match jupiter_client
        .get_quote(&config.quote_mint, &config.base_mint, quote_amount, 50)
        .await
    {
        Ok(quote) => {
            let vol = quote_amount as f64 / 10_f64.powi(quote_decimals as i32);

            // Log periodic updates with price impact
            if price_tracker.update_count() % 10 == 0 {
                info!(
                    "💰 Price: ${:.4} | 1h avg: ${:.4} | Updates: {} | Impact: {}%",
                    price,
                    price_tracker.moving_average(60).unwrap_or(price),
                    price_tracker.update_count(),
                    quote.price_impact_pct
                );
            }
            vol
        }
        Err(e) => {
            warn!("Failed to get quote for volume: {}", e);
            100.0 // Default volume
        }
    };

    price_tracker.add_price(price, volume, timestamp);
    metrics.record_price_update();
}
