use anyhow::Result;
use futures::StreamExt;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use anchor_client::solana_account_decoder::UiAccountEncoding;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::account::Account;
use std::sync::Arc;
use std::str::FromStr;
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

impl SwapData {
    pub fn is_relevant_pair(&self, base_mint: &str, quote_mint: &str) -> bool {
        (self.input_mint == base_mint && self.output_mint == quote_mint) ||
        (self.input_mint == quote_mint && self.output_mint == base_mint)
    }

    pub fn calculate_price(&self) -> f64 {
        if self.output_amount == 0 {
            return 0.0;
        }
        self.input_amount as f64 / self.output_amount as f64
    }
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
        
        // Start WebSocket subscription in background
        let latest_data_clone = latest_data.clone();
        let latest_slot_clone = latest_slot.clone();
        let ws_endpoint = ws_endpoint.to_string();
        let program_id_str = defituna_program.to_string();
        
        tokio::spawn(async move {
            loop {
                match PubsubClient::new(&ws_endpoint).await {
                    Ok(pubsub_client) => {
                        info!("✅ Connected to Solana RPC WebSocket");
                        
                        let program_id = match Pubkey::from_str(&program_id_str) {
                            Ok(id) => id,
                            Err(e) => {
                                warn!("Invalid program ID: {}", e);
                                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                                continue;
                            }
                        };
                        
                        // Subscribe to program accounts (limit orders owned by DeFiTuna)
                        let config = RpcProgramAccountsConfig {
                            filters: None,
                            account_config: RpcAccountInfoConfig {
                                encoding: Some(UiAccountEncoding::Base64),
                                data_slice: None,
                                commitment: Some(CommitmentConfig::confirmed()),
                                min_context_slot: None,
                            },
                            with_context: Some(true),
                            sort_results: None,
                        };
                        
                        match pubsub_client.program_subscribe(&program_id, Some(config)).await {
                            Ok((mut stream, _unsub)) => {
                                info!("📡 Subscribed to DeFiTuna program accounts");
                                
                                while let Some(response) = stream.next().await {
                                    let slot = response.context.slot;
                                    let pubkey = response.value.pubkey;
                                    
                                    debug!("Account update: {} @ slot {}", pubkey, slot);
                                    
                                    // Update slot
                                    *latest_slot_clone.lock().await = slot;
                                    
                                    // Parse account data
                                    if let Some(account) = response.value.account.decode::<Account>() {
                                        // TODO: Decode limit order account data properly
                                        let limit_order = LimitOrderUpdate {
                                            pubkey: pubkey.clone(),
                                            owner: account.owner.to_string(),
                                            input_mint: "SOL".to_string(),
                                            output_mint: "USDC".to_string(),
                                            price: 0.0,
                                            size: 0,
                                            filled: 0,
                                            is_active: true,
                                        };
                                        
                                        // Update latest data
                                        let mut data = latest_data_clone.lock().await;
                                        *data = Some(SlotData {
                                            slot,
                                            swaps: None,
                                            limit_orders: Some(vec![limit_order]),
                                        });
                                    }
                                }
                                
                                warn!("Program subscription stream ended");
                            }
                            Err(e) => {
                                warn!("Failed to subscribe to program: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("WebSocket connection error: {}, reconnecting in 5s...", e);
                    }
                }
                
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        Ok(Self {
            latest_data,
            latest_slot,
            defituna_program: defituna_program.to_string(),
        })
    }

    pub async fn get_latest_slot(&self) -> Result<SlotData> {
        let data = self.latest_data.lock().await;
        let slot = *self.latest_slot.lock().await;
        
        Ok(data.clone().unwrap_or_else(|| SlotData {
            slot,
            swaps: None,
            limit_orders: None,
        }))
    }
}
