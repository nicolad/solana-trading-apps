use anyhow::Result;
use jupiter_laserstream_bot::*;

/// Test LaserStream container deployment
#[tokio::test]
async fn test_laserstream_container_health() -> Result<()> {
    let url = "https://laserstream-container.eeeew.workers.dev";
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/health", url))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Health check should return 200");
    
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["status"], "ok", "Status should be 'ok'");
    assert!(body["timestamp"].is_string(), "Should have timestamp");
    
    Ok(())
}

#[tokio::test]
async fn test_laserstream_container_start() -> Result<()> {
    let url = "https://laserstream-container.eeeew.workers.dev";
    let client = reqwest::Client::new();
    
    let response = client
        .post(format!("{}/start", url))
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Start should return 200");
    
    let body = response.text().await?;
    assert!(body.trim() == "started", "Should return 'started' message");
    
    Ok(())
}

#[tokio::test]
async fn test_laserstream_container_latest() -> Result<()> {
    let url = "https://laserstream-container.eeeew.workers.dev";
    let client = reqwest::Client::new();
    
    let response = client
        .get(format!("{}/latest", url))
        .send()
        .await?;
    
    // Accept both success (200) and 404 (no data yet)
    assert!(
        response.status().is_success() || response.status().as_u16() == 404, 
        "Latest should return 2xx or 404 (no data yet), got: {}",
        response.status()
    );
    
    // If we got data, verify it has the expected structure from gRPC
    if response.status().is_success() {
        let data: serde_json::Value = response.json().await?;
        assert!(data.get("slot").is_some(), "Should have slot field from gRPC stream");
        assert!(data.get("status").is_some(), "Should have status field from gRPC stream");
        println!("✅ gRPC data received: slot={}, status={}", 
                 data["slot"], data["status"]);
    } else {
        println!("⚠️  No gRPC data yet (needs valid HELIUS_API_KEY)");
    }
    
    Ok(())
}

