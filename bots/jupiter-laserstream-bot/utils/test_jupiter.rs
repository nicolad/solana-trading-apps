use anyhow::Result;
use dotenv::dotenv;
use tracing::{info, error};

use jupiter_laserstream_bot::jupiter_client::JupiterClient;
use jupiter_laserstream_bot::swap_parser::get_token_decimals;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load environment variables
    dotenv().ok();
    
    info!("🧪 Testing Jupiter Integration");
    info!("================================");
    
    // Test 1: Jupiter Client Direct
    info!("\n📊 Test 1: Jupiter Price Client");
    let jupiter_client = JupiterClient::new();
    
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    
    match jupiter_client.get_price(sol_mint, usdc_mint).await {
        Ok(price) => {
            info!("✅ SOL/USDC Price: ${:.4}", price);
        }
        Err(e) => {
            error!("❌ Failed to fetch price: {}", e);
        }
    }
    
    // Test 2: Get Quote
    info!("\n📊 Test 2: Jupiter Quote");
    let amount = 100_000_000; // 0.1 SOL
    match jupiter_client.get_quote(sol_mint, usdc_mint, amount, 50).await {
        Ok(quote) => {
            let sol_decimals = get_token_decimals(sol_mint);
            let usdc_decimals = get_token_decimals(usdc_mint);
            
            let in_amount: u64 = quote.in_amount.parse().unwrap_or(0);
            let out_amount: u64 = quote.out_amount.parse().unwrap_or(0);
            
            let in_human = in_amount as f64 / 10_f64.powi(sol_decimals as i32);
            let out_human = out_amount as f64 / 10_f64.powi(usdc_decimals as i32);
            
            info!("✅ Quote: {:.4} SOL → {:.4} USDC", in_human, out_human);
            info!("   Price Impact: {}%", quote.price_impact_pct);
            info!("   Route: {} swaps", quote.route_plan.len());
        }
        Err(e) => {
            error!("❌ Failed to get quote: {}", e);
        }
    }
    
    // Test 3: Multiple Token Pairs
    info!("\n📊 Test 3: Multiple Token Pairs");
    let pairs = vec![
        ("SOL", sol_mint, usdc_mint),
        ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", sol_mint),
        ("JUP", "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN", usdc_mint),
    ];
    
    for (name, base, quote) in pairs {
        match jupiter_client.get_price(base, quote).await {
            Ok(price) => {
                info!("✅ {}: ${:.6}", name, price);
            }
            Err(e) => {
                error!("❌ {} failed: {}", name, e);
            }
        }
        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    info!("\n✅ All tests completed!");
    
    Ok(())
}
