# Vercel Deployment Guide

Complete guide for deploying Solana trading strategies as serverless functions on Vercel.

---

## Why Vercel?

- **Serverless**: No infrastructure to manage
- **Auto-scaling**: Handles traffic spikes automatically
- **Edge Network**: Fast global response times
- **Cron Jobs**: Built-in scheduled execution
- **Monitoring**: Integrated logging and metrics
- **Rust Support**: Native Rust runtime for performance
- **TypeScript APIs**: Easy monitoring dashboards

---

## Prerequisites

1. **Vercel Account**: [Sign up](https://vercel.com/signup)
2. **Vercel CLI**: `npm i -g vercel`
3. **Rust**: 1.75+
4. **Node.js**: 20+

---

## Project Structure

```
strategies/dca/
├── Cargo.toml              # Rust workspace
├── vercel.json             # Vercel configuration
├── .vercelignore           # Files to ignore
├── .env.production         # Production secrets
├── api/                    # Serverless functions
│   ├── execute.rs          # Main strategy (Rust)
│   ├── backtest.rs         # Backtesting (Rust)
│   ├── status.ts           # Status API (TypeScript)
│   └── trades.ts           # Trade history (TypeScript)
├── src/                    # Shared Rust code
│   ├── lib.rs
│   └── config.rs
├── client/                 # Adapter clients
│   └── jupiter_vault.rs
└── tests/
    └── integration_test.rs
```

---

## Step 1: Initialize Project

### 1.1 Create Strategy Directory

```bash
cd strategies
mkdir dca
cd dca
```

### 1.2 Initialize Rust Workspace

```bash
cargo init --lib
```

### 1.3 Update `Cargo.toml`

```toml
[package]
name = "dca-strategy"
version = "0.1.0"
edition = "2021"

[dependencies]
# Vercel runtime
vercel_runtime = "1.1"

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Solana
solana-client = "1.18"
solana-sdk = "1.18"
anchor-client = "0.32"

# Utilities
bs58 = "0.5"
chrono = "0.4"

# Each serverless function is a binary
[[bin]]
name = "execute"
path = "api/execute.rs"

[[bin]]
name = "backtest"
path = "api/backtest.rs"

[lib]
name = "dca_strategy"
path = "src/lib.rs"
```

---

## Step 2: Create Vercel Configuration

### 2.1 Create `vercel.json`

```json
{
  "version": 2,
  "name": "dca-strategy",
  "functions": {
    "api/execute.rs": {
      "runtime": "vercel-rust@4.0.0",
      "maxDuration": 60,
      "memory": 1024
    },
    "api/backtest.rs": {
      "runtime": "vercel-rust@4.0.0",
      "maxDuration": 300,
      "memory": 3008
    },
    "api/status.ts": {
      "runtime": "nodejs20.x",
      "maxDuration": 10
    },
    "api/trades.ts": {
      "runtime": "nodejs20.x",
      "maxDuration": 10
    }
  },
  "crons": [
    {
      "path": "/api/execute",
      "schedule": "0 15 * * *"
    }
  ],
  "env": {
    "RUST_LOG": "info"
  }
}
```

### 2.2 Create `.vercelignore`

```
target/
node_modules/
.env
.env.local
*.log
tests/
backtests/
```

---

## Step 3: Implement Serverless Functions

### 3.1 Main Execution Function (`api/execute.rs`)

```rust
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use dca_strategy::execute_trade;

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
    timestamp: i64,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    // Parse request
    let body: ExecuteRequest = if req.body().is_empty() {
        ExecuteRequest { dry_run: false }
    } else {
        serde_json::from_slice(req.body())?
    };

    // Execute strategy
    match execute_trade(body.dry_run).await {
        Ok(sig) => {
            let response = ExecuteResponse {
                success: true,
                signature: Some(sig),
                message: "Trade executed successfully".to_string(),
                timestamp: chrono::Utc::now().timestamp(),
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
                message: format!("Error: {}", e),
                timestamp: chrono::Utc::now().timestamp(),
            };
            
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&response)?))?)
        }
    }
}
```

### 3.2 Status API (`api/status.ts`)

```typescript
import type { VercelRequest, VercelResponse } from '@vercel/node';
import { kv } from '@vercel/kv';

export default async function handler(
  req: VercelRequest,
  res: VercelResponse,
) {
  try {
    const lastExecution = await kv.get('last_execution') || 'Never';
    const totalTrades = await kv.get('total_trades') || 0;
    const errorCount = await kv.get('error_count') || 0;
    const successRate = totalTrades > 0 
      ? ((totalTrades - errorCount) / totalTrades * 100).toFixed(2)
      : '0.00';

    const status = {
      healthy: errorCount < 5,
      lastExecution,
      totalTrades,
      errorCount,
      successRate: `${successRate}%`,
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

### 3.3 Trade History API (`api/trades.ts`)

```typescript
import type { VercelRequest, VercelResponse } from '@vercel/node';
import { kv } from '@vercel/kv';

interface Trade {
  signature: string;
  timestamp: number;
  amountIn: number;
  amountOut: number;
  inputMint: string;
  outputMint: string;
}

export default async function handler(
  req: VercelRequest,
  res: VercelResponse,
) {
  try {
    const limit = parseInt(req.query.limit as string) || 50;
    const trades = await kv.lrange<Trade>('trades', 0, limit - 1);
    
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

### 3.4 TypeScript Dependencies (`package.json`)

```json
{
  "name": "dca-strategy",
  "version": "1.0.0",
  "dependencies": {
    "@vercel/node": "^3.0.0",
    "@vercel/kv": "^1.0.0"
  },
  "devDependencies": {
    "@types/node": "^20.0.0",
    "typescript": "^5.0.0"
  }
}
```

---

## Step 4: Environment Variables

### 4.1 Required Variables

Create `.env.production` (DO NOT commit):

```bash
# Solana RPC
RPC_URL=https://api.mainnet-beta.solana.com

# Executor keypair (base58 encoded)
EXECUTOR_PRIVATE_KEY=your_base58_private_key_here

# Program IDs
VAULT_PROGRAM_ID=JupVau1t11111111111111111111111111111111111

# Strategy configuration
INPUT_MINT=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v  # USDC
OUTPUT_MINT=So11111111111111111111111111111111111111112   # SOL
AMOUNT_PER_TRADE=100000000  # 100 USDC (6 decimals)
INTERVAL_SECONDS=86400      # 24 hours
SLIPPAGE_BPS=50             # 0.5%
```

### 4.2 Add to Vercel

```bash
# Add secrets one by one
vercel env add RPC_URL production
vercel env add EXECUTOR_PRIVATE_KEY production
vercel env add VAULT_PROGRAM_ID production

# Or import from file
vercel env pull .env.production
```

### 4.3 Vercel KV (Optional for State)

```bash
# Create Vercel KV store
vercel link
vercel kv create

# Store will be automatically linked to your project
```

---

## Step 5: Deploy

### 5.1 Install Dependencies

```bash
# Install Node.js dependencies
npm install

# Build Rust (optional, Vercel will do this)
cargo build
```

### 5.2 Test Locally

```bash
# Start local dev server
vercel dev

# Test execute endpoint
curl http://localhost:3000/api/execute

# Test with dry run
curl -X POST http://localhost:3000/api/execute \
  -H "Content-Type: application/json" \
  -d '{"dry_run": true}'

# Test status endpoint
curl http://localhost:3000/api/status
```

### 5.3 Deploy to Preview

```bash
# Deploy to preview environment
vercel

# Your function will be available at:
# https://dca-strategy-abc123.vercel.app
```

### 5.4 Deploy to Production

```bash
# Deploy to production
vercel --prod

# Your function will be available at:
# https://dca-strategy.vercel.app (if you have a custom domain)
# or https://dca-strategy-username.vercel.app
```

---

## Step 6: Configure Cron Jobs

### 6.1 In `vercel.json`

```json
{
  "crons": [
    {
      "path": "/api/execute",
      "schedule": "0 15 * * *"
    }
  ]
}
```

### 6.2 Cron Schedule Formats

| Schedule | Description |
|----------|-------------|
| `0 15 * * *` | Daily at 3 PM UTC |
| `0 */6 * * *` | Every 6 hours |
| `*/30 * * * *` | Every 30 minutes |
| `0 0 * * 1` | Weekly on Monday at midnight |
| `0 9 * * 1-5` | Weekdays at 9 AM UTC |

### 6.3 Verify Cron Setup

```bash
# Check cron status in Vercel dashboard
vercel inspect dca-strategy --prod

# Or check logs
vercel logs --prod
```

---

## Step 7: Monitoring

### 7.1 Vercel Dashboard

Access at: <https://vercel.com/dashboard>

- **Logs**: Real-time function execution logs
- **Metrics**: Execution count, duration, errors
- **Cron Jobs**: View scheduled execution history

### 7.2 Custom Monitoring Dashboard

Create `api/dashboard.ts`:

```typescript
import type { VercelRequest, VercelResponse } from '@vercel/node';
import { kv } from '@vercel/kv';

export default async function handler(
  req: VercelRequest,
  res: VercelResponse,
) {
  const [
    lastExecution,
    totalTrades,
    errorCount,
    totalVolume,
    recentTrades
  ] = await Promise.all([
    kv.get('last_execution'),
    kv.get('total_trades'),
    kv.get('error_count'),
    kv.get('total_volume'),
    kv.lrange('trades', 0, 9)
  ]);

  const html = `
    <!DOCTYPE html>
    <html>
      <head>
        <title>DCA Strategy Dashboard</title>
        <style>
          body { font-family: Arial, sans-serif; margin: 40px; }
          .metric { padding: 20px; margin: 10px 0; background: #f5f5f5; border-radius: 8px; }
          .metric h3 { margin: 0 0 10px 0; color: #333; }
          .metric p { margin: 0; font-size: 24px; font-weight: bold; color: #007bff; }
          table { width: 100%; border-collapse: collapse; margin-top: 20px; }
          th, td { padding: 12px; text-align: left; border-bottom: 1px solid #ddd; }
          th { background-color: #007bff; color: white; }
        </style>
      </head>
      <body>
        <h1>DCA Strategy Dashboard</h1>
        
        <div class="metric">
          <h3>Last Execution</h3>
          <p>${lastExecution || 'Never'}</p>
        </div>
        
        <div class="metric">
          <h3>Total Trades</h3>
          <p>${totalTrades || 0}</p>
        </div>
        
        <div class="metric">
          <h3>Success Rate</h3>
          <p>${totalTrades > 0 ? ((1 - errorCount / totalTrades) * 100).toFixed(2) : 0}%</p>
        </div>
        
        <div class="metric">
          <h3>Total Volume</h3>
          <p>$${(totalVolume / 1e6 || 0).toFixed(2)}</p>
        </div>
        
        <h2>Recent Trades</h2>
        <table>
          <tr>
            <th>Signature</th>
            <th>Time</th>
            <th>Amount In</th>
            <th>Amount Out</th>
          </tr>
          ${(recentTrades || []).map(t => `
            <tr>
              <td>${t.signature.substring(0, 8)}...</td>
              <td>${new Date(t.timestamp * 1000).toLocaleString()}</td>
              <td>${(t.amountIn / 1e6).toFixed(2)} USDC</td>
              <td>${(t.amountOut / 1e9).toFixed(4)} SOL</td>
            </tr>
          `).join('')}
        </table>
      </body>
    </html>
  `;

  res.setHeader('Content-Type', 'text/html');
  return res.status(200).send(html);
}
```

Access at: `https://your-deployment.vercel.app/api/dashboard`

---

## Step 8: Security Best Practices

### 8.1 Secure Environment Variables

```bash
# Never commit .env files
echo ".env*" >> .gitignore

# Use Vercel Secrets
vercel env add SECRET_NAME production
```

### 8.2 API Rate Limiting

```typescript
// api/middleware/rate-limit.ts
import { NextRequest } from 'next/server';

const rateLimiter = new Map<string, { count: number; resetTime: number }>();

export function rateLimit(req: NextRequest, limit: number = 10, windowMs: number = 60000): boolean {
  const ip = req.ip || 'unknown';
  const now = Date.now();
  
  const record = rateLimiter.get(ip);
  
  if (!record || now > record.resetTime) {
    rateLimiter.set(ip, { count: 1, resetTime: now + windowMs });
    return true;
  }
  
  if (record.count >= limit) {
    return false;
  }
  
  record.count++;
  return true;
}
```

### 8.3 Authentication

```typescript
// api/middleware/auth.ts
export function verifyApiKey(req: VercelRequest): boolean {
  const apiKey = req.headers['x-api-key'];
  return apiKey === process.env.API_KEY;
}

// Usage in endpoint
export default async function handler(req: VercelRequest, res: VercelResponse) {
  if (!verifyApiKey(req)) {
    return res.status(401).json({ error: 'Unauthorized' });
  }
  // ... rest of handler
}
```

---

## Step 9: Testing

### 9.1 Local Testing

```bash
# Start local server
vercel dev

# Test endpoints
curl http://localhost:3000/api/execute
curl http://localhost:3000/api/status
```

### 9.2 Integration Tests

```bash
# Run Rust tests
cargo test

# Run specific test
cargo test test_execute_trade
```

### 9.3 Load Testing

```bash
# Install artillery
npm install -g artillery

# Create test config
cat > artillery.yml <<EOF
config:
  target: 'https://your-deployment.vercel.app'
  phases:
    - duration: 60
      arrivalRate: 10
scenarios:
  - flow:
      - get:
          url: '/api/status'
EOF

# Run load test
artillery run artillery.yml
```

---

## Step 10: Troubleshooting

### Common Issues

#### 1. Function Timeout

**Error**: "Function execution timed out"

**Solution**: Increase `maxDuration` in `vercel.json`:

```json
{
  "functions": {
    "api/execute.rs": {
      "maxDuration": 300
    }
  }
}
```

#### 2. Memory Limit

**Error**: "Function exceeded memory limit"

**Solution**: Increase memory:

```json
{
  "functions": {
    "api/execute.rs": {
      "memory": 3008
    }
  }
}
```

#### 3. Cold Starts

**Issue**: First request is slow

**Solutions**:

- Use Vercel Pro for faster cold starts
- Keep functions warm with periodic pings
- Minimize dependencies

#### 4. Environment Variables Not Found

**Error**: "environment variable not found: RPC_URL"

**Solution**:

```bash
# Verify env vars are set
vercel env ls

# Pull env vars to local
vercel env pull
```

---

## Resources

- [Vercel Documentation](https://vercel.com/docs)
- [Vercel Rust Runtime](https://vercel.com/docs/functions/runtimes/rust)
- [Vercel Cron Jobs](https://vercel.com/docs/cron-jobs)
- [Vercel KV](https://vercel.com/docs/storage/vercel-kv)
- [Vercel CLI Reference](https://vercel.com/docs/cli)

---

## Next Steps

1. **Add Monitoring**: Integrate with Datadog or Sentry
2. **Add Alerts**: Use Vercel webhooks to send alerts on failures
3. **Optimize Costs**: Review function execution times and memory usage
4. **Add More Strategies**: Deploy arbitrage, grid trading, etc.
5. **Create Dashboard**: Build a React dashboard for live monitoring
