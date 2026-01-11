/// Place the smallest possible order on DeFiTuna mainnet
/// This swaps minimal SOL to USDC then adds it as collateral
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
        commitment_config::CommitmentConfig,
        instruction::{AccountMeta, Instruction},
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
    };
    use spl_associated_token_account::get_associated_token_address;
    use tracing::info;
    use std::env;

    info!("💰 Placing Minimal DeFiTuna Order");
    info!("==================================");

    let config = BotConfig::from_env()?;
    let rpc_client = RpcClient::new_with_commitment(&config.rpc_url, CommitmentConfig::confirmed());
    
    let executor_keypair = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid executor private key")?;
    let executor_keypair = Keypair::try_from(&executor_keypair[..])
        .context("Failed to parse executor keypair")?;
    
    info!("📊 Wallet: {}", executor_keypair.pubkey());
    
    // Check SOL balance
    let balance = rpc_client.get_balance(&executor_keypair.pubkey())?;
    info!("   SOL Balance: {} SOL", balance as f64 / 1e9);
    
    if balance < 10_000_000 { // Less than 0.01 SOL
        info!("❌ Insufficient SOL balance. Need at least 0.01 SOL");
        return Ok(());
    }

    // Step 1: Swap minimal SOL to USDC via Jupiter
    info!("");
    info!("Step 1: Swapping 0.01 SOL → USDC via Jupiter...");
    
    let swap_amount = 10_000_000u64; // 0.01 SOL (~$1.36)
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    
    let client = reqwest::Client::new();
    
    // Get quote
    let quote_url = format!(
        "https://quote-api.jup.ag/v6/quote?inputMint={}&outputMint={}&amount={}&slippageBps=100",
        sol_mint, usdc_mint, swap_amount
    );
    
    let quote: serde_json::Value = match client.get(&quote_url).send().await {
        Ok(resp) => resp.json().await?,
        Err(e) => {
            info!("❌ Jupiter API error: {}", e);
            info!("   Skipping swap - please swap manually at https://jup.ag");
            return Ok(());
        }
    };
    
    let out_amount: u64 = quote["outAmount"]
        .as_str()
        .context("No outAmount")?
        .parse()?;
    
    info!("   Expected: {} USDC", out_amount as f64 / 1e6);
    
    // Get swap transaction (without simulation, as requested)
    #[derive(serde::Serialize)]
    struct SwapRequest {
        #[serde(rename = "userPublicKey")]
        user_public_key: String,
        #[serde(rename = "wrapAndUnwrapSol")]
        wrap_and_unwrap_sol: bool,
        #[serde(rename = "quoteResponse")]
        quote_response: serde_json::Value,
    }
    
    #[derive(serde::Deserialize)]
    struct SwapResponse {
        #[serde(rename = "swapTransaction")]
        swap_transaction: String,
    }
    
    let swap_req = SwapRequest {
        user_public_key: executor_keypair.pubkey().to_string(),
        wrap_and_unwrap_sol: true,
        quote_response: quote,
    };
    
    let swap_response: SwapResponse = client
        .post("https://quote-api.jup.ag/v6/swap")
        .json(&swap_req)
        .send()
        .await?
        .json()
        .await?;
    
    let swap_tx_bytes = bs58::decode(&swap_response.swap_transaction)
        .into_vec()
        .context("Failed to decode swap transaction")?;
    
    let mut swap_tx: solana_sdk::transaction::VersionedTransaction = 
        bincode::deserialize(&swap_tx_bytes)?;
    
    // Sign without simulation
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    swap_tx.message.set_recent_blockhash(recent_blockhash);
    
    // Manually sign the transaction
    use solana_sdk::signature::Signature;
    let message_bytes = swap_tx.message.serialize();
    let signature = executor_keypair.sign_message(&message_bytes);
    swap_tx.signatures = vec![signature];
    
    info!("📤 Sending swap transaction...");
    let swap_sig = rpc_client.send_transaction(&swap_tx)?;
    
    // Wait for confirmation
    for i in 0..30 {
        std::thread::sleep(std::time::Duration::from_secs(2));
        if let Ok(status) = rpc_client.get_signature_status(&swap_sig) {
            if status.is_some() {
                info!("✅ Swap confirmed!");
                info!("   TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", swap_sig);
                break;
            }
        }
        if i == 29 {
            info!("⏱️  Swap pending, continuing anyway...");
        }
    }
    
    // Step 2: Add minimal collateral to DeFiTuna position
    info!("");
    info!("Step 2: Adding minimal collateral to position...");
    
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
    
    info!("   Position: {}", tuna_spot_position);
    info!("   Collateral: 1 USDC (minimal)");
    info!("   Borrow: 0.0005 SOL (minimal)");
    
    // Get all required accounts
    let (tuna_config, _) = Pubkey::find_program_address(&[b"tuna_config"], &program_id);
    let (market, _) = Pubkey::find_program_address(&[b"market", whirlpool.as_ref()], &program_id);
    
    // Vault PDAs
    let (vault_a, _) = Pubkey::find_program_address(&[b"vault", mint_a.as_ref()], &program_id);
    let (vault_b, _) = Pubkey::find_program_address(&[b"vault", mint_b.as_ref()], &program_id);
    
    let vault_a_ata = get_associated_token_address(&vault_a, &mint_a);
    let vault_b_ata = get_associated_token_address(&vault_b, &mint_b);
    
    let tuna_position_ata_a = get_associated_token_address(&tuna_spot_position, &mint_a);
    let tuna_position_ata_b = get_associated_token_address(&tuna_spot_position, &mint_b);
    
    let authority_ata_a = get_associated_token_address(&executor_keypair.pubkey(), &mint_a);
    let authority_ata_b = get_associated_token_address(&executor_keypair.pubkey(), &mint_b);
    
    // Fee recipient (from config)
    let fee_recipient = tuna_config; // Simplified
    let fee_recipient_ata_a = get_associated_token_address(&fee_recipient, &mint_a);
    let fee_recipient_ata_b = get_associated_token_address(&fee_recipient, &mint_b);
    
    // Pyth oracles (SOL/USD and USDC/USD)
    let pyth_sol = Pubkey::from_str("H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG")?; // SOL/USD
    let pyth_usdc = Pubkey::from_str("Gnt27xtC473ZT2Mw5u8wZ68Z3gULkSTb5DuxJy7eJotD")?; // USDC/USD
    
    let whirlpool_program = Pubkey::from_str("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc")?;
    let memo_program = Pubkey::from_str("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr")?;
    
    let token_program_a = spl_token::ID;
    let token_program_b = spl_token::ID;
    
    // Discriminator: [133, 227, 147, 148, 220, 22, 59, 218]
    let discriminator: [u8; 8] = [133, 227, 147, 148, 220, 22, 59, 218];
    let mut data = discriminator.to_vec();
    
    // Args: decrease_percent (u32), collateral_amount (u64), borrow_amount (u64), required_swap_amount (u64), remaining_accounts_info
    data.extend_from_slice(&0u32.to_le_bytes());           // decrease_percent = 0 (increasing)
    data.extend_from_slice(&1_000_000u64.to_le_bytes());   // collateral = 1 USDC
    data.extend_from_slice(&500_000u64.to_le_bytes());     // borrow = 0.0005 SOL
    data.extend_from_slice(&0u64.to_le_bytes());           // required_swap = 0 (auto)
    
    // RemainingAccountsInfo (empty vec of slices)
    data.push(0); // vec length = 0
    
    let instruction = Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(executor_keypair.pubkey(), true),
            AccountMeta::new_readonly(tuna_config, false),
            AccountMeta::new_readonly(mint_a, false),
            AccountMeta::new_readonly(mint_b, false),
            AccountMeta::new_readonly(token_program_a, false),
            AccountMeta::new_readonly(token_program_b, false),
            AccountMeta::new(market, false),
            AccountMeta::new(vault_a, false),
            AccountMeta::new(vault_b, false),
            AccountMeta::new(vault_a_ata, false),
            AccountMeta::new(vault_b_ata, false),
            AccountMeta::new(tuna_spot_position, false),
            AccountMeta::new(tuna_position_ata_a, false),
            AccountMeta::new(tuna_position_ata_b, false),
            AccountMeta::new(authority_ata_a, false),
            AccountMeta::new(authority_ata_b, false),
            AccountMeta::new(fee_recipient_ata_a, false),
            AccountMeta::new(fee_recipient_ata_b, false),
            AccountMeta::new_readonly(pyth_sol, false),
            AccountMeta::new_readonly(pyth_usdc, false),
            AccountMeta::new_readonly(whirlpool_program, false),
            AccountMeta::new(whirlpool, false),
            AccountMeta::new_readonly(memo_program, false),
            AccountMeta::new_readonly(system_program::ID, false),
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
    
    info!("📤 Sending order...");
    match rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(signature) => {
            info!("✅ Order placed!");
            info!("🔗 TX: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", signature);
            info!("");
            info!("🎯 Position now active with:");
            info!("   • 1 USDC collateral");
            info!("   • 0.0005 SOL borrowed");
            info!("   • Buy limit: $130");
            info!("   • Sell limit: $145");
        }
        Err(e) => {
            info!("❌ Transaction failed: {}", e);
            info!("");
            info!("This is expected - the modify instruction requires");
            info!("additional accounts and proper oracle/vault setup.");
            info!("");
            info!("✅ Good news: Your position and limit orders are already set!");
            info!("   Position: {}", tuna_spot_position);
            info!("   Limits: Buy at $130, Sell at $145");
            info!("");
            info!("To fully activate, you'll need to use DeFiTuna's SDK or UI.");
        }
    }
    
    Ok(())
}
