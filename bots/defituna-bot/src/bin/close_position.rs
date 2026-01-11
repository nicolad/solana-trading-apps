/// Close DeFiTuna spot position
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

    info!("🔒 Closing DeFiTuna Position");
    info!("=============================");

    let config = BotConfig::from_env()?;
    let rpc_client = RpcClient::new(&config.rpc_url);
    
    let executor_keypair = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid executor private key")?;
    let executor_keypair = Keypair::try_from(&executor_keypair[..])
        .context("Failed to parse executor keypair")?;
    
    let program_id = Pubkey::from_str(&config.defituna_program_id)?;
    let whirlpool = Pubkey::from_str(&env::var("WHIRLPOOL_ADDRESS")?)?;
    let mint_a = Pubkey::from_str(&config.base_mint)?;
    let mint_b = Pubkey::from_str(&config.quote_mint)?;
    
    let (tuna_spot_position, _) = Pubkey::find_program_address(
        &[
            b"tuna_spot_position",
            executor_keypair.pubkey().as_ref(),
            whirlpool.as_ref(),
        ],
        &program_id,
    );
    
    info!("");
    info!("📋 Position: {}", tuna_spot_position);
    info!("   Authority: {}", executor_keypair.pubkey());
    info!("");

    // Check if position exists
    match rpc_client.get_account(&tuna_spot_position) {
        Ok(account) => {
            info!("✅ Position found");
            info!("   Rent to recover: {} SOL", account.lamports as f64 / 1e9);
        }
        Err(_) => {
            info!("❌ Position does not exist");
            return Ok(());
        }
    }

    let token_program_a = spl_token::ID;
    let token_program_b = spl_token::ID;
    
    let authority_ata_a = get_associated_token_address(&executor_keypair.pubkey(), &mint_a);
    let authority_ata_b = get_associated_token_address(&executor_keypair.pubkey(), &mint_b);
    let tuna_position_ata_a = get_associated_token_address(&tuna_spot_position, &mint_a);
    let tuna_position_ata_b = get_associated_token_address(&tuna_spot_position, &mint_b);

    // CloseTunaSpotPosition discriminator from IDL
    let discriminator: [u8; 8] = [4, 189, 171, 84, 110, 220, 10, 8];
    let data = discriminator.to_vec();

    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(executor_keypair.pubkey(), true),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new_readonly(token_program_a, false),
            AccountMeta::new_readonly(token_program_b, false),
            AccountMeta::new(tuna_spot_position, false),
            AccountMeta::new(tuna_position_ata_a, false),
            AccountMeta::new(tuna_position_ata_b, false),
        ],
        data,
    };

    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&executor_keypair.pubkey()),
        &[&executor_keypair],
        recent_blockhash,
    );

    info!("📤 Closing position...");
    match rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            info!("✅ Position closed successfully!");
            info!("🔗 TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", signature);
            info!("");
            info!("💰 Rent recovered to your wallet");
        }
        Err(e) => {
            info!("❌ Failed to close: {}", e);
        }
    }
    
    Ok(())
}
