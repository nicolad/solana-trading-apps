/// Set limit orders on DeFiTuna spot position
use anyhow::{Context, Result};
use dotenvy::dotenv;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

// Convert price to sqrt_price for limit orders
// Formula: sqrt_price = sqrt(price) * 2^64
fn price_to_sqrt_price(price: f64, decimals_a: u8, decimals_b: u8) -> u128 {
    let decimal_diff = decimals_a as i32 - decimals_b as i32;
    let adjusted_price = if decimal_diff >= 0 {
        price * 10_f64.powi(decimal_diff)
    } else {
        price / 10_f64.powi(-decimal_diff)
    };
    
    let sqrt_price_f64 = adjusted_price.sqrt() * (1u128 << 64) as f64;
    sqrt_price_f64 as u128
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    dotenv().ok();

    use defituna_bot::config::BotConfig;
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use tracing::info;
    use std::env;

    info!("🎯 Setting DeFiTuna Spot Position Limit Orders");
    info!("================================================");

    let config = BotConfig::from_env()?;
    let rpc_client = RpcClient::new(&config.rpc_url);
    
    let executor_keypair = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid executor private key")?;
    let executor_keypair = Keypair::try_from(&executor_keypair[..])
        .context("Failed to parse executor keypair")?;
    
    let program_id = Pubkey::from_str(&config.defituna_program_id)?;
    
    // Get Orca Whirlpool address from env
    let whirlpool = Pubkey::from_str(
        &env::var("WHIRLPOOL_ADDRESS")
            .context("WHIRLPOOL_ADDRESS not set")?
    )?;
    
    // Derive tuna_spot_position PDA (same as open_position)
    let (tuna_spot_position, _) = Pubkey::find_program_address(
        &[
            b"tuna_spot_position",
            executor_keypair.pubkey().as_ref(),
            whirlpool.as_ref(),
        ],
        &program_id,
    );
    
    info!("");
    info!("📋 Position Details:");
    info!("   Authority: {}", executor_keypair.pubkey());
    info!("   Position PDA: {}", tuna_spot_position);
    info!("   Whirlpool: {}", whirlpool);
    info!("");

    // Check if position exists
    info!("🔍 Checking position exists...");
    match rpc_client.get_account(&tuna_spot_position) {
        Ok(_account) => {
            info!("✅ Position exists");
        }
        Err(_) => {
            info!("❌ Position does not exist");
            info!("   Run open_spot_position first!");
            return Ok(());
        }
    }

    // Set limit order prices
    // For SOL/USDC pool (SOL has 9 decimals, USDC has 6 decimals)
    // Current price ~$136.45, let's set:
    // - Lower limit: $130 (buy SOL if price drops)
    // - Upper limit: $145 (sell SOL if price rises)
    
    let lower_price = 130.0; // Buy at $130
    let upper_price = 145.0; // Sell at $145
    
    let lower_sqrt_price = price_to_sqrt_price(lower_price, 9, 6);
    let upper_sqrt_price = price_to_sqrt_price(upper_price, 9, 6);
    
    info!("💰 Limit Order Prices:");
    info!("   Lower (buy): ${} → sqrt_price: {}", lower_price, lower_sqrt_price);
    info!("   Upper (sell): ${} → sqrt_price: {}", upper_price, upper_sqrt_price);
    info!("");

    // SetTunaSpotPositionLimitOrders instruction
    // Discriminator: [10, 180, 19, 205, 169, 133, 52, 118]
    let discriminator: [u8; 8] = [10, 180, 19, 205, 169, 133, 52, 118];
    let mut data = discriminator.to_vec();
    
    // Add args: lower_limit_order_sqrt_price (u128), upper_limit_order_sqrt_price (u128)
    data.extend_from_slice(&lower_sqrt_price.to_le_bytes());
    data.extend_from_slice(&upper_sqrt_price.to_le_bytes());

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(executor_keypair.pubkey(), true), // authority (signer)
            AccountMeta::new(tuna_spot_position, false),                // tuna_position (writable)
        ],
        data,
    };

    info!("📤 Simulating transaction...");
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction.clone()],
        Some(&executor_keypair.pubkey()),
        &[&executor_keypair],
        recent_blockhash,
    );

    // Simulate first
    match rpc_client.simulate_transaction(&transaction) {
        Ok(result) => {
            if let Some(err) = result.value.err {
                info!("❌ Simulation failed: {:?}", err);
                if let Some(logs) = result.value.logs {
                    info!("📋 Transaction logs:");
                    for log in logs {
                        info!("   {}", log);
                    }
                }
                return Ok(());
            }
            info!("✅ Simulation successful");
        }
        Err(e) => {
            info!("❌ Simulation error: {}", e);
            return Ok(());
        }
    }

    info!("📤 Submitting transaction...");
    let signature = rpc_client.send_and_confirm_transaction(&transaction)?;
    
    info!("✅ Limit orders set successfully!");
    info!("🔗 Transaction: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", signature);
    info!("");
    info!("📊 Your limit orders:");
    info!("   • BUY SOL if price drops to ${}", lower_price);
    info!("   • SELL SOL if price rises to ${}", upper_price);
    info!("");
    info!("Note: You still need to add collateral via modify_tuna_spot_position");
    info!("      to activate trading. Limit orders define price triggers only.");
    
    Ok(())
}
