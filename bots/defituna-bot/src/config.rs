use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct BotConfig {
    // RPC endpoints
    pub rpc_ws_url: String,
    pub poll_interval_seconds: u64,

    // Trading pair
    pub base_token: String,
    pub quote_token: String,
    pub base_mint: String,
    pub quote_mint: String,

    // Strategy
    pub strategy_type: String,
    pub trade_amount: u64,

    // Market maker specific (for limit orders)
    pub spread_bps: u16,
    pub order_size: u64,
    pub max_position_size: u64,

    // Arbitrage specific
    pub min_profit_bps: u16,

    // Risk management
    pub max_slippage_bps: u16,
    pub cooldown_minutes: u64,
    pub max_daily_trades: usize,

    // Solana
    pub rpc_url: String,
    pub executor_keypair: String,

    // DefiTuna
    pub defituna_program_id: String,

    // Strategy parameters
    pub lookback_minutes: usize,
    pub min_price_movement: f64,
}

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        // Build RPC URL using HELIUS_API_KEY from root .env if available
        let rpc_url = if let Ok(helius_key) = env::var("HELIUS_API_KEY") {
            let network = env::var("SOLANA_NETWORK").unwrap_or_else(|_| "devnet".to_string());
            format!("https://{}.helius-rpc.com/?api-key={}", network, helius_key)
        } else {
            env::var("RPC_URL")
                .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string())
        };

        Ok(Self {
            rpc_ws_url: env::var("RPC_WS_URL")
                .unwrap_or_else(|_| "wss://api.devnet.solana.com".to_string()),
            poll_interval_seconds: env::var("POLL_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "2".to_string())
                .parse()
                .context("Invalid POLL_INTERVAL_SECONDS")?,

            base_token: env::var("BASE_TOKEN").unwrap_or_else(|_| "SOL".to_string()),
            quote_token: env::var("QUOTE_TOKEN").unwrap_or_else(|_| "USDC".to_string()),
            base_mint: env::var("BASE_MINT")
                .unwrap_or_else(|_| "So11111111111111111111111111111111111111112".to_string()),
            quote_mint: env::var("QUOTE_MINT")
                .unwrap_or_else(|_| "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),

            strategy_type: env::var("STRATEGY").unwrap_or_else(|_| "market_maker".to_string()),
            trade_amount: env::var("TRADE_AMOUNT_USDC")
                .unwrap_or_else(|_| "100".to_string())
                .parse::<u64>()
                .context("Invalid TRADE_AMOUNT_USDC")?
                * 1_000_000, // Convert to lamports (6 decimals for USDC)

            spread_bps: env::var("SPREAD_BPS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .context("Invalid SPREAD_BPS")?,
            order_size: env::var("ORDER_SIZE_USDC")
                .unwrap_or_else(|_| "50".to_string())
                .parse::<u64>()
                .context("Invalid ORDER_SIZE_USDC")?
                * 1_000_000,
            max_position_size: env::var("MAX_POSITION_SIZE")
                .unwrap_or_else(|_| "1000".to_string())
                .parse::<u64>()
                .context("Invalid MAX_POSITION_SIZE")?
                * 1_000_000,

            min_profit_bps: env::var("MIN_PROFIT_BPS")
                .unwrap_or_else(|_| "20".to_string())
                .parse()
                .context("Invalid MIN_PROFIT_BPS")?,

            max_slippage_bps: env::var("MAX_SLIPPAGE_BPS")
                .unwrap_or_else(|_| "50".to_string())
                .parse()
                .context("Invalid MAX_SLIPPAGE_BPS")?,
            cooldown_minutes: env::var("COOLDOWN_MINUTES")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .context("Invalid COOLDOWN_MINUTES")?,
            max_daily_trades: env::var("MAX_DAILY_TRADES")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .context("Invalid MAX_DAILY_TRADES")?,

            rpc_url,
            executor_keypair: env::var("EXECUTOR_KEYPAIR")
                .or_else(|_| env::var("EXECUTOR_PRIVATE_KEY"))
                .or_else(|_| env::var("PRIVATE_KEY"))
                .context("EXECUTOR_KEYPAIR, EXECUTOR_PRIVATE_KEY, or PRIVATE_KEY not set")?,

            defituna_program_id: env::var("DEFITUNA_PROGRAM_ID")
                .unwrap_or_else(|_| "tuna4uSQZncNeeiAMKbstuxA9CUkHH6HmC64wgmnogD".to_string()),

            lookback_minutes: env::var("LOOKBACK_MINUTES")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .context("Invalid LOOKBACK_MINUTES")?,
            min_price_movement: env::var("MIN_PRICE_MOVEMENT")
                .unwrap_or_else(|_| "0.02".to_string())
                .parse()
                .context("Invalid MIN_PRICE_MOVEMENT")?,
        })
    }
}
