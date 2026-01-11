/// Set limit orders on DeFiTuna spot position  
/// Run with: ./target/release/place_order --position <POSITION_PUBKEY> --lower-price 95 --upper-price 105
use anyhow::{Context, Result};
use clap::Parser;
use dotenvy::dotenv;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(name = "place_order")]
#[command(about = "Set limit orders on DeFiTuna spot position", long_about = None)]
struct Args {
    /// Lower limit price in USD per SOL (for bids/buys)
    #[arg(long)]
    lower_price: Option<f64>,

    /// Upper limit price in USD per SOL (for asks/sells)
    #[arg(long)]
    upper_price: Option<f64>,
    
    /// Spot position address (required)
    #[arg(short, long)]
    position: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    dotenv().ok();

    let args = Args::parse();

    use defituna_bot::config::BotConfig;
    use solana_client::rpc_client::RpcClient;
    use solana_sdk::{
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        transaction::Transaction,
    };
    use tracing::info;

    info!("🐟 DeFiTuna Limit Order Setup");
    info!("==========================================");

    let config = BotConfig::from_env()?;
    let rpc_client = RpcClient::new(&config.rpc_url);
    
    let executor_keypair = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid executor private key")?;
    let executor_keypair = Keypair::try_from(&executor_keypair[..])
        .context("Failed to parse executor keypair")?;
    
    let program_id = Pubkey::from_str(&config.defituna_program_id)?;
    let position = Pubkey::from_str(&args.position)?;

    info!("");
    info!("📋 Limit Order Details:");
    if let Some(lower) = args.lower_price {
        info!("   Lower limit (bid): ${:.2}", lower);
    }
    if let Some(upper) = args.upper_price {
        info!("   Upper limit (ask): ${:.2}", upper);
    }
    info!("   Position: {}", position);
    info!("");

    // Check if position exists
    info!("🔍 Checking position account...");
    match rpc_client.get_account(&position) {
        Ok(account) => {
            if account.owner == program_id {
                info!("✅ Position account exists and owned by DeFiTuna program");
            } else {
                info!("❌ Position account exists but not owned by DeFiTuna program");
                info!("   Expected owner: {}", program_id);
                info!("   Actual owner: {}", account.owner);
                info!("");
                info!("⚠️  The position address might be incorrect.");
                return Ok(());
            }
        }
        Err(_) => {
            info!("❌ Position account does not exist");
            info!("");
            info!("⚠️  You need to open a position first!");
            info!("");
            info!("Steps to fix:");
            info!("  1. Open a position first: ./target/release/open_position");
            info!("  2. Note: Positions require initialized pools");
            info!("  3. On devnet: Pools don't exist (see README_DEVNET.md)");
            info!("  4. For devnet testing: Use Jupiter bot instead");
            info!("");
            return Ok(());
        }
    }

    // Convert prices to sqrt prices (Q64.64 fixed point)
    let lower_sqrt = args.lower_price.map(price_to_sqrt_price_x64).unwrap_or(0u128);
    let upper_sqrt = args.upper_price.map(price_to_sqrt_price_x64).unwrap_or(u128::MAX);

    // Build instruction with discriminator from IDL
    let discriminator: [u8; 8] = [10, 180, 19, 205, 169, 133, 52, 118];
    let mut data = discriminator.to_vec();
    data.extend_from_slice(&lower_sqrt.to_le_bytes());
    data.extend_from_slice(&upper_sqrt.to_le_bytes());

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(executor_keypair.pubkey(), true),
            AccountMeta::new(position, false),
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
                info!("");
                info!("This usually means:");
                info!("  - Position is not properly initialized");
                info!("  - Invalid price ranges");
                info!("  - Incorrect position address");
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
    info!("🔗 Transaction: https://explorer.solana.com/tx/{}?cluster=devnet", signature);

    Ok(())
}

fn price_to_sqrt_price_x64(price: f64) -> u128 {
    // Convert price to sqrt price in Q64.64 fixed point format
    // sqrt_price = sqrt(price) * 2^64
    let sqrt_price = price.sqrt();
    (sqrt_price * (1u128 << 64) as f64) as u128
}
