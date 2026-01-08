# Strategy Development Template

## What is a Strategy?

In NautilusTrader terminology, a **strategy** contains the trading logic and decision-making that determines WHEN and HOW to trade. Strategies compose one or more adapters to execute trades while implementing:

1. **Signal generation**: Market analysis, indicators, ML models
2. **Position management**: Entry/exit logic, position sizing
3. **Risk management**: Stop losses, take profits, exposure limits
4. **Execution logic**: Order routing, timing, fee optimization

## Technology Stack

| Component | Technology | Deployment |
|-----------|-----------|-----------|
| **Adapter** | Rust (Anchor) | On-chain (Solana) |
| **Strategy Logic** | Rust | Serverless (Vercel) |
| **API/Monitoring** | TypeScript | Serverless (Vercel) |

## Strategy vs Adapter

| Component | Responsibility | Where Logic Lives |
|-----------|---------------|-------------------|
| **Adapter** | HOW to interface with a protocol | On-chain (Rust/Anchor) |
| **Strategy** | WHEN and WHY to trade | Serverless (Rust on Vercel) |
| **API** | Web interface, monitoring | Serverless (TypeScript on Vercel) |

**Example**:

- Adapter: "Here's how to execute a Jupiter swap with safety checks"
- Strategy: "I want to swap 100 USDC → SOL every day at 3 PM if price < $150"
- API: "Show me trade history and current positions"

## Strategy Types

### 1. Serverless Strategies (Recommended)

**Implementation**: Rust serverless function on Vercel
**Execution**: Vercel Cron, webhooks, or manual triggers

**Pros**:

- No server maintenance
- Auto-scaling
- Built-in monitoring
- Cost-effective (pay per invocation)
- Type-safe with Rust
- Fast execution with Vercel Edge

**Cons**:

- Function timeout limits (60s on Pro, 900s on Enterprise)
- Stateless (use Vercel KV/Postgres for state)
- Cold start latency (minimal with Vercel)

**Example**: DCA strategy that executes on Vercel Cron schedule

### 2. On-Chain Composed Strategies

**Implementation**: Anchor program that CPIs into adapters
**Execution**: Keeper-triggered or permissionless

**Pros**:

- Fully trustless execution
- No off-chain infrastructure
- Can be permissionless

**Cons**:

- Limited computation (200k CU)
- Harder to test and optimize
- More complex development

**Example**: On-chain grid trading that composes multiple DEX adapters

### 3. Hybrid Strategies

**Implementation**: On-chain program + Vercel serverless orchestration
**Execution**: Serverless signals → on-chain execution

**Pros**:

- Complex analysis in Rust serverless
- Trustless execution on-chain
- Best of both worlds

**Cons**:

- Most complex architecture
- Higher development cost

**Example**: ML price prediction in Vercel → trigger on-chain arbitrage

## Strategy Directory Structure

```
strategies/<strategy-name>/
├── README.md                   # Strategy documentation
├── Cargo.toml                  # Rust workspace
├── vercel.json                 # Vercel configuration
├── api/                        # Vercel serverless endpoints
│   ├── execute.rs              # Main execution (Rust)
│   ├── backtest.rs             # Backtesting (Rust)
│   └── status.ts               # Monitoring (TypeScript)
├── src/                        # Core strategy logic
│   ├── lib.rs                  # Strategy implementation
│   ├── signals.rs              # Signal generation
│   ├── risk.rs                 # Risk management
│   └── config.rs               # Configuration
├── client/                     # Adapter SDK
│   └── jupiter_vault.rs        # Adapter client
├── backtests/                  # Historical testing
│   └── data/                   # Historical data
└── tests/                      # Unit/integration tests
```

## Quick Start

### 1. Create New Strategy

```bash
cd strategies
mkdir dca-strategy
cd dca-strategy

# Initialize Rust workspace
cargo init --lib
```

### 2. Configure Vercel

Create `vercel.json`:

```json
{
  "functions": {
    "api/execute.rs": {
      "runtime": "vercel-rust@4.0.0",
      "maxDuration": 60
    },
    "api/status.ts": {
      "runtime": "nodejs20.x"
    }
  },
  "crons": [
    {
      "path": "/api/execute",
      "schedule": "0 15 * * *"
    }
  ]
}
```

