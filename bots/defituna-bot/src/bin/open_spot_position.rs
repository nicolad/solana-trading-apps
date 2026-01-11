/// Open a DeFiTuna spot position on Orca Whirlpool (mainnet)
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
    use std::env;

    info!("🐟 Opening DeFiTuna Spot Position on Orca Whirlpool");
    info!("===================================================");

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
    
    let mint_a = Pubkey::from_str(&config.base_mint)?;  // SOL
    let mint_b = Pubkey::from_str(&config.quote_mint)?; // USDC
    
    // Derive tuna_spot_position PDA
    let (tuna_spot_position, _) = Pubkey::find_program_address(
        &[
            b"tuna_spot_position",
            executor_keypair.pubkey().as_ref(),
            whirlpool.as_ref(),
        ],
        &program_id,
    );
    
    // Get ATAs for the spot position PDA
    let tuna_spot_position_ata_a = get_associated_token_address(&tuna_spot_position, &mint_a);
    let tuna_spot_position_ata_b = get_associated_token_address(&tuna_spot_position, &mint_b);
    
    info!("");
    info!("📋 Position Details:");
    info!("   Authority: {}", executor_keypair.pubkey());
    info!("   Whirlpool (Orca): {}", whirlpool);
    info!("   Spot Position PDA: {}", tuna_spot_position);
    info!("   Mint A (SOL): {}", mint_a);
    info!("   Mint B (USDC): {}", mint_b);
    info!("");

    // Check if whirlpool exists
    info!("🔍 Checking Orca Whirlpool...");
    match rpc_client.get_account(&whirlpool) {
        Ok(_account) => {
            info!("✅ Orca Whirlpool exists");
        }
        Err(_) => {
            info!("❌ Whirlpool does not exist");
            return Ok(());
        }
    }

    // Token programs (SOL uses Token Program, USDC uses Token-2022 or Token Program)
    let token_program_a = spl_token::ID; // SOL uses standard token program
    let token_program_b = spl_token::ID; // USDC uses standard token program

    // OpenTunaSpotPosition instruction
    // Discriminator from IDL: [87, 208, 173, 48, 231, 62, 210, 220]
    let discriminator: [u8; 8] = [87, 208, 173, 48, 231, 62, 210, 220];
    let mut data = discriminator.to_vec();
    
    // Add args: position_token (PoolToken), collateral_token (PoolToken)
    // PoolToken::A = 0, PoolToken::B = 1
    data.push(0); // position_token = A (SOL) - this is what we're trading
    data.push(1); // collateral_token = B (USDC) - this is what we use as collateral

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(executor_keypair.pubkey(), true),  // authority (signer, writable)
            AccountMeta::new_readonly(mint_a, false),           // mint_a (SOL)
            AccountMeta::new_readonly(mint_b, false),           // mint_b (USDC)
            AccountMeta::new_readonly(token_program_a, false),  // token_program_a
            AccountMeta::new_readonly(token_program_b, false),  // token_program_b
            AccountMeta::new(tuna_spot_position, false),        // tuna_position (writable)
            AccountMeta::new(tuna_spot_position_ata_a, false),  // tuna_position_ata_a (writable)
            AccountMeta::new(tuna_spot_position_ata_b, false),  // tuna_position_ata_b (writable)
            AccountMeta::new_readonly(whirlpool, false),        // pool (Orca Whirlpool)
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
            info!("");
            info!("This may indicate missing ATAs or insufficient balance.");
            info!("You need USDC as collateral to open a position.");
            return Ok(());
        }
    }

    info!("📤 Submitting transaction...");
    let signature = rpc_client.send_and_confirm_transaction(&transaction)?;
    
    info!("✅ Spot position opened successfully!");
    info!("🔗 Transaction: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", signature);
    info!("📍 Position Address: {}", tuna_spot_position);
    info!("");
    info!("Now you can set limit orders with modify_tuna_spot_position");

    Ok(())
}
