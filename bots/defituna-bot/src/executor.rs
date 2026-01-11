use anyhow::{Context, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use tracing::{info, warn};

use crate::config::BotConfig;
use crate::defituna_client::DefiTunaClient;
use crate::strategies::TradeSignal;

pub struct TradeExecutor {
    rpc_client: RpcClient,
    executor_keypair: Keypair,
    defituna_client: DefiTunaClient,
}

impl TradeExecutor {
    pub async fn new(config: &BotConfig, defituna_client: DefiTunaClient) -> Result<Self> {
        let rpc_client = RpcClient::new(&config.rpc_url);

        let executor_keypair = bs58::decode(&config.executor_keypair)
            .into_vec()
            .context("Invalid executor private key")?;
        let executor_keypair = Keypair::try_from(&executor_keypair[..])
            .context("Failed to parse executor keypair")?;

        info!(
            "Initialized executor: wallet={}",
            executor_keypair.pubkey()
        );

        Ok(Self {
            rpc_client,
            executor_keypair,
            defituna_client,
        })
    }

    pub async fn execute_trade(
        &self,
        signal: &TradeSignal,
        config: &BotConfig,
    ) -> Result<String> {
        match signal {
            TradeSignal::Buy { amount, reason } => {
                info!("Executing BUY: {} | Reason: {}", amount, reason);
                self.defituna_client
                    .execute_market_order(true, *amount, config.max_slippage_bps)
                    .await
            }
            TradeSignal::Sell { amount, reason } => {
                info!("Executing SELL: {} | Reason: {}", amount, reason);
                self.defituna_client
                    .execute_market_order(false, *amount, config.max_slippage_bps)
                    .await
            }
            TradeSignal::PlaceBid { price, size } => {
                info!("Placing BID: price=${:.4}, size={}", price, size);
                self.defituna_client
                    .place_limit_order(true, *price, *size)
                    .await
            }
            TradeSignal::PlaceAsk { price, size } => {
                info!("Placing ASK: price=${:.4}, size={}", price, size);
                self.defituna_client
                    .place_limit_order(false, *price, *size)
                    .await
            }
            TradeSignal::Hold => {
                warn!("Received HOLD signal, but execute_trade was called");
                Err(anyhow::anyhow!("Cannot execute HOLD signal"))
            }
        }
    }

    pub async fn get_balance(&self, mint: &Pubkey) -> Result<u64> {
        info!("💰 Fetching balance for mint: {}", mint);
        
        // TODO: Implement token balance fetching
        // Steps:
        // 1. Derive associated token account (ATA)
        // 2. Fetch account data from RPC
        // 3. Deserialize TokenAccount struct
        // 4. Return balance amount
        
        info!("   Wallet: {}", self.executor_keypair.pubkey());
        info!("   Token Mint: {}", mint);
        info!("📊 Balance: 0 (placeholder - implement ATA lookup)");
        
        Ok(0)
    }

    pub fn pubkey(&self) -> Pubkey {
        self.executor_keypair.pubkey()
    }
}