### 3. Implement Strategy

See examples below for DCA, Grid Trading, and Arbitrage patterns.

### 4. Deploy

```bash
vercel --prod
```

## Example: DCA Strategy (Complete)

### Project Structure

```
strategies/dca/
├── Cargo.toml
├── vercel.json
├── api/
│   ├── execute.rs
│   └── status.ts
├── src/
│   ├── lib.rs
│   └── config.rs
└── client/
    └── jupiter_vault.rs
```

### `Cargo.toml`

```toml
[package]
name = "dca-strategy"
version = "0.1.0"
edition = "2021"

[dependencies]
vercel_runtime = "1.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
solana-client = "1.18"
solana-sdk = "1.18"
anchor-client = "0.32"
bs58 = "0.5"
chrono = "0.4"

[[bin]]
name = "execute"
path = "api/execute.rs"

[lib]
name = "dca_strategy"
path = "src/lib.rs"
```

### `src/lib.rs`

```rust
use solana_sdk::pubkey::Pubkey;
use std::error::Error;

pub struct DCAConfig {
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount_per_trade: u64,
    pub interval_seconds: i64,
    pub slippage_bps: u64,
}

impl DCAConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            input_mint: std::env::var("INPUT_MINT")?.parse()?,
            output_mint: std::env::var("OUTPUT_MINT")?.parse()?,
            amount_per_trade: std::env::var("AMOUNT_PER_TRADE")?.parse()?,
            interval_seconds: std::env::var("INTERVAL_SECONDS")?.parse()?,
            slippage_bps: std::env::var("SLIPPAGE_BPS")?.parse()?,
        })
    }
}

pub fn should_trade_now(last_trade_ts: i64, interval: i64) -> bool {
    let now = chrono::Utc::now().timestamp();
    now - last_trade_ts >= interval
}

pub fn calculate_min_out(amount_in: u64, slippage_bps: u64) -> u64 {
    // Placeholder: In production, fetch price from Jupiter API
    // For now, allow slippage
    let slippage_factor = (10000 - slippage_bps) as f64 / 10000.0;
    (amount_in as f64 * slippage_factor) as u64
}

pub fn get_next_order_id() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}
```

### `api/execute.rs`

```rust
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{signature::Keypair, commitment_config::CommitmentConfig};
use dca_strategy::{DCAConfig, should_trade_now, calculate_min_out, get_next_order_id};

mod client;
use client::JupiterVaultClient;

#[derive(Deserialize)]
struct ExecuteRequest {
    #[serde(default)]
    dry_run: bool,
}

#[derive(Serialize)]
struct ExecuteResponse {
    success: bool,
    signature: Option<String>,
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let body: ExecuteRequest = if req.body().is_empty() {
        ExecuteRequest { dry_run: false }
    } else {
        serde_json::from_slice(req.body())
            .unwrap_or(ExecuteRequest { dry_run: false })
    };

    match execute_dca(body.dry_run).await {
        Ok(sig) => {
            let response = ExecuteResponse {
                success: true,
                signature: Some(sig.to_string()),
                message: "DCA trade executed successfully".to_string(),
            };
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response)?))?)
        }
        Err(e) => {
            let response = ExecuteResponse {
                success: false,
                signature: None,
                message: format!("Failed: {}", e),
            };
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response)?))?)
        }
    }
}

async fn execute_dca(dry_run: bool) -> Result<String, Box<dyn std::error::Error>> {
    // Load config
    let config = DCAConfig::from_env()?;
    
    // Check if should trade
    let last_trade_ts = get_last_trade_ts().await?;
    if !should_trade_now(last_trade_ts, config.interval_seconds) {
        return Err("Not time to trade yet".into());
    }

    // Calculate parameters
    let amount_in = config.amount_per_trade;
    let min_out = calculate_min_out(amount_in, config.slippage_bps);
    let order_id = get_next_order_id();

    if dry_run {
        return Ok(format!(
            "DRY RUN: amount_in={}, min_out={}, order_id={}",
            amount_in, min_out, order_id
        ));
    }

    // Initialize client
    let rpc = RpcClient::new_with_commitment(
        std::env::var("RPC_URL")?,
        CommitmentConfig::confirmed(),
    );
    let executor = load_keypair()?;
    let client = JupiterVaultClient::new(rpc, executor);

    // Execute trade
    let sig = client.execute_swap(
        config.input_mint,
        config.output_mint,
        amount_in,
        min_out,
        order_id,
    ).await?;

    // Save last trade time
    save_last_trade_ts(chrono::Utc::now().timestamp()).await?;

    Ok(sig.to_string())
}

async fn get_last_trade_ts() -> Result<i64, Box<dyn std::error::Error>> {
    // In production: use Vercel KV
    // For now: return 0 (always trade)
    Ok(0)
}

async fn save_last_trade_ts(_ts: i64) -> Result<(), Box<dyn std::error::Error>> {
    // In production: save to Vercel KV
    Ok(())
}

fn load_keypair() -> Result<Keypair, Box<dyn std::error::Error>> {
    let key_str = std::env::var("EXECUTOR_PRIVATE_KEY")?;
    let bytes = bs58::decode(key_str).into_vec()?;
    Ok(Keypair::from_bytes(&bytes)?)
}
```

