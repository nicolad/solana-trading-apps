use anyhow::{Context, Result};
use base64::Engine;
use bincode;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    transaction::VersionedTransaction,
};
use std::str::FromStr;
use tracing::{info, warn};

use crate::config::BotConfig;
use crate::jupiter_client::JupiterClient;
use crate::strategies::TradeSignal;

pub struct TradeExecutor {
    rpc_client: RpcClient,
    executor: Keypair,
    vault_program_id: Pubkey,
    vault_state: Pubkey,
    jupiter_client: JupiterClient,
}

impl TradeExecutor {
    pub async fn new(config: &BotConfig) -> Result<Self> {
        let rpc_client = RpcClient::new(&config.rpc_url);
        
        // Decode executor keypair from base58
        let keypair_bytes = bs58::decode(&config.executor_keypair)
            .into_vec()
            .context("Invalid executor keypair")?;
        let executor = Keypair::from_bytes(&keypair_bytes)
            .context("Failed to parse keypair")?;
        
        let vault_program_id = Pubkey::from_str(&config.vault_program_id)?;
        let vault_state = Pubkey::from_str(&config.vault_state_address)?;
        let jupiter_client = JupiterClient::new();
        
        info!("Executor pubkey: {}", executor.pubkey());
        info!("Vault program: {}", vault_program_id);
        
        Ok(Self {
            rpc_client,
            executor,
            vault_program_id,
            vault_state,
            jupiter_client,
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
                self.execute_swap(
                    &config.quote_mint,
                    &config.base_mint,
                    *amount,
                    config.max_slippage_bps,
                ).await
            }
            TradeSignal::Sell { amount, reason } => {
                info!("Executing SELL: {} | Reason: {}", amount, reason);
                self.execute_swap(
                    &config.base_mint,
                    &config.quote_mint,
                    *amount,
                    config.max_slippage_bps,
                ).await
            }
            TradeSignal::Hold => {
                warn!("Received HOLD signal, but execute_trade was called");
                Err(anyhow::anyhow!("Cannot execute HOLD signal"))
            }
        }
    }
    
    async fn execute_swap(
        &self,
        input_mint: &str,
        output_mint: &str,
        amount_in: u64,
        slippage_bps: u16,
    ) -> Result<String> {
        let input_mint_pubkey = Pubkey::from_str(input_mint)?;
        let output_mint_pubkey = Pubkey::from_str(output_mint)?;
        
        info!("Swap: {} {} -> {}",
              amount_in,
              input_mint_pubkey.to_string().split_at(8).0,
              output_mint_pubkey.to_string().split_at(8).0);
        
        // Step 1: Get quote from Jupiter
        info!("📊 Getting quote from Jupiter...");
        let quote = self.jupiter_client.get_quote(
            input_mint,
            output_mint,
            amount_in,
            slippage_bps,
        ).await?;
        
        info!("✅ Quote received: {} {} -> {} {} (impact: {}%)",
              quote.in_amount,
              input_mint_pubkey.to_string().split_at(8).0,
              quote.out_amount,
              output_mint_pubkey.to_string().split_at(8).0,
              quote.price_impact_pct);
        
        // Step 2: Get swap transaction
        info!("🔨 Building swap transaction...");
        let swap_response = self.jupiter_client.get_swap_transaction(
            &quote,
            &self.executor.pubkey().to_string(),
            true, // Wrap/unwrap SOL if needed
        ).await?;
        
        // Step 3: Deserialize and sign transaction
        let transaction_bytes = base64::engine::general_purpose::STANDARD
            .decode(&swap_response.swap_transaction)
            .context("Failed to decode swap transaction")?;
        
        let mut transaction: VersionedTransaction = bincode::deserialize(&transaction_bytes)
            .context("Failed to deserialize transaction")?;
        
        // Get latest blockhash for transaction
        let blockhash = self.rpc_client.get_latest_blockhash()
            .context("Failed to get latest blockhash")?;
        
        // Sign the transaction with our executor keypair
        transaction.message.set_recent_blockhash(blockhash);
        transaction = VersionedTransaction::try_new(
            transaction.message,
            &[&self.executor],
        ).context("Failed to sign transaction")?;
        
        // Step 4: Simulate transaction first (safety check)
        info!("🔍 Simulating transaction...");
        match self.rpc_client.simulate_transaction(&transaction) {
            Ok(result) => {
                if let Some(err) = result.value.err {
                    anyhow::bail!("Transaction simulation failed: {:?}", err);
                }
                info!("✅ Simulation successful");
            }
            Err(e) => {
                warn!("Simulation check failed: {}, proceeding anyway", e);
            }
        }
        
        // Step 5: Send and confirm transaction
        info!("📤 Sending transaction...");
        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)
            .context("Failed to send transaction")?;
        
        info!("✅ Transaction confirmed: {}", signature);
        
        Ok(signature.to_string())
    }
}
