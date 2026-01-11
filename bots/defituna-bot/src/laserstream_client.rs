use anyhow::{Context, Result};
use futures::StreamExt;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_account_decoder::UiAccountEncoding;
use solana_sdk::commitment_config::CommitmentConfig;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, debug};

#[derive(Debug, Clone)]
pub struct SlotData {
    pub slot: u64,
    pub swaps: Option<Vec<SwapData>>,
    pub limit_orders: Option<Vec<LimitOrderUpdate>>,
}

#[derive(Debug, Clone)]
pub struct SwapData {
    pub input_mint: String,
    pub output_mint: String,
    pub input_amount: u64,
    pub output_amount: u64,
}

#[derive(Debug, Clone)]
pub struct LimitOrderUpdate {
    pub pubkey: String,
    pub owner: String,
    pub input_mint: String,
    pub output_mint: String,
    pub price: f64,
    pub size: u64,
    pub filled: u64,
    pub is_active: bool,
}

impl SwapData {
    pub fn is_relevant_pair(&self, base_mint: &str, quote_mint: &str) -> bool {
        (self.input_mint == base_mint && self.output_mint == quote_mint)
            || (self.input_mint == quote_mint && self.output_mint == base_mint)
    }

    pub fn calculate_price(&self) -> f64 {
        // Simplified price calculation
        // In production, adjust for token decimals
        self.output_amount as f64 / self.input_amount as f64
    }
}

pub struct SolanaRpcClient {
    latest_data: Arc<Mutex<Option<SlotData>>>,
    latest_slot: Arc<Mutex<u64>>,
    defituna_program: String,
}

impl SolanaRpcClient {
    pub async fn new(ws_endpoint: &str, defituna_program: &str) -> Result<Self> {
        info!("📡 Connecting to Solana RPC WebSocket for DeFiTuna limit order monitoring...");
        info!("   Endpoint: {}", ws_endpoint);
        info!("   Program: {}", defituna_program);

        let latest_data: Arc<Mutex<Option<SlotData>>> = Arc::new(Mutex::new(None));
        let latest_slot = Arc::new(Mutex::new(0));
        
        let data_clone = latest_data.clone();
        let slot_clone = latest_slot.clone();

        // Start LaserStream subscription in background
        tokio::spawn(async move {
            let (stream, _handle) = subscribe(config, request);
            tokio::pin!(stream);

            info!("✅ LaserStream connected, streaming DeFiTuna limit orders...");

            while let Some(result) = stream.next().await {
                match result {
                    Ok(update) => {
                        // Process updates using update_oneof enum
                        if let Some(update_oneof) = &update.update_oneof {
                            match update_oneof {
                                helius_laserstream::grpc::subscribe_update::UpdateOneof::Account(account_update) => {
                                    let slot = account_update.slot;
                                    *slot_clone.lock().await = slot;
                                    
                                    if let Some(account_info) = &account_update.account {
                                        debug!("📦 Limit order account update in slot {}: pubkey={:?}, owner={:?}, lamports={}", 
                                            slot, 
                                            bs58::encode(&account_info.pubkey).into_string(),
                                            bs58::encode(&account_info.owner).into_string(),
                                            account_info.lamports);
                                        
                                        // TODO: Decode limit order account data
                                        // Parse the account data to extract limit order details
                                        // For now, create placeholder
                                        let limit_order = LimitOrderUpdate {
                                            pubkey: bs58::encode(&account_info.pubkey).into_string(),
                                            owner: bs58::encode(&account_info.owner).into_string(),
                                            input_mint: String::new(), // TODO: decode from account data
                                            output_mint: String::new(), // TODO: decode from account data
                                            price: 0.0,
                                            size: 0,
                                            filled: 0,
                                            is_active: true,
                                        };
                                        
                                        let mut data = data_clone.lock().await;
                                        if let Some(slot_data) = data.as_mut() {
                                            if let Some(ref mut orders) = slot_data.limit_orders {
                                                orders.push(limit_order);
                                            } else {
                                                slot_data.limit_orders = Some(vec![limit_order]);
                                            }
                                        } else {
                                            *data = Some(SlotData {
                                                slot,
                                                swaps: None,
                                                limit_orders: Some(vec![limit_order]),
                                            });
                                        }
                                    }
                                }
                                helius_laserstream::grpc::subscribe_update::UpdateOneof::Transaction(tx_update) => {
                                    let slot = tx_update.slot;
                                    *slot_clone.lock().await = slot;
                                    
                                    if let Some(tx) = &tx_update.transaction {
                                        debug!("📊 DeFiTuna transaction in slot {}: sig={:?}", 
                                            slot, tx.signature);
                                        
                                        // Parse swap/limit order fill data from transaction
                                        // For now, create dummy swap data - in production, decode instruction data
                                        let slot_data = SlotData {
                                            slot,
                                            swaps: Some(vec![]), // TODO: Parse actual swap data from tx
                                            limit_orders: None,
                                        };
                                        
                                        *data_clone.lock().await = Some(slot_data);
                                    }
                                }
                                helius_laserstream::grpc::subscribe_update::UpdateOneof::Slot(slot_update) => {
                                    debug!("Slot: {}", slot_update.slot);
                                    *slot_clone.lock().await = slot_update.slot;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(e) => {
                        warn!("LaserStream error: {}, will auto-reconnect...", e);
                    }
                }
            }
        });

        Ok(Self {
            latest_data,
            latest_slot,
            defituna_program: defituna_program.to_string(),
        })
    }

    pub async fn get_latest_slot(&self) -> Result<SlotData> {
        // Get LaserStream data or return empty if not available yet
        if let Some(data) = self.latest_data.lock().await.clone() {
            return Ok(data);
        }
        
        // Return empty data if LaserStream hasn't received anything yet
        let slot = *self.latest_slot.lock().await;
        Ok(SlotData {
            slot,
            swaps: None,
            limit_orders: None,
        })
    }
}
