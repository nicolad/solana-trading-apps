# Jupiter LaserStream Trading Bot

A real-time Solana trading bot that uses LaserStream (via Cloudflare Containers) for slot updates and executes trades through the Jupiter vault adapter.

## Architecture

```
LaserStream Container → HTTP polling → Jupiter Bot → Strategy Logic → Jupiter Vault → On-chain Execution
```

### Components

1. **LaserStream Container** (Cloudflare): Real-time Solana slot updates via Helius gRPC
2. **Jupiter Bot** (This project): Polls container, analyzes data, generates signals
3. **Trading Strategies**: DCA, Momentum, Mean Reversion
4. **Jupiter Vault**: On-chain adapter for safe trade execution

## Setup

### Prerequisites

- **LaserStream Container**: Must be deployed and running
  - URL: `https://laserstream-container.eeeew.workers.dev/`
  - **⚠️ IMPORTANT**: Set a valid Helius API key first!
  - From repo root: `pnpm run container:secret HELIUS_API_KEY`

- **Helius API Key**: Get from [https://dev.helius.xyz/](https://dev.helius.xyz/) (free tier available)

- **Solana Wallet**: For executing trades (devnet for testing)

### Configuration

1. **Copy environment template**:

```bash
cp .env.example .env
```

1. **Edit `.env`**:

```bash
# LaserStream Container (already deployed)
LASERSTREAM_CONTAINER_URL=https://laserstream-container.eeeew.workers.dev
POLL_INTERVAL_SECONDS=2

# Trading pair
BASE_TOKEN=SOL
QUOTE_TOKEN=USDC

# Strategy
STRATEGY=momentum  # or: dca, mean_reversion
TRADE_AMOUNT_USDC=100

# Solana (devnet)
RPC_URL=https://api.devnet.solana.com
EXECUTOR_PRIVATE_KEY=your_devnet_wallet_key

# Jupiter vault (deploy first)
VAULT_PROGRAM_ID=your_program_id
VAULT_STATE_ADDRESS=your_vault_address
```

## Running the Bot

### 1. Verify LaserStream Container

First, ensure the LaserStream container is working:

```bash
# From repo root
pnpm run container:tail
```

You should see:

- ✅ `INFO laserstream_container: listening on 0.0.0.0:8080`
- ✅ Slot updates streaming (not authentication errors)

If you see "Missing or invalid api key" errors:

```bash
# Set valid Helius API key
echo "your_helius_api_key" | pnpm run container:secret HELIUS_API_KEY

# Or interactively
pnpm run container:secret HELIUS_API_KEY
```

### 2. Build the Bot

```bash
cargo build --release
```

### 3. Run the Bot

```bash
cargo run --release
```

### 4. Monitor

The bot will:

1. Connect to LaserStream container
2. Poll for slot updates every 2 seconds
3. Track price data (placeholder - integrate with Jupiter API)
4. Generate trading signals based on strategy
5. Execute trades through Jupiter vault

## Current Integration Status

### ✅ Completed

- LaserStream container client (`src/laserstream_client.rs`)
- HTTP polling for slot updates
- Configuration management
- Strategy framework (DCA, Momentum, Mean Reversion)
- Trade executor integration

### 🚧 TODO

- [ ] **Jupiter Price Integration**: Currently using placeholder prices
  - Parse swap events from slot data
  - Query Jupiter API for real-time prices
  - Update `price_tracker` with actual price data

- [ ] **Deploy Jupiter Vault**: On-chain program for safe trade execution
  - Deploy to devnet
  - Update `VAULT_PROGRAM_ID` and `VAULT_STATE_ADDRESS`

- [ ] **Testing**: End-to-end testing with real devnet data

## How It Works

### LaserStream Integration

```rust
// 1. Connect to container
let laserstream = LaserStreamClient::new(&config.laserstream_url);

// 2. Health check
laserstream.health_check().await?;

// 3. Start subscription
laserstream.start().await?;

// 4. Poll for updates
loop {
    if let Some(update) = laserstream.get_latest().await? {
        // Process slot update
        handle_slot_update(update);
    }
    tokio::time::sleep(poll_interval).await;
}
```

### Strategy Flow

1. **Slot Update** → LaserStream container receives new slot
2. **Price Extraction** → Parse swap events or query Jupiter API
3. **Price Tracking** → Update moving averages, volatility, volume
4. **Signal Generation** → Strategy analyzes price data
5. **Trade Execution** → If signal generated, execute via Jupiter vault
6. **Cooldown** → Wait before next trade

## Development

### Add a New Strategy

1. Create `src/strategies/your_strategy.rs`:

```rust
use super::{Signal, Strategy};
use crate::price_tracker::PriceTracker;

pub struct YourStrategy {
    // Strategy parameters
}

impl Strategy for YourStrategy {
    fn generate_signal(&self, tracker: &PriceTracker) -> Option<Signal> {
        // Your strategy logic
        None
    }
}
```

1. Register in `src/strategies/mod.rs`

2. Update `config.rs` to support your strategy name

### Testing Without Valid API Key

The LaserStream container will still respond to requests without a valid Helius key, but won't receive real slot data:

- `/health` → Always works
- `/start` → Returns "started" (but gRPC connection fails)
- `/latest` → Returns "no data yet"

For full testing, you need a valid Helius API key.

## Troubleshooting

**Bot can't connect to container**:

- Verify URL: `https://laserstream-container.eeeew.workers.dev/`
- Test health: `curl https://laserstream-container.eeeew.workers.dev/health`

**No slot updates**:

- Check container logs: `pnpm run container:tail`
- If you see auth errors, set valid key: `pnpm run container:secret HELIUS_API_KEY`
- Verify key works at [https://dev.helius.xyz/](https://dev.helius.xyz/)

**Placeholder prices**:

- This is expected! Jupiter price integration is TODO
- Real implementation would query Jupiter API or parse swap events

## Resources

- [LaserStream Container](../../laserstream-container/README.md)
- [Helius LaserStream Docs](https://docs.helius.dev/laserstream/overview)
- [Jupiter API](https://station.jup.ag/docs/apis/swap-api)
- [Solana Devnet Faucet](https://faucet.solana.com/)
