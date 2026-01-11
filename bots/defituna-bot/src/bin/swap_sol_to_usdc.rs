/// Swap SOL for USDC using Jupiter to prepare for DeFiTuna position
use anyhow::{Context, Result};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    hash::Hash,
    signature::{Keypair, Signer},
};
use std::str::FromStr;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
struct SwapRequest {
    #[serde(rename = "userPublicKey")]
    user_public_key: String,
    #[serde(rename = "wrapAndUnwrapSol")]
    wrap_and_unwrap_sol: bool,
    #[serde(rename = "quoteResponse")]
    quote_response: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SwapResponse {
    #[serde(rename = "swapTransaction")]
    swap_transaction: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    dotenv().ok();

    use defituna_bot::config::BotConfig;

    info!("🔄 Swapping SOL for USDC via Jupiter");
    info!("======================================");

    let config = BotConfig::from_env()?;
    let rpc_client = RpcClient::new_with_commitment(&config.rpc_url, CommitmentConfig::confirmed());
    
    let executor_keypair = bs58::decode(&config.executor_keypair)
        .into_vec()
        .context("Invalid executor private key")?;
    let executor_keypair = Keypair::try_from(&executor_keypair[..])
        .context("Failed to parse executor keypair")?;
    
    // Swap 0.05 SOL for USDC (50000000 lamports)
    let amount_lamports = 50_000_000u64; // 0.05 SOL
    
    info!("💰 Swapping {} SOL for USDC", amount_lamports as f64 / 1e9);
    info!("   Wallet: {}", executor_keypair.pubkey());
    
    // Get quote from Jupiter
    let quote_url = format!(
        "https://quote-api.jup.ag/v6/quote?inputMint={}&outputMint={}&amount={}&slippageBps=50",
        "So11111111111111111111111111111111111111112", // SOL
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // USDC
        amount_lamports
    );
    
    info!("📊 Getting quote from Jupiter...");
    let client = reqwest::Client::new();
    let quote: serde_json::Value = client
        .get(&quote_url)
        .send()
        .await?
        .json()
        .await?;
    
    let out_amount: u64 = quote["outAmount"]
        .as_str()
        .context("No outAmount in quote")?
        .parse()?;
    
    info!("   Expected output: {} USDC", out_amount as f64 / 1e6);
    
    // Get swap transaction
    info!("📝 Preparing swap transaction...");
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
    
    // Deserialize and sign transaction  
    let swap_tx_bytes = bs58::decode(&swap_response.swap_transaction)
        .into_vec()
        .context("Failed to decode swap transaction")?;
    
    let mut versioned_tx: solana_sdk::transaction::VersionedTransaction = 
        bincode::deserialize(&swap_tx_bytes)?;
    
    // Get recent blockhash for signing
    let recent_blockhash = rpc_client.get_latest_blockhash()?;
    
    // Sign the versioned transaction
    versioned_tx.sign(&[&executor_keypair], recent_blockhash);
    
    info!("📤 Sending swap transaction...");
    let signature = rpc_client.send_and_confirm_transaction_with_config(
        &versioned_tx,
        CommitmentConfig::confirmed(),
        Default::default(),
    )?;
    
    info!("✅ Swap successful!");
    info!("🔗 Transaction: https://explorer.solana.com/tx/{}?cluster=mainnet-beta", signature);
    info!("");
    info!("You can now open a DeFiTuna position with USDC as collateral");
    
    Ok(())
}
