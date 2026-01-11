use anyhow::Result;
use dotenv::dotenv;
use std::time::Duration;
use tracing::{error, info, warn};

mod config;
mod defituna_client;
mod executor;
mod solana_rpc_client;

use solana_rpc_client::SolanaRpcClient;
mod price_tracker;
mod strategies;

use config::BotConfig;
use defituna_client::DefiTunaClient;
use executor::TradeExecutor;
use price_tracker::PriceTracker;
use strategies::create_strategy;

/// Bot state to track cooldowns and processed slots
struct BotState {
    cooldown_until: Option<chrono::DateTime<chrono::Utc>>,
    last_slot: Option<u64>,
    daily_trade_count: usize,
    day_start: chrono::DateTime<chrono::Utc>,
}

impl BotState {
    fn new() -> Self {
        Self {
            cooldown_until: None,
            last_slot: None,
            daily_trade_count: 0,
            day_start: chrono::Utc::now(),
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
        self.cooldown_until =
            Some(chrono::Utc::now() + chrono::Duration::minutes(minutes as i64));
        info!("⏰ Cooldown until: {}", self.cooldown_until.unwrap());
    }

    fn should_skip_slot(&self, slot: u64) -> bool {
        self.last_slot.map_or(false, |s| s >= slot)
    }

    fn update_slot(&mut self, slot: u64) {
        self.last_slot = Some(slot);
    }

    fn can_trade(&mut self, max_daily_trades: usize) -> bool {
        let now = chrono::Utc::now();
        if now.signed_duration_since(self.day_start).num_hours() >= 24 {
            // Reset daily counter
            self.daily_trade_count = 0;
            self.day_start = now;
        }
        self.daily_trade_count < max_daily_trades
    }

    fn record_trade(&mut self) {
        self.daily_trade_count += 1;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    info!("🐟 Starting DefiTuna Trading Bot");

    let config = BotConfig::from_env()?;
    info!(
        "Loaded config: strategy={}, pair={}/{}",
        config.strategy_type, config.base_token, config.quote_token
    );

    // Initialize components
    let rpc_client = SolanaRpcClient::new(
        &config.rpc_ws_url,
        &config.defituna_program_id,
    ).await?;
    let mut price_tracker = PriceTracker::new(config.lookback_minutes);
    let strategy = create_strategy(&config)?;
    let defituna_client = DefiTunaClient::new(&config)?;
    let executor = TradeExecutor::new(&config, defituna_client).await?;

    let mut state = BotState::new();
    let poll_interval = Duration::from_secs(config.poll_interval_seconds);

    info!("✅ Bot is running! Monitoring DefiTuna markets...");
    info!("⚙️  Configuration:");
    info!("   Strategy: {}", config.strategy_type);
    info!("   Pair: {}/{}", config.base_token, config.quote_token);
    info!("   Poll interval: {}s", config.poll_interval_seconds);
    info!("   Max slippage: {}bps ({:.2}%)", config.max_slippage_bps, config.max_slippage_bps as f64 / 100.0);
    info!("   Cooldown: {} minutes", config.cooldown_minutes);
    info!("   Max daily trades: {}", config.max_daily_trades);
    info!("🔄 Starting main event loop...");

    let mut loop_count = 0;
    // Main event loop
    loop {
        loop_count += 1;
        if loop_count % 10 == 1 {
            info!("🔄 Event loop iteration #{}", loop_count);
        }
        
        if let Err(e) = process_slot_update(
            &rpc_client,
            &mut price_tracker,
            &strategy,
            &executor,
            &config,
            &mut state,
        )
        .await
        {
            warn!("⚠️  Error processing slot update: {}", e);
        }

        tokio::time::sleep(poll_interval).await;
    }
}

async fn process_slot_update(
    rpc_client: &SolanaRpcClient,
    price_tracker: &mut PriceTracker,
    strategy: &Box<dyn strategies::Strategy>,
    executor: &TradeExecutor,
    config: &BotConfig,
    state: &mut BotState,
) -> Result<()> {
    // Fetch latest slot data
    let slot_data = rpc_client.get_latest_slot().await?;

    // Skip if already processed or no slot data yet
    if slot_data.slot == 0 {
        return Ok(());
    }
    
    if state.should_skip_slot(slot_data.slot) {
        return Ok(());
    }

    info!("📦 Processing slot: {}", slot_data.slot);
    state.update_slot(slot_data.slot);

    // Update price tracker with swap data
    if let Some(swaps) = slot_data.swaps {
        info!("🔄 Processing {} swap(s) from slot", swaps.len());
        let mut relevant_swaps = 0;
        
        for swap in swaps {
            if swap.is_relevant_pair(&config.base_mint, &config.quote_mint) {
                relevant_swaps += 1;
                let price = swap.calculate_price();
                price_tracker.add_price(price);

                let ma_1h = price_tracker.moving_average(60).unwrap_or(price);
                let ma_15m = price_tracker.moving_average(15).unwrap_or(price);
                
                info!(
                    "💰 Slot {} | Price: ${:.4} | MA(15m): ${:.4} | MA(1h): ${:.4}",
                    slot_data.slot,
                    price,
                    ma_15m,
                    ma_1h
                );
                info!("   Swap: {} {} → {} {}", 
                    swap.input_amount, swap.input_mint[..8].to_string(),
                    swap.output_amount, swap.output_mint[..8].to_string());
            }
        }
        
        if relevant_swaps == 0 {
            info!("⏭️  No relevant swaps for {}/{} pair", config.base_token, config.quote_token);
        }
    } else {
        info!("📭 No swap data in this slot");
    }

    // Check cooldown
    if state.is_in_cooldown() {
        if let Some(until) = state.cooldown_until {
            let remaining = until.signed_duration_since(chrono::Utc::now());
            info!("⏰ In cooldown for {} more seconds", remaining.num_seconds());
        }
        return Ok(());
    }

    if state.cooldown_until.is_some() {
        state.clear_cooldown();
    }

    // Check daily trade limit
    if !state.can_trade(config.max_daily_trades) {
        warn!("⚠️  Daily trade limit reached: {}/{} trades today", 
            state.daily_trade_count, config.max_daily_trades);
        return Ok(());
    }
    
    info!("📊 Daily trades: {}/{}", state.daily_trade_count, config.max_daily_trades);

    // Generate trading signal
    info!("🤖 Analyzing market with {} strategy...", config.strategy_type);
    info!("   Price history: {} data points", price_tracker.len());
    
    if let Some(signal) = strategy.generate_signal(price_tracker) {
        // Check if it's a Hold signal
        if matches!(signal, strategies::TradeSignal::Hold) {
            info!("⏸️  Strategy decision: HOLD - no action taken");
            return Ok(());
        }
        
        info!("📊 ═══════════════════════════════════════");
        info!("📊 TRADING SIGNAL GENERATED");
        info!("📊 Type: {:?}", signal);
        info!("📊 ═══════════════════════════════════════");

        // Execute trade
        info!("🚀 Executing trade...");
        match executor.execute_trade(&signal, config).await {
            Ok(signature) => {
                info!("✅ ═══════════════════════════════════════");
                info!("✅ TRADE EXECUTED SUCCESSFULLY");
                info!("✅ Signature: {}", signature);
                info!("✅ ═══════════════════════════════════════");
                state.record_trade();
                state.set_cooldown(config.cooldown_minutes);
            }
            Err(e) => {
                error!("❌ ═══════════════════════════════════════");
                error!("❌ TRADE EXECUTION FAILED");
                error!("❌ Error: {}", e);
                error!("❌ ═══════════════════════════════════════");
            }
        }
    } else {
        info!("⏸️  No trading signal - market conditions not met");
    }

    Ok(())
}
