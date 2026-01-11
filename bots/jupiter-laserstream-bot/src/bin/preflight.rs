use anyhow::{Context, Result};
use dotenv::dotenv;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{commitment_config::CommitmentConfig, signature::{Keypair, Signer}};
use tracing::info;

use jupiter_laserstream_bot::{config::BotConfig, jupiter_client::JupiterClient};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    info!("🧪 Pre-Flight Check for Devnet Trading");
    info!("=====================================\n");

    // Load config
    let config = BotConfig::from_env()?;

    // 1. Check RPC connection
    info!("1️⃣  Checking RPC connection...");
    let rpc_client = RpcClient::new_with_commitment(
        config.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );
    
    match rpc_client.get_slot() {
        Ok(slot) => info!("   ✅ Connected to RPC (slot: {})", slot),
        Err(e) => {
            info!("   ❌ RPC connection failed: {}", e);
            return Err(e.into());
        }
    }

    // 2. Check wallet
    info!("\n2️⃣  Checking wallet...");
    let keypair_bytes = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid private key format")?;
    let keypair = Keypair::from_bytes(&keypair_bytes)
        .context("Failed to parse keypair")?;
    
    info!("   Public Key: {}", keypair.pubkey());
    
    match rpc_client.get_balance(&keypair.pubkey()) {
        Ok(balance) => {
            let sol = balance as f64 / 1_000_000_000.0;
            info!("   ✅ Balance: {:.4} SOL", sol);
            
            if sol < 0.1 {
                info!("   ⚠️  Low balance! Airdrop needed:");
                info!("      solana airdrop 2 {} --url devnet", keypair.pubkey());
            }
        }
        Err(e) => {
            info!("   ❌ Failed to get balance: {}", e);
        }
    }

    // 3. Check Jupiter API
    info!("\n3️⃣  Checking Jupiter API...");
    let jupiter = JupiterClient::new();
    
    match jupiter.get_price(&config.base_mint, &config.quote_mint).await {
        Ok(price) => {
            info!("   ✅ Jupiter API working");
            info!("   {}/{} price: ${:.4}", config.base_token, config.quote_token, price);
        }
        Err(e) => {
            info!("   ⚠️  Jupiter API error: {}", e);
            info!("   Note: Jupiter may have limited devnet support");
        }
    }

    // 4. Test quote
    info!("\n4️⃣  Testing quote for small swap...");
    let test_amount = 10_000_000; // 0.01 SOL
    match jupiter.get_quote(&config.base_mint, &config.quote_mint, test_amount, 100).await {
        Ok(quote) => {
            info!("   ✅ Quote successful");
            info!("   0.01 SOL → {} USDC", 
                quote.out_amount.parse::<u64>().unwrap_or(0) as f64 / 1_000_000.0);
            info!("   Price Impact: {}%", quote.price_impact_pct);
        }
        Err(e) => {
            info!("   ❌ Quote failed: {}", e);
            info!("   This is expected if Jupiter doesn't support this pair on devnet");
        }
    }

    info!("\n5️⃣  Configuration Summary:");
    info!("   Strategy: {}", config.strategy_type);
    info!("   Trade Amount: {} USDC", config.trade_amount / 1_000_000);
    info!("   Min Movement: {}%", config.min_price_movement * 100.0);
    info!("   Slippage: {}%", config.max_slippage_bps as f64 / 100.0);
    
    info!("\n✅ Pre-flight check complete!");
    info!("\n📝 Next steps:");
    info!("   1. Make sure you have enough SOL for fees (0.1+ SOL)");
    info!("   2. Start with SMALL amounts first");
    info!("   3. Run: cargo run --release");
    
    Ok(())
}