### `client/jupiter_vault.rs`

```rust
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::error::Error;

pub struct JupiterVaultClient {
    rpc: RpcClient,
    executor: Keypair,
    program_id: Pubkey,
}

impl JupiterVaultClient {
    pub fn new(rpc: RpcClient, executor: Keypair) -> Self {
        let program_id = std::env::var("VAULT_PROGRAM_ID")
            .expect("VAULT_PROGRAM_ID must be set")
            .parse()
            .expect("Invalid program ID");

        Self {
            rpc,
            executor,
            program_id,
        }
    }

    pub async fn execute_swap(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount_in: u64,
        min_out: u64,
        client_order_id: u64,
    ) -> Result<Signature, Box<dyn Error>> {
        // Build instruction using Anchor client
        // This is a simplified example - in production use anchor-client
        
        // 1. Fetch Jupiter quote
        let jupiter_ix_data = self.fetch_jupiter_quote(
            input_mint,
            output_mint,
            amount_in,
        ).await?;

        // 2. Build vault execute_swap instruction
        // let ix = build_execute_swap_ix(...);

        // 3. Send transaction
        // let tx = Transaction::new_signed_with_payer(...);
        // let sig = self.rpc.send_and_confirm_transaction(&tx)?;

        // Placeholder
        Ok(Signature::default())
    }

    async fn fetch_jupiter_quote(
        &self,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount: u64,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        // Call Jupiter API to get quote and instruction data
        // https://quote-api.jup.ag/v6/quote
        todo!("Implement Jupiter API integration")
    }
}
```

### `api/status.ts`

```typescript
import type { VercelRequest, VercelResponse } from '@vercel/node';

export default async function handler(
  req: VercelRequest,
  res: VercelResponse,
) {
  try {
    // In production: fetch from Vercel KV or database
    const status = {
      healthy: true,
      lastExecution: new Date().toISOString(),
      errorCount: 0,
      uptime: process.uptime(),
    };

    return res.status(200).json(status);
  } catch (error) {
    return res.status(500).json({ 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
}
```

### `vercel.json`

```json
{
  "functions": {
    "api/execute.rs": {
      "runtime": "vercel-rust@4.0.0",
      "maxDuration": 60
    },
    "api/status.ts": {
      "runtime": "nodejs20.x"
    }
  },
  "crons": [
    {
      "path": "/api/execute",
      "schedule": "0 15 * * *"
    }
  ],
  "env": {
    "RPC_URL": "@rpc_url",
    "EXECUTOR_PRIVATE_KEY": "@executor_private_key",
    "VAULT_PROGRAM_ID": "@vault_program_id",
    "INPUT_MINT": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "OUTPUT_MINT": "So11111111111111111111111111111111111111112",
    "AMOUNT_PER_TRADE": "100000000",
    "INTERVAL_SECONDS": "86400",
    "SLIPPAGE_BPS": "50"
  }
}
```

