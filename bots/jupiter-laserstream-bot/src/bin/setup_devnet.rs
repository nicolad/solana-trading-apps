use anyhow::Result;
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::{Keypair, Signer},
};
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔧 Devnet Wallet Setup");
    println!("=====================\n");

    // Check for wallet file
    let wallet_path = env::args()
        .nth(1)
        .unwrap_or_else(|| {
            println!("Usage: cargo run --bin setup_devnet <path-to-wallet.json>");
            println!("\nOr create a new wallet:");
            println!("  solana-keygen new --outfile ~/devnet-wallet.json");
            std::process::exit(1);
        });

    println!("📂 Loading wallet from: {}", wallet_path);
    
    // Read the wallet file (it's a JSON array of bytes)
    let wallet_contents = std::fs::read_to_string(&wallet_path)?;
    let wallet_bytes: Vec<u8> = serde_json::from_str(&wallet_contents)?;
    let keypair = Keypair::try_from(wallet_bytes.as_slice())?;
    
    println!("✅ Wallet loaded");
    println!("   Public Key: {}\n", keypair.pubkey());

    // Connect to devnet
    let rpc_url = "https://api.devnet.solana.com";
    let client = RpcClient::new_with_commitment(rpc_url.to_string(), CommitmentConfig::confirmed());
    
    println!("🔗 Connecting to devnet...");
    
    // Check balance
    match client.get_balance(&keypair.pubkey()) {
        Ok(balance) => {
            let sol_balance = balance as f64 / 1_000_000_000.0;
            println!("✅ Current balance: {:.4} SOL\n", sol_balance);
            
            if sol_balance < 0.1 {
                println!("⚠️  Low balance! Request airdrop:");
                println!("   solana airdrop 2 {} --url devnet\n", keypair.pubkey());
            }
        }
        Err(e) => {
            println!("❌ Failed to get balance: {}\n", e);
        }
    }

    // Get base58 private key for .env
    println!("📋 Configuration for .env:");
    println!("==========================");
    println!("EXECUTOR_PRIVATE_KEY={}", bs58::encode(keypair.to_bytes()).into_string());
    println!("RPC_URL=https://api.devnet.solana.com");
    println!("\n⚠️  IMPORTANT:");
    println!("   1. Jupiter on devnet has limited liquidity");
    println!("   2. Start with SMALL amounts (0.01 SOL)");
    println!("   3. Set TRADE_AMOUNT_USDC=1 (not 100)");
    println!("   4. Use DCA strategy for testing");
    
    Ok(())
}
