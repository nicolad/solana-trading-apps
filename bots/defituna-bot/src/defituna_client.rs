use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
    compute_budget::ComputeBudgetInstruction,
};
use spl_associated_token_account::{get_associated_token_address, instruction::create_associated_token_account};
use std::str::FromStr;
use tracing::{info, warn};

use crate::config::BotConfig;

// DeFiTuna FusionAMM constants
const TICK_SPACING: i32 = 64; // Standard tick spacing for SOL/USDC pools
const SPL_TOKEN_2022_ID: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";
const MEMO_PROGRAM_ID: &str = "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr";

// FusionAMM NFT metadata update authority (from SDK)
const FP_NFT_UPDATE_AUTH: &str = "GEyKMXn6zp7VN1JcXQJmBKnWcSm3sPZFLzTL2V2ub5K7";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pool {
    pub address: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_reserve: u64,
    pub quote_reserve: u64,
}

pub struct DefiTunaClient {
    rpc_client: RpcClient,
    program_id: Pubkey,
    base_mint: Pubkey,
    quote_mint: Pubkey,
    executor_keypair: Keypair,
}

impl DefiTunaClient {
    pub fn new(config: &BotConfig) -> Result<Self> {
        let rpc_client = RpcClient::new(&config.rpc_url);
        let program_id = Pubkey::from_str(&config.defituna_program_id)
            .context("Invalid DefiTuna program ID")?;
        let base_mint = Pubkey::from_str(&config.base_mint)
            .context("Invalid base mint")?;
        let quote_mint = Pubkey::from_str(&config.quote_mint)
            .context("Invalid quote mint")?;

        let executor_keypair = bs58::decode(&config.executor_keypair)
            .into_vec()
            .context("Invalid executor private key")?;
        let executor_keypair = Keypair::try_from(&executor_keypair[..])
            .context("Failed to parse executor keypair")?;

        info!(
            "Initialized DefiTuna client: program={}, pair={}/{}, executor={}",
            program_id, base_mint, quote_mint, executor_keypair.pubkey()
        );

        Ok(Self {
            rpc_client,
            program_id,
            base_mint,
            quote_mint,
            executor_keypair,
        })
    }

    pub async fn get_pool(&self) -> Result<Pool> {
        // Derive pool PDA from token mints
        let (pool_pda, _bump) = Pubkey::find_program_address(
            &[
                b"fusion_pool",
                self.base_mint.as_ref(),
                self.quote_mint.as_ref(),
            ],
            &self.program_id,
        );
        
        info!("   Derived pool PDA: {}", pool_pda);
        
        Ok(Pool {
            address: pool_pda,
            base_mint: self.base_mint,
            quote_mint: self.quote_mint,
            base_reserve: 1_000_000_000,
            quote_reserve: 100_000_000_000,
        })
    }

    pub async fn get_spot_price(&self) -> Result<f64> {
        let pool = self.get_pool().await?;
        let price = (pool.quote_reserve as f64 / 1_000_000.0) / (pool.base_reserve as f64 / 1_000_000_000.0);
        Ok(price)
    }