## Vercel Deployment

### Set Environment Variables

```bash
# Add secrets
vercel env add RPC_URL production
vercel env add EXECUTOR_PRIVATE_KEY production
vercel env add VAULT_PROGRAM_ID production

# Or use .env.production
echo "RPC_URL=https://api.mainnet-beta.solana.com" > .env.production
# Note: Never commit private keys!
```

### Deploy

```bash
# Deploy to production
vercel --prod

# Deploy to preview
vercel

# View logs
vercel logs
```

### Cron Schedules

Common patterns:

```json
{
  "crons": [
    {
      "path": "/api/execute",
      "schedule": "0 */6 * * *"
    }
  ]
}
```

- `0 15 * * *` - Daily at 3 PM UTC
- `0 */6 * * *` - Every 6 hours
- `*/30 * * * *` - Every 30 minutes
- `0 0 * * 1` - Weekly on Monday at midnight

## Testing

### Local Development

```bash
# Install Vercel CLI
npm i -g vercel

# Run locally
vercel dev

# Test endpoint
curl http://localhost:3000/api/execute?dry_run=true
```

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_trade() {
        let last_trade = 0;
        let interval = 86400; // 1 day
        assert!(should_trade_now(last_trade, interval));
    }

    #[test]
    fn test_min_out_calculation() {
        let amount_in = 100_000_000; // 100 USDC
        let slippage_bps = 50; // 0.5%
        let min_out = calculate_min_out(amount_in, slippage_bps);
        assert!(min_out < amount_in);
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_execute_swap() {
    // Test with devnet
    std::env::set_var("RPC_URL", "https://api.devnet.solana.com");
    std::env::set_var("EXECUTOR_PRIVATE_KEY", "test_key");
    
    // Execute with dry_run
    match execute_dca(true).await {
        Ok(msg) => assert!(msg.contains("DRY RUN")),
        Err(e) => panic!("Test failed: {}", e),
    }
}
```

## Monitoring with TypeScript

### `api/trades.ts` - Trade History

```typescript
import type { VercelRequest, VercelResponse } from '@vercel/node';
import { kv } from '@vercel/kv';

export default async function handler(
  req: VercelRequest,
  res: VercelResponse,
) {
  try {
    const trades = await kv.lrange('trades', 0, 49); // Last 50 trades
    
    return res.status(200).json({
      count: trades.length,
      trades,
    });
  } catch (error) {
    return res.status(500).json({ 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
}
```

### `api/metrics.ts` - Performance Metrics

```typescript
import type { VercelRequest, VercelResponse } from '@vercel/node';
import { kv } from '@vercel/kv';

export default async function handler(
  req: VercelRequest,
  res: VercelResponse,
) {
  try {
    const metrics = {
      totalTrades: await kv.get('total_trades') || 0,
      successRate: await kv.get('success_rate') || 0,
      avgExecutionTime: await kv.get('avg_execution_time') || 0,
      totalVolume: await kv.get('total_volume') || 0,
    };
    
    return res.status(200).json(metrics);
  } catch (error) {
    return res.status(500).json({ 
      error: error instanceof Error ? error.message : 'Unknown error' 
    });
  }
}
```

## Resources

### Vercel

- [Vercel Rust Runtime](https://vercel.com/docs/functions/runtimes/rust)
- [Vercel Cron Jobs](https://vercel.com/docs/cron-jobs)
- [Vercel KV](https://vercel.com/docs/storage/vercel-kv)
- [Vercel Postgres](https://vercel.com/docs/storage/vercel-postgres)

### Solana

- [Solana Rust SDK](https://docs.rs/solana-sdk/)
- [Anchor Client](https://docs.rs/anchor-client/)
- [Jupiter API](https://station.jup.ag/docs/apis/swap-api)

### Testing

- [Tokio Testing](https://tokio.rs/tokio/topics/testing)
- [Rust Testing Guide](https://doc.rust-lang.org/book/ch11-00-testing.html)
