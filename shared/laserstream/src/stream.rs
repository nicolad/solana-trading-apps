use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn, error};
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::prelude::*;

use crate::config::{LaserStreamConfig, CommitmentLevel};
use crate::broadcaster::{WebSocketBroadcaster, StreamMessage, PriceUpdate, AccountUpdate};
use crate::metrics::Metrics;

pub struct StreamManager {
    config: LaserStreamConfig,
    broadcaster: Arc<WebSocketBroadcaster>,
    metrics: Arc<Metrics>,
    reconnect_attempts: usize,
}

impl StreamManager {
    pub fn new(
        config: LaserStreamConfig,
        broadcaster: Arc<WebSocketBroadcaster>,
        metrics: Arc<Metrics>,
    ) -> Self {
        Self {
            config,
            broadcaster,
            metrics,
            reconnect_attempts: 0,
        }
    }
    
    pub async fn start(&mut self) -> Result<()> {
        loop {
            match self.run_stream().await {
                Ok(_) => {
                    info!("Stream completed successfully");
                    break;
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    
                    if !self.config.auto_reconnect {
                        return Err(e);
                    }
                    
                    if let Some(max) = self.config.max_reconnect_attempts {
                        if self.reconnect_attempts >= max {
                            error!("Max reconnect attempts ({}) reached", max);
                            return Err(e);
                        }
                    }
                    
                    self.reconnect_attempts += 1;
                    self.metrics.record_reconnection();
                    
                    let backoff = Duration::from_secs(2u64.pow(self.reconnect_attempts.min(5) as u32));
                    warn!("Reconnecting in {:?} (attempt {})", backoff, self.reconnect_attempts);
                    tokio::time::sleep(backoff).await;
                }
            }
        }
        
        Ok(())
    }
    
    async fn run_stream(&mut self) -> Result<()> {
        // Connect to LaserStream
        let mut client = GeyserGrpcClient::connect(
            &self.config.endpoint,
            Some(&self.config.api_key),
            None,
        )
        .await
        .context("Failed to connect to LaserStream")?;
        
        info!("Connected to LaserStream");
        self.reconnect_attempts = 0; // Reset on successful connection
        
        // Build subscription request
        let mut accounts = HashMap::new();
        
        // Example: Subscribe to Jupiter V6 swaps
        accounts.insert(
            "jupiter".to_string(),
            SubscribeRequestFilterAccounts {
                account: vec!["JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4".to_string()],
                owner: vec![],
                filters: vec![],
            },
        );
        
        let commitment = self.config.commitment_level.to_grpc() as i32;
        
        let request = SubscribeRequest {
            accounts,
            slots: HashMap::from([(
                "slots".to_string(),
                SubscribeRequestFilterSlots {
                    filter_by_commitment: Some(true),
                },
            )]),
            transactions: HashMap::new(),
            blocks: HashMap::new(),
            blocks_meta: HashMap::new(),
            entry: HashMap::new(),
            commitment: Some(commitment),
            accounts_data_slice: vec![],
            ping: None,
        };
        
        // Subscribe
        let (mut subscribe_tx, mut stream) = client
            .subscribe_with_request(Some(request))
            .await
            .context("Failed to subscribe")?;
        
        info!("Subscribed to LaserStream");
        
        // Process stream
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    self.metrics.record_message();
                    
                    if let Err(e) = self.process_message(msg).await {
                        error!("Error processing message: {}", e);
                        self.metrics.record_error();
                    }
                }
                Err(e) => {
                    error!("Stream error: {}", e);
                    return Err(e.into());
                }
            }
        }
        
        Ok(())
    }
    
    async fn process_message(&self, msg: SubscribeUpdate) -> Result<()> {
        use subscribe_update::UpdateOneof;
        
        match msg.update_oneof {
            Some(UpdateOneof::Account(account_update)) => {
                self.handle_account_update(account_update).await?;
            }
            Some(UpdateOneof::Slot(slot_update)) => {
                self.handle_slot_update(slot_update).await?;
            }
            Some(UpdateOneof::Transaction(tx_update)) => {
                self.handle_transaction_update(tx_update).await?;
            }
            Some(UpdateOneof::Ping(_)) => {
                self.broadcaster.broadcast(StreamMessage::Ping).await?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    async fn handle_account_update(&self, update: SubscribeUpdateAccount) -> Result<()> {
        if let Some(account_info) = update.account {
            let update_msg = AccountUpdate {
                pubkey: bs58::encode(&account_info.pubkey).into_string(),
                lamports: account_info.lamports,
                owner: bs58::encode(&account_info.owner).into_string(),
                slot: update.slot,
                timestamp: chrono::Utc::now().timestamp(),
            };
            
            self.broadcaster
                .broadcast(StreamMessage::AccountUpdate(update_msg))
                .await?;
        }
        
        Ok(())
    }
    
    async fn handle_slot_update(&self, update: SubscribeUpdateSlot) -> Result<()> {
        self.broadcaster
            .broadcast(StreamMessage::SlotUpdate {
                slot: update.slot,
                timestamp: chrono::Utc::now().timestamp(),
            })
            .await?;
        
        Ok(())
    }
    
    async fn handle_transaction_update(&self, update: SubscribeUpdateTransaction) -> Result<()> {
        // Parse transaction for price updates (simplified example)
        // In production, you'd parse the actual instruction data
        
        info!("Received transaction at slot {}", update.slot);
        
        // Example: Extract swap data from Jupiter transaction
        // This is a placeholder - actual implementation would parse instruction data
        
        Ok(())
    }
}