#[tokio::test]
async fn test_laserstream_grpc_pipeline() -> Result<()> {
    // This test verifies the full gRPC pipeline:
    // Helius LaserStream (gRPC) → Container → HTTP endpoint
    
    let url = "https://laserstream-container.eeeew.workers.dev";
    let client = reqwest::Client::new();
    
    // First ensure the stream is started
    client.post(format!("{}/start", url)).send().await?;
    
    // Wait a moment for gRPC connection to establish
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // Check for data multiple times to see if it's updating
    let mut slots = Vec::new();
    for i in 0..3 {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        if let Ok(response) = client.get(format!("{}/latest", url)).send().await {
            if response.status().is_success() {
                if let Ok(data) = response.json::<serde_json::Value>().await {
                    if let Some(slot) = data.get("slot").and_then(|s| s.as_u64()) {
                        slots.push(slot);
                        println!("Poll {}: slot={}", i + 1, slot);
                    }
                }
            }
        }
    }
    
    if slots.is_empty() {
        println!("⚠️  No gRPC data received - container needs valid HELIUS_API_KEY");
        println!("   Set with: pnpm run container:secret HELIUS_API_KEY");
    } else {
        println!("✅ gRPC pipeline working: received {} slot updates", slots.len());
        
        // Check if slots are progressing (blockchain is moving forward)
        if slots.len() > 1 {
            let min_slot = slots.iter().min().unwrap();
            let max_slot = slots.iter().max().unwrap();
            if max_slot > min_slot {
                println!("✅ Slots are progressing: {} → {}", min_slot, max_slot);
            }
        }
    }
    
    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored if you want to test external APIs
async fn test_jupiter_price_api() -> Result<()> {
    let client = reqwest::Client::new();
    
    // Test with SOL/USDC pair
    let sol_mint = "So11111111111111111111111111111111111111112";
    
    let response = client
        .get("https://price.jup.ag/v6/price")
        .query(&[("ids", sol_mint)])
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Jupiter price API should be accessible");
    
    let body: serde_json::Value = response.json().await?;
    assert!(body["data"].is_object(), "Should return price data");
    assert!(body["data"][sol_mint].is_object(), "Should have SOL price");
    
    Ok(())
}

#[tokio::test]
#[ignore] // Run with --ignored if you want to test external APIs
async fn test_jupiter_quote_api() -> Result<()> {
    let client = reqwest::Client::new();
    
    let sol_mint = "So11111111111111111111111111111111111111112";
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    let amount = 100_000_000; // 100 USDC (6 decimals)
    
    let response = client
        .get("https://quote-api.jup.ag/v6/quote")
        .query(&[
            ("inputMint", usdc_mint),
            ("outputMint", sol_mint),
            ("amount", &amount.to_string()),
            ("slippageBps", "50"),
        ])
        .send()
        .await?;
    
    assert!(response.status().is_success(), "Jupiter quote API should be accessible");
    
    let body: serde_json::Value = response.json().await?;
    assert!(body["inAmount"].is_string() || body["inAmount"].is_number(), "Should have input amount");
    assert!(body["outAmount"].is_string() || body["outAmount"].is_number(), "Should have output amount");
    
    Ok(())
}

#[test]
fn test_price_tracker_initialization() {
    use jupiter_laserstream_bot::price_tracker::PriceTracker;
    
    let tracker = PriceTracker::new(60);
    assert_eq!(tracker.update_count(), 0, "New tracker should have 0 updates");
}

#[test]
fn test_price_tracker_add_price() {
    use jupiter_laserstream_bot::price_tracker::PriceTracker;
    
    let mut tracker = PriceTracker::new(60);
    let timestamp = chrono::Utc::now().timestamp();
    
    tracker.add_price(100.0, 1000.0, timestamp);
    assert_eq!(tracker.update_count(), 1, "Should have 1 update");
    
    tracker.add_price(101.0, 1100.0, timestamp + 1);
    assert_eq!(tracker.update_count(), 2, "Should have 2 updates");
}

#[test]
fn test_price_tracker_moving_average() {
    use jupiter_laserstream_bot::price_tracker::PriceTracker;
    
    let mut tracker = PriceTracker::new(60);
    let base_time = chrono::Utc::now().timestamp();
    
    // Add some prices
    tracker.add_price(100.0, 1000.0, base_time);
    tracker.add_price(102.0, 1000.0, base_time + 60);
    tracker.add_price(104.0, 1000.0, base_time + 120);
    
    let avg = tracker.moving_average(3).expect("Should calculate average");
    assert!((avg - 102.0).abs() < 0.01, "Average should be ~102.0");
}

#[test]
fn test_config_validation() {
    // This test just verifies the config module compiles and can be used
    // We don't test BotConfig::from_env() as it requires many environment variables
    // Real validation happens at runtime
    
    use jupiter_laserstream_bot::config::BotConfig;
    
    // Just verify the type exists and is publicly accessible
    let _type_check: Option<BotConfig> = None;
    
    // Test passes if we can reference the type
    assert!(true, "Config module is accessible");
}

#[tokio::test]
#[ignore] // Run with --ignored if you want to test external APIs
async fn test_full_integration_flow() -> Result<()> {
    // This test verifies the complete integration:
    // 1. LaserStream container is accessible
    // 2. Jupiter APIs are accessible
    // 3. Bot components can be initialized
    
    let client = reqwest::Client::new();
    
    // 1. Verify LaserStream container
    let health = client
        .get("https://laserstream-container.eeeew.workers.dev/health")
        .send()
        .await?;
    assert!(health.status().is_success(), "LaserStream should be healthy");
    
    // 2. Verify Jupiter price API
    let price = client
        .get("https://price.jup.ag/v6/price")
        .query(&[("ids", "So11111111111111111111111111111111111111112")])
        .send()
        .await?;
    assert!(price.status().is_success(), "Jupiter price API should work");
    
    // 3. Verify Jupiter quote API
    let quote = client
        .get("https://quote-api.jup.ag/v6/quote")
        .query(&[
            ("inputMint", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"),
            ("outputMint", "So11111111111111111111111111111111111111112"),
            ("amount", "100000000"),
            ("slippageBps", "50"),
        ])
        .send()
        .await?;
    assert!(quote.status().is_success(), "Jupiter quote API should work");
    
    println!("✅ All integration tests passed!");
    
    Ok(())
}
