use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    // LaserStream adapter
    pub adapter_ws_url: String,
    
    // Trading pair
    pub base_token: String,
    pub quote_token: String,
    pub base_mint: String,
    pub quote_mint: String,
    
    // Strategy
    pub strategy_type: String,
    pub trade_amount: u64,
    pub min_price_movement: f64,
    pub lookback_minutes: usize,
    
    // Risk management
    pub max_position_size: u64,
    pub max_slippage_bps: u16,
    pub cooldown_minutes: u64,
    
    // Solana
    pub rpc_url: String,
    pub executor_keypair: String,
    
    // Jupiter vault
    pub vault_program_id: String,
    pub vault_state_address: String,
}

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        let adapter_ws_url = env::var("ADAPTER_WS_URL")
            .unwrap_or_else(|_| "ws://localhost:8080".to_string());
        
        let base_token = env::var("BASE_TOKEN")
            .unwrap_or_else(|_| "SOL".to_string());
        
        let quote_token = env::var("QUOTE_TOKEN")
            .unwrap_or_else(|_| "USDC".to_string());
        
        // Default mints
        let base_mint = env::var("BASE_MINT")
            .unwrap_or_else(|_| "So11111111111111111111111111111111111111112".to_string());
        
        let quote_mint = env::var("QUOTE_MINT")
            .unwrap_or_else(|_| "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string());
        
        let strategy_type = env::var("STRATEGY")
            .unwrap_or_else(|_| "momentum".to_string());
        
        let trade_amount = env::var("TRADE_AMOUNT_USDC")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<u64>()?
            * 1_000_000; // Convert to lamports (6 decimals for USDC)
        
        let min_price_movement = env::var("MIN_PRICE_MOVEMENT")
            .unwrap_or_else(|_| "0.02".to_string())
            .parse()?;
        
        let lookback_minutes = env::var("LOOKBACK_MINUTES")
            .unwrap_or_else(|_| "60".to_string())
            .parse()?;
        
        let max_position_size = env::var("MAX_POSITION_SIZE")
            .unwrap_or_else(|_| "10000".to_string())
            .parse::<u64>()?
            * 1_000_000;
        
        let max_slippage_bps = env::var("MAX_SLIPPAGE_BPS")
            .unwrap_or_else(|_| "50".to_string())
            .parse()?;
        
        let cooldown_minutes = env::var("COOLDOWN_MINUTES")
            .unwrap_or_else(|_| "60".to_string())
            .parse()?;
        
        let rpc_url = env::var("RPC_URL")
            .context("RPC_URL not set")?;
        
        let executor_keypair = env::var("EXECUTOR_PRIVATE_KEY")
            .context("EXECUTOR_PRIVATE_KEY not set")?;
        
        let vault_program_id = env::var("VAULT_PROGRAM_ID")
            .context("VAULT_PROGRAM_ID not set")?;
        
        let vault_state_address = env::var("VAULT_STATE_ADDRESS")
            .context("VAULT_STATE_ADDRESS not set")?;
        
        Ok(Self {
            adapter_ws_url,
            base_token,
            quote_token,
            base_mint,
            quote_mint,
            strategy_type,
            trade_amount,
            min_price_movement,
            lookback_minutes,
            max_position_size,
            max_slippage_bps,
            cooldown_minutes,
            rpc_url,
            executor_keypair,
            vault_program_id,
            vault_state_address,
        })
    }
}
