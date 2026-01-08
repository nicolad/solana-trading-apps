# Jupiter Trading Bot with LaserStream

Real-time trading bot that monitors Jupiter swaps via LaserStream and executes automated trades.

## Overview

This bot demonstrates:

1. **Real-time Price Monitoring**: Uses LaserStream to track all Jupiter swaps
2. **Price Analysis**: Calculates moving averages and trends from live data
3. **Automated Trading**: Executes DCA or momentum trades based on price signals
4. **Risk Management**: Implements position limits and cooldown periods

## Architecture

```
LaserStream (Helius)
    ↓ gRPC stream of Jupiter swaps
LaserStream Adapter (adapters/laserstream)
    ↓ WebSocket broadcast
Jupiter Trading Bot (this example)
    ↓ Trading signals
Jupiter Vault Adapter (adapters/jupiter_vault)
    ↓ Execute swaps on-chain
Jupiter Aggregator
```

## Features

- **Live Price Tracking**: Real-time SOL/USDC prices via LaserStream utility
- **Multiple Strategies**: DCA, momentum, mean reversion
- **On-Chain Execution**: Trades execute via Jupiter adapter (on-chain safety)
- **Historical Replay**: Catch up on missed data after disconnections
- **Metrics Dashboard**: Monitor performance and uptime
- **Vercel Deployment**: Run as serverless function with cron triggers

## Quick Start

### 1. Prerequisites

```bash
# Helius API key with LaserStream access
export HELIUS_API_KEY="your_key_here"

# Solana wallet for trading
export EXECUTOR_PRIVATE_KEY="your_base58_private_key"

# RPC endpoint
export RPC_URL="https://api.mainnet-beta.solana.com"
```

### 2. Install Dependencies

```bash
cd examples/jupiter-laserstream-bot
cargo build --release
```

### 3. Run Locally

```bash
# Start the bot
cargo run --release

# The bot will:
# 1. Connect to LaserStream
# 2. Monitor Jupiter swaps
# 3. Calculate price signals
# 4. Execute trades when conditions are met
```

### 4. Deploy to Vercel

```bash
# Deploy as serverless function
vercel --prod

# Configure cron job in vercel.json
# Bot will run on schedule (e.g., every hour)
```

## Configuration

### Environment Variables

```bash
# LaserStream utility (for price data)
HELIUS_API_KEY=your_api_key
LASERSTREAM_REGION=ewr  # Choose closest region
LASERSTREAM_NETWORK=mainnet

# Trading configuration
STRATEGY=momentum  # Options: dca, momentum, mean_reversion
TRADE_AMOUNT_USDC=100  # Amount per trade
MIN_PRICE_MOVEMENT=0.02  # 2% minimum movement to trade
COOLDOWN_MINUTES=60  # Wait between trades

# Risk limits
MAX_POSITION_SIZE=10000  # Max USDC exposure
MAX_SLIPPAGE_BPS=50  # 0.5% max slippage

# Jupiter Vault Program
VAULT_PROGRAM_ID=your_vault_program_id
VAULT_STATE_ADDRESS=your_vault_state_address
```

### Strategy Types

#### DCA (Dollar Cost Average)

Buys fixed amount on schedule regardless of price:

```bash
STRATEGY=dca
TRADE_AMOUNT_USDC=100
COOLDOWN_MINUTES=1440  # Once per day
```

#### Momentum

Buys when price is rising, sells when falling:

```bash
STRATEGY=momentum
MIN_PRICE_MOVEMENT=0.02  # Trade on 2% moves
LOOKBACK_MINUTES=60  # Compare to 1-hour average
```

#### Mean Reversion

Buys dips, sells peaks:

```bash
STRATEGY=mean_reversion
OVERSOLD_THRESHOLD=-0.05  # Buy at 5% below average
OVERBOUGHT_THRESHOLD=0.05  # Sell at 5% above average
```

## How It Works

### 1. Price Data Collection

```rust
// Connect to LaserStream adapter
let ws_client = WebSocketClient::connect("ws://localhost:8080").await?;

// Subscribe to price updates
ws_client.send(&WsMessage::Subscribe {
    channels: vec!["jupiter:SOL-USDC".to_string()],
}).await?;

// Receive real-time prices
while let Some(msg) = ws_client.receive::<WsMessage>().await? {
    match msg {
        WsMessage::PriceUpdate { price, volume, timestamp, .. } => {
            price_tracker.update(price, volume, timestamp);
        }
        _ => {}
    }
}
```

### 2. Signal Generation

```rust
// Momentum strategy example
fn should_buy(price_tracker: &PriceTracker) -> bool {
    let current = price_tracker.current_price();
    let avg_1h = price_tracker.moving_average(60);
    
    let change = (current - avg_1h) / avg_1h;
    
    change > 0.02 // Buy if price up 2% vs 1h average
}
```

### 3. Trade Execution

```rust
// Execute buy via Jupiter vault
let signature = execute_swap(
    &vault_client,
    SwapParams {
        input_mint: USDC_MINT,
        output_mint: SOL_MINT,
        amount_in: 100_000_000, // 100 USDC
        min_amount_out: calculate_min_out(current_price),
        slippage_bps: 50,
    }
).await?;

info!("Trade executed: {}", signature);
```

## Project Structure

