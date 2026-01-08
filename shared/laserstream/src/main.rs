use anyhow::{Context, Result};
use dotenv::dotenv;
use std::env;
use tracing::{info, warn, error};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;

mod config;
mod stream;
mod broadcaster;
mod metrics;

use config::LaserStreamConfig;
use stream::StreamManager;
use broadcaster::WebSocketBroadcaster;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load environment variables
    dotenv().ok();
    
    info!("Starting LaserStream Adapter");
    
    // Load configuration
    let config = LaserStreamConfig::from_env()?;
    info!("Loaded configuration for region: {}", config.region);
    
    // Initialize metrics
    let metrics = metrics::init_metrics();
    
    // Create WebSocket broadcaster for forwarding data
    let broadcaster = WebSocketBroadcaster::new(config.broadcast_port).await?;
    info!("WebSocket broadcaster listening on port {}", config.broadcast_port);
    
    // Create stream manager
    let mut stream_manager = StreamManager::new(config.clone(), broadcaster, metrics);
    
    // Start streaming
    info!("Connecting to LaserStream endpoint: {}", config.endpoint);
    stream_manager.start().await?;
    
    Ok(())
}
