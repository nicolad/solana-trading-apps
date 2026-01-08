use anyhow::Result;
use dotenv::dotenv;
use tracing::{info, warn, error};
use std::time::Duration;

mod config;
mod price_tracker;
mod strategies;
mod executor;
mod metrics;

use config::BotConfig;
use price_tracker::PriceTracker;
use strategies::{Strategy, create_strategy};
use executor::TradeExecutor;
use websocket_utils::client::WebSocketClient;
use websocket_utils::types::WsMessage;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load environment variables
    dotenv().ok();
    
    info!("🚀 Starting Jupiter LaserStream Trading Bot");
    
    // Load configuration
    let config = BotConfig::from_env()?;
    info!("Loaded config: strategy={}, pair={}/{}", 
          config.strategy_type, config.base_token, config.quote_token);
    
    // Initialize components
    let mut price_tracker = PriceTracker::new(config.lookback_minutes);
    let strategy = create_strategy(&config)?;
    let executor = TradeExecutor::new(&config).await?;
    let metrics = metrics::init_metrics();
    
    // Connect to LaserStream adapter via WebSocket
    info!("Connecting to LaserStream adapter at {}", config.adapter_ws_url);
    let mut ws_client = WebSocketClient::builder()
        .url(&config.adapter_ws_url)
        .auto_reconnect(true)
        .build()
        .await?;
    
    // Subscribe to price updates
    let channel = format!("jupiter:{}-{}", config.base_token, config.quote_token);
    info!("Subscribing to channel: {}", channel);
    
    ws_client.send(&WsMessage::Subscribe {
        channels: vec![channel],
    }).await?;
    
    info!("✅ Bot is running! Monitoring prices...");
    
    // Main event loop
    let mut cooldown_until: Option<chrono::DateTime<chrono::Utc>> = None;
    
    while let Some(msg) = ws_client.receive::<WsMessage>().await? {
        match msg {
            WsMessage::PriceUpdate { 
                input_mint,
                output_mint,
                price,
                volume,
                timestamp 
            } => {
                // Update price tracker
                price_tracker.add_price(price, volume as f64, timestamp);
                
                metrics.record_price_update();
                
                // Log periodic updates
                if price_tracker.update_count() % 100 == 0 {
                    info!("Price: ${:.2} | 1h avg: ${:.2} | Updates: {}", 
                          price, 
                          price_tracker.moving_average(60).unwrap_or(price),
                          price_tracker.update_count());
                }
                
                // Check if we're in cooldown
                if let Some(until) = cooldown_until {
                    if chrono::Utc::now() < until {
                        continue;
                    } else {
                        info!("✅ Cooldown period ended");
                        cooldown_until = None;
                    }
                }
                
                // Generate trading signal
                if let Some(signal) = strategy.generate_signal(&price_tracker) {
                    info!("📊 Signal: {:?}", signal);
                    
                    // Execute trade
                    match executor.execute_trade(&signal, &config).await {
                        Ok(signature) => {
                            info!("✅ Trade executed: {}", signature);
                            metrics.record_trade(true);
                            
                            // Set cooldown
                            cooldown_until = Some(
                                chrono::Utc::now() + 
                                chrono::Duration::minutes(config.cooldown_minutes as i64)
                            );
                            info!("⏰ Cooldown until: {}", cooldown_until.unwrap());
                        }
                        Err(e) => {
                            error!("❌ Trade failed: {}", e);
                            metrics.record_trade(false);
                        }
                    }
                }
            }
            
            WsMessage::Ping => {
                ws_client.send(&WsMessage::Pong).await?;
            }
            
            _ => {}
        }
    }
    
    Ok(())
}
