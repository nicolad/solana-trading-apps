/// Open a test spot position on DeFiTuna devnet
use anyhow::{Context, Result};
use dotenvy::dotenv;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

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
        system_program,
        transaction::Transaction,
    };
    use spl_associated_token_account::get_associated_token_address;
    use tracing::info;

    info!("🐟 Opening DeFiTuna Spot Position on Devnet");
    info!("==========================================");

    let config = BotConfig::from_env()?;
    let rpc_client = RpcClient::new(&config.rpc_url);
    
    let executor_keypair = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid executor private key")?;
    let executor_keypair = Keypair::try_from(&executor_keypair[..])
        .context("Failed to parse executor keypair")?;
    
    let program_id = Pubkey::from_str(&config.defituna_program_id)?;
    let mint_a = Pubkey::from_str(&config.base_mint)?;  // SOL
    let mint_b = Pubkey::from_str(&config.quote_mint)?; // USDC
    
    // Derive pool PDA (this might need adjustment based on actual pool structure)
    let (pool_pda, _) = Pubkey::find_program_address(
        &[
            b"fusion_pool",
            mint_a.as_ref(),
            mint_b.as_ref(),
        ],
        &program_id,
    );
    
    // Derive tuna_position PDA - use just authority for now
    let (tuna_position, _) = Pubkey::find_program_address(
        &[
            b"tuna_position",
            executor_keypair.pubkey().as_ref(),
        ],
        &program_id,
    );
    
    // Get ATAs for the position
    let tuna_position_ata_a = get_associated_token_address(&tuna_position, &mint_a);
    let tuna_position_ata_b = get_associated_token_address(&tuna_position, &mint_b);
    
    // Token programs
    let token_program_a = spl_token::ID;
    let token_program_b = spl_token::ID;
    
    info!("");
    info!("📋 Position Details:");
    info!("   Authority: {}", executor_keypair.pubkey());
    info!("   Pool: {}", pool_pda);
    info!("   Position PDA: {}", tuna_position);
    info!("   Mint A (SOL): {}", mint_a);
    info!("   Mint B (USDC): {}", mint_b);
    info!("");

    // Check if pool exists
    info!("🔍 Checking pool account...");
    match rpc_client.get_account(&pool_pda) {
        Ok(_account) => {
            info!("✅ Pool exists on devnet");
        }
        Err(_) => {
            info!("❌ Pool does not exist on devnet");
            info!("");
            info!("⚠️  The SOL/USDC fusion pool is not initialized on devnet.");
            info!("   Pool address would be: {}", pool_pda);
            info!("");
            info!("Options:");
            info!("  1. Use mainnet: solana config set --url mainnet-beta");
            info!("  2. Initialize pool on devnet (requires admin access)");
            info!("  3. Use Jupiter bot for devnet testing: cd ../jupiter-laserstream-bot");
            info!("");
            return Ok(());
        }
    }

    // Build instruction
    // Discriminator: [87, 208, 173, 48, 231, 62, 210, 220]
    // Args: position_token (PoolToken), collateral_token (PoolToken)
    // PoolToken::A = 0, PoolToken::B = 1
    let discriminator: [u8; 8] = [87, 208, 173, 48, 231, 62, 210, 220];
    let mut data = discriminator.to_vec();
    data.push(0); // position_token = PoolToken::A (SOL)
    data.push(1); // collateral_token = PoolToken::B (USDC)

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(executor_keypair.pubkey(), true),  // authority
            AccountMeta::new_readonly(mint_a, false),           // mint_a
            AccountMeta::new_readonly(mint_b, false),           // mint_b
            AccountMeta::new_readonly(token_program_a, false),  // token_program_a
            AccountMeta::new_readonly(token_program_b, false),  // token_program_b
            AccountMeta::new(tuna_position, false),             // tuna_position
            AccountMeta::new(tuna_position_ata_a, false),       // tuna_position_ata_a
            AccountMeta::new(tuna_position_ata_b, false),       // tuna_position_ata_b
            AccountMeta::new_readonly(pool_pda, false),         // pool
            AccountMeta::new_readonly(system_program::ID, false), // system_program
            AccountMeta::new_readonly(spl_associated_token_account::ID, false), // associated_token_program
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
                info!("  - Pool is not properly initialized");
                info!("  - Incorrect account structure");
                info!("  - Missing token accounts");
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
    
    info!("✅ Position opened successfully!");
    info!("🔗 Transaction: https://explorer.solana.com/tx/{}?cluster=devnet", signature);
    info!("📍 Position Address: {}", tuna_position);
    info!("");
    info!("Now you can set limit orders with:");
    info!("  ./target/release/place_order --position {} --lower-price 95 --upper-price 105", tuna_position);

    Ok(())
}