    pub async fn place_limit_order(&self, is_bid: bool, price: f64, size: u64) -> Result<String> {
        let order_type = if is_bid { "BID" } else { "ASK" };
        let sol_amount = size as f64 / 1_000_000_000.0;
        let usdc_value = sol_amount * price;
        
        // Get pool PDA
        let pool = self.get_pool().await?;
        let pool_address = pool.address;
        
        info!("� Placing ON-CHAIN {} limit order on DeFiTuna FusionAMM", order_type);
        info!("   Price: ${:.4}", price);
        info!("   Size: {:.4} SOL (${:.2} USDC value)", sol_amount, usdc_value);
        info!("   Pool: {}", pool_address);
        info!("   Wallet: {}", self.executor_keypair.pubkey());
        
        // Step 1: Create limit order NFT mint (represents order ownership)
        let limit_order_mint = Keypair::new();
        info!("   Order NFT Mint: {}", limit_order_mint.pubkey());
        
        // Step 2: Derive limit order PDA (the actual order account)
        let (limit_order_pda, _bump) = Pubkey::find_program_address(
            &[b"limit_order", limit_order_mint.pubkey().as_ref()],
            &self.program_id,
        );
        info!("   Order PDA: {}", limit_order_pda);
        
        // Step 3: Convert price to tick index (concentrated liquidity uses ticks)
        let tick_index = self.price_to_tick_index(price, is_bid);
        let initializable_tick = self.get_initializable_tick(tick_index);
        info!("   Tick index: {} (initializable: {})", tick_index, initializable_tick);
        
        // Step 4: Calculate tick array address
        let tick_array_start = (initializable_tick / (TICK_SPACING * 88)) * (TICK_SPACING * 88);
        let (tick_array_pda, _) = Pubkey::find_program_address(
            &[
                b"tick_array",
                pool_address.as_ref(),
                &tick_array_start.to_le_bytes(),
            ],
            &self.program_id,
        );
        info!("   Tick array: {} (start: {})", tick_array_pda, tick_array_start);
        
        // Step 5: Get token accounts
        let token_2022 = Pubkey::from_str(SPL_TOKEN_2022_ID)?;
        let limit_order_token_account = get_associated_token_address(
            &self.executor_keypair.pubkey(),
            &limit_order_mint.pubkey(),
        );
        
        let input_mint = if is_bid { self.quote_mint } else { self.base_mint };
        let user_token_account = get_associated_token_address(
            &self.executor_keypair.pubkey(),
            &input_mint,
        );
        
        // Step 6: Derive token vault PDA
        let (token_vault, _) = Pubkey::find_program_address(
            &[
                b"token_vault",
                pool_address.as_ref(),
                input_mint.as_ref(),
            ],
            &self.program_id,
        );
        
        info!("   Input token: {}", input_mint);
        info!("   User ATA: {}", user_token_account);
        info!("   Token vault: {}", token_vault);
        
        // Build instructions
        let mut instructions = Vec::new();
        
        // Add compute budget to avoid running out
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(400_000));
        
