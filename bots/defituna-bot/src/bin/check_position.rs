/// Check DeFiTuna position status on mainnet
use anyhow::{Context, Result};
use dotenvy::dotenv;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn sqrt_price_to_price(sqrt_price: u128, decimals_a: u8, decimals_b: u8) -> f64 {
    let decimal_diff = decimals_a as i32 - decimals_b as i32;
    let price_raw = (sqrt_price as f64 / (1u128 << 64) as f64).powi(2);
    
    if decimal_diff >= 0 {
        price_raw / 10_f64.powi(decimal_diff)
    } else {
        price_raw * 10_f64.powi(-decimal_diff)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    dotenv().ok();

    use defituna_bot::config::BotConfig;
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::signature::{Keypair, Signer};
    use tracing::info;
    use std::env;

    info!("📊 DeFiTuna Position Status");
    info!("============================");
    info!("");

    let config = BotConfig::from_env()?;
    let rpc_client = RpcClient::new(&config.rpc_url);
    
    let executor_keypair = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid executor private key")?;
    let executor_keypair = Keypair::try_from(&executor_keypair[..])
        .context("Failed to parse executor keypair")?;
    
    let program_id = Pubkey::from_str(&config.defituna_program_id)?;
    let whirlpool = Pubkey::from_str(&env::var("WHIRLPOOL_ADDRESS")?)?;
    
    let (tuna_spot_position, _) = Pubkey::find_program_address(
        &[
            b"tuna_spot_position",
            executor_keypair.pubkey().as_ref(),
            whirlpool.as_ref(),
        ],
        &program_id,
    );
    
    info!("🔍 Position Details:");
    info!("   Address: {}", tuna_spot_position);
    info!("   Authority: {}", executor_keypair.pubkey());
    info!("   Pool: {}", whirlpool);
    info!("");
    
    match rpc_client.get_account(&tuna_spot_position) {
        Ok(account) => {
            info!("✅ Position exists on-chain!");
            info!("   Owner: {}", account.owner);
            info!("   Lamports: {} SOL", account.lamports as f64 / 1e9);
            info!("   Data size: {} bytes", account.data.len());
            info!("");
            
            // Parse position data (simplified)
            if account.data.len() >= 200 {
                // Discriminator at 0-8
                // Authority at 8-40
                // Pool at 40-72
                // Position token at ~90
                // Collateral token at ~91
                // Amount at ~92-100
                // Borrowed at ~100-108
                // Lower limit sqrt_price at ~184-200
                // Upper limit sqrt_price at ~200-216
                
                let lower_bytes: [u8; 16] = account.data[184..200].try_into().unwrap();
                let upper_bytes: [u8; 16] = account.data[200..216].try_into().unwrap();
                
                let lower_sqrt_price = u128::from_le_bytes(lower_bytes);
                let upper_sqrt_price = u128::from_le_bytes(upper_bytes);
                
                if lower_sqrt_price > 0 || upper_sqrt_price > 0 {
                    info!("🎯 Limit Orders Set:");
                    
                    if lower_sqrt_price > 0 {
                        let lower_price = sqrt_price_to_price(lower_sqrt_price, 9, 6);
                        info!("   Lower (Buy): ${:.2}", lower_price);
                        info!("      √price: {}", lower_sqrt_price);
                    }
                    
                    if upper_sqrt_price > 0 {
                        let upper_price = sqrt_price_to_price(upper_sqrt_price, 9, 6);
                        info!("   Upper (Sell): ${:.2}", upper_price);
                        info!("      √price: {}", upper_sqrt_price);
                    }
                    info!("");
                }
                
                // Check amount and borrowed
                let amount_bytes: [u8; 8] = account.data[92..100].try_into().unwrap_or([0u8; 8]);
                let borrowed_bytes: [u8; 8] = account.data[100..108].try_into().unwrap_or([0u8; 8]);
                
                let amount = u64::from_le_bytes(amount_bytes);
                let borrowed = u64::from_le_bytes(borrowed_bytes);
                
                info!("💰 Position Funding:");
                info!("   Amount: {} (position token)", amount);
                info!("   Borrowed: {} (position token)", borrowed);
                
                if amount == 0 && borrowed == 0 {
                    info!("   Status: ⚠️  UNFUNDED");
                    info!("");
                    info!("   Your position is created with limit orders set,");
                    info!("   but needs collateral to start trading.");
                    info!("");
                    info!("   To activate:");
                    info!("   1. Get USDC (swap SOL at https://jup.ag)");
                    info!("   2. Add collateral via modify_tuna_spot_position");
                } else {
                    info!("   Status: ✅ ACTIVE");
                    info!("");
                    info!("   Position will automatically execute when");
                    info!("   SOL price hits your limit order levels!");
                }
            }
            
            info!("");
            info!("🔗 View on Explorer:");
            info!("   https://explorer.solana.com/address/{}?cluster=mainnet-beta", tuna_spot_position);
        }
        Err(_) => {
            info!("❌ Position does not exist");
            info!("   Run: ./target/release/open_spot_position");
        }
    }
    
    Ok(())
}