```
jupiter-laserstream-bot/
├── Cargo.toml              # Dependencies
├── src/
│   ├── main.rs            # Entry point
│   ├── price_tracker.rs   # Price data management
│   ├── strategies/
│   │   ├── mod.rs
│   │   ├── dca.rs         # DCA implementation
│   │   ├── momentum.rs    # Momentum trading
│   │   └── mean_reversion.rs
│   ├── executor.rs        # Trade execution
│   └── metrics.rs         # Performance tracking
├── api/
│   ├── execute.rs         # Vercel serverless endpoint
│   └── status.ts          # Status dashboard
└── vercel.json            # Deployment config
```

## Example Output

```
[INFO] Connected to LaserStream adapter ws://localhost:8080
[INFO] Subscribed to channel: jupiter:SOL-USDC
[INFO] Price update: $98.45 (volume: 125.5 SOL)
[INFO] Moving average (1h): $97.82
[INFO] Price movement: +0.64% vs 1h average
[INFO] Signal: HOLD (threshold not met)
...
[INFO] Price update: $100.12 (volume: 234.2 SOL)
[INFO] Moving average (1h): $98.05
[INFO] Price movement: +2.11% vs 1h average
[INFO] Signal: BUY
[INFO] Executing swap: 100 USDC -> SOL
[INFO] Min expected output: 0.995 SOL
[INFO] Trade successful: signature 5Xj7...k3Qp
[INFO] Received: 1.002 SOL (slippage: -0.7%)
[INFO] New cooldown until: 2026-01-08 15:30:00
```

## Monitoring

### Metrics Endpoint

```bash
# Check bot status
curl http://localhost:3000/api/status

# Response:
{
  "status": "running",
  "uptime_seconds": 3600,
  "total_trades": 24,
  "successful_trades": 23,
  "failed_trades": 1,
  "success_rate": "95.83%",
  "total_volume_usdc": 2400,
  "average_slippage_bps": 12,
  "last_trade": "2026-01-08T14:30:00Z",
  "current_price": 100.12,
  "price_1h_avg": 98.05,
  "strategy": "momentum"
}
```

### Dashboard

Access the web dashboard at `http://localhost:3000/dashboard`:

- Real-time price chart
- Trade history
- Performance metrics
- Strategy parameters

## Cost Optimization

### LaserStream Usage

Monitoring all Jupiter swaps 24/7:

```
Estimated data: ~50 MB/hour
Cost: 500 credits/hour
Daily: ~$12
Monthly: ~$360

Optimization: Filter to specific token pairs
Reduced data: ~5 MB/hour
Cost: 50 credits/hour
Daily: ~$1.20
Monthly: ~$36
```

### Trade Frequency

```
DCA (daily): 1 trade/day = ~$3/month in tx fees
Momentum (hourly): 24 trades/day = ~$72/month
High frequency (5min): 288 trades/day = ~$864/month
```

## Advanced Features

### Multi-Pair Trading

Monitor and trade multiple pairs:

```rust
let pairs = vec![
    ("SOL", "USDC"),
    ("ETH", "USDC"),
    ("BTC", "USDC"),
];

for (base, quote) in pairs {
    let strategy = create_strategy(&base, &quote);
    strategies.push(strategy);
}
```

### Portfolio Rebalancing

Maintain target allocations:

```rust
let target = Portfolio {
    sol_percent: 40,
    eth_percent: 30,
    btc_percent: 20,
    usdc_percent: 10,
};

if current_allocation.drift_from(target) > 5.0 {
    rebalance_portfolio(current, target).await?;
}
```

### Backtesting

Test strategies on historical data:

```bash
# Replay historical LaserStream data
cargo run --features backtest -- \
  --start-slot 250000000 \
  --end-slot 250100000 \
  --strategy momentum
```

## Troubleshooting

### LaserStream Connection Issues

**Problem**: Can't connect to LaserStream

**Solutions**:

1. Verify API key: `echo $HELIUS_API_KEY`
2. Check plan limits in [Helius Dashboard](https://dashboard.helius.dev/)
3. Try different region endpoint
4. Ensure LaserStream adapter is running

### No Price Updates

**Problem**: Connected but no prices received

**Solutions**:

1. Check subscription filters in adapter
2. Verify Jupiter program ID is correct
3. Ensure mainnet/devnet matches your config
4. Check adapter logs for errors

### Trades Not Executing

**Problem**: Signals generated but no trades

**Solutions**:

1. Verify vault program is deployed
2. Check executor keypair has SOL for fees
3. Ensure vault has sufficient liquidity
4. Check cooldown hasn't been triggered

## Resources

- [LaserStream Documentation](https://docs.helius.dev/laserstream)
- [Jupiter Aggregator](https://station.jup.ag/)
- [Adapter Template](../../docs/guides/ADAPTER_TEMPLATE.md)
- [Strategy Template](../../docs/guides/STRATEGY_TEMPLATE.md)

## Next Steps

1. **Start with Devnet**: Test with devnet before mainnet
2. **Paper Trade**: Log trades without executing
3. **Monitor Performance**: Track for 1 week before live trading
4. **Optimize Parameters**: Tune thresholds based on results
5. **Scale Up**: Increase position sizes gradually

## Safety Features

- **Position Limits**: Maximum exposure per token
- **Cooldown Periods**: Prevent overtrading
- **Slippage Protection**: Cancel if price moves too much
- **Circuit Breakers**: Stop trading on errors
- **Dry Run Mode**: Test without real trades

## License

MIT