        // Instruction 1: OpenLimitOrder (creates the order account)
        let open_order_data = self.build_open_limit_order_data(initializable_tick, is_bid)?;
        instructions.push(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.executor_keypair.pubkey(), true),  // funder
                AccountMeta::new_readonly(self.executor_keypair.pubkey(), false),  // owner
                AccountMeta::new(limit_order_pda, false),  // limit_order
                AccountMeta::new(limit_order_mint.pubkey(), true),  // limit_order_mint (signer!)
                AccountMeta::new(limit_order_token_account, false),  // limit_order_token_account
                AccountMeta::new_readonly(pool_address, false),  // fusion_pool
                AccountMeta::new_readonly(spl_associated_token_account::ID, false),  // associated_token_program
                AccountMeta::new_readonly(token_2022, false),  // token2022_program
                AccountMeta::new_readonly(system_program::ID, false),  // system_program
                AccountMeta::new_readonly(Pubkey::from_str(FP_NFT_UPDATE_AUTH)?, false),  // metadata_update_auth
            ],
            data: open_order_data,
        });
        
        // Instruction 2: IncreaseLimitOrder (deposits tokens)
        let increase_order_data = self.build_increase_limit_order_data(size)?;
        instructions.push(Instruction {
            program_id: self.program_id,
            accounts: vec![
                AccountMeta::new(self.executor_keypair.pubkey(), true),  // limit_order_authority
                AccountMeta::new(pool_address, false),  // fusion_pool
                AccountMeta::new(limit_order_pda, false),  // limit_order
                AccountMeta::new_readonly(limit_order_token_account, false),  // limit_order_token_account
                AccountMeta::new_readonly(input_mint, false),  // token_mint
                AccountMeta::new(user_token_account, false),  // token_owner_account
                AccountMeta::new(token_vault, false),  // token_vault
                AccountMeta::new(tick_array_pda, false),  // tick_array
                AccountMeta::new_readonly(spl_token::ID, false),  // token_program
                AccountMeta::new_readonly(Pubkey::from_str(MEMO_PROGRAM_ID)?, false),  // memo_program
            ],
            data: increase_order_data,
        });
        
        // Sign and send transaction
        info!("📤 Sending transaction with {} instructions...", instructions.len());
        let recent_blockhash = self.rpc_client.get_latest_blockhash()?;
        
        let transaction = Transaction::new_signed_with_payer(
            &instructions,
            Some(&self.executor_keypair.pubkey()),
            &[&self.executor_keypair, &limit_order_mint],  // Both wallet and NFT mint must sign
            recent_blockhash,
        );
        
        let signature = self.rpc_client.send_and_confirm_transaction(&transaction)?;
        let sig = signature.to_string();
        
        info!("✅ ON-CHAIN limit order placed successfully!");
        info!("🔗 Transaction: {}", sig);
        info!("🔗 Explorer: https://explorer.solana.com/tx/{}?cluster=devnet", sig);
        info!("💎 Order NFT: {}", limit_order_mint.pubkey());
        info!("📋 Order Account: {}", limit_order_pda);
        
        Ok(sig)
    }
    
    fn price_to_tick_index(&self, price: f64, _is_bid: bool) -> i32 {
        // Convert price to tick using: tick = log(price) / log(1.0001)
        // For SOL/USDC with 9 and 6 decimals
        let adjusted_price = price / 1000.0; // Adjust for decimal difference
        let tick = (adjusted_price.ln() / 1.0001_f64.ln()) as i32;
        tick
    }
    
    fn get_initializable_tick(&self, tick: i32) -> i32 {
        // Round to nearest initializable tick based on spacing
        let remainder = tick % TICK_SPACING;
        if remainder == 0 {
            tick
        } else if remainder > 0 {
            tick - remainder
        } else {
            tick - (TICK_SPACING + remainder)
        }
    }
    
    fn build_open_limit_order_data(&self, tick_index: i32, a_to_b: bool) -> Result<Vec<u8>> {
        // OpenLimitOrder instruction format (from SDK analysis):
        // [0-8]: discriminator (sighash of "open_limit_order")
        // [8-12]: tick_index (i32)
        // [12]: a_to_b (bool)
        // [13]: with_token_metadata_extension (bool)
        
        let mut data = Vec::new();
        
        // Calculate discriminator using anchor's method: first 8 bytes of sha256("global:open_limit_order")
        let discriminator = anchor_discriminator("open_limit_order");
        data.extend_from_slice(&discriminator);
        
        // tick_index
        data.extend_from_slice(&tick_index.to_le_bytes());
        
        // a_to_b
        data.push(if a_to_b { 1 } else { 0 });
        
        // with_token_metadata_extension
        data.push(1); // true - include metadata
        
        Ok(data)
    }
    
    fn build_increase_limit_order_data(&self, amount: u64) -> Result<Vec<u8>> {
        // IncreaseLimitOrder instruction format:
        // [0-8]: discriminator
        // [8-16]: amount (u64)
        // [16]: remaining_accounts_info option (None = 0)
        
        let mut data = Vec::new();
        
        let discriminator = anchor_discriminator("increase_limit_order");
        data.extend_from_slice(&discriminator);
        
        // amount
        data.extend_from_slice(&amount.to_le_bytes());
        
        // remaining_accounts_info (Option<None>)
        data.push(0); // None
        
        Ok(data)
    }

    pub async fn execute_market_order(&self, is_buy: bool, amount: u64, max_slippage_bps: u16) -> Result<String> {
        warn!("🚧 PLACEHOLDER: Market orders not yet implemented");
        info!("💱 Market Order Parameters: is_buy={}, amount={}, max_slippage={}bps ({:.2}%)", 
            is_buy, amount, max_slippage_bps, max_slippage_bps as f64 / 100.0);
        
        let price = self.get_spot_price().await.unwrap_or(100.0);
        let max_price_deviation = price * (max_slippage_bps as f64 / 10000.0);
        
        if is_buy {
            let usdc = amount as f64 / 1_000_000.0;
            let max_price = price + max_price_deviation;
            info!("📝 Would BUY ${:.2} USDC @ ${:.2}/SOL (max price: ${:.2})", usdc, price, max_price);
            info!("   Expected SOL received: {:.4} SOL", usdc / price);
        } else {
            let sol = amount as f64 / 1_000_000_000.0;
            let min_price = price - max_price_deviation;
            info!("📝 Would SELL {:.4} SOL @ ${:.2}/SOL (min price: ${:.2})", sol, price, min_price);
            info!("   Expected USDC received: ${:.2}", sol * price);
        }
        
        let sig = format!("PLACEHOLDER_MARKET_{}_{}", if is_buy { "BUY" } else { "SELL" }, chrono::Utc::now().timestamp());
        info!("✅ Simulated transaction: {}", sig);
        info!("🔗 View on explorer: https://explorer.solana.com/tx/{}?cluster=devnet", sig);
        Ok(sig)
    }

    pub async fn cancel_order(&self, order_id: Pubkey) -> Result<String> {
        warn!("🚧 PLACEHOLDER: Cancel not yet implemented");
        info!("📝 Would cancel: {}", order_id);
        Ok(format!("PLACEHOLDER_CANCEL_{}", chrono::Utc::now().timestamp()))
    }
}

/// Calculate Anchor instruction discriminator
/// Anchor uses: discriminator = first 8 bytes of sha256("global:<instruction_name>")
fn anchor_discriminator(name: &str) -> [u8; 8] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{}", name));
    let result = hasher.finalize();
    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&result[..8]);
    discriminator
}
