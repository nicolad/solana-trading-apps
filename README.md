# Solana Trading Apps

A collection of on-chain trading bots and vaults for Solana.

## Project Structure

Following NautilusTrader conventions:

```
solana-trading-apps/
├── adapters/               # On-chain protocol interfaces
│   ├── jupiter_vault/     # Jupiter swap execution adapter
│   ├── serum-dex/         # (Future) Serum DEX adapter
│   └── raydium/           # (Future) Raydium adapter
├── strategies/            # Trading logic (serverless/off-chain)
│   ├── dca/              # DCA strategies
│   ├── arbitrage/        # (Future) Arbitrage strategies
│   ├── grid-trading/     # (Future) Grid trading
│   └── momentum/         # (Future) Momentum strategies
├── shared/               # Utilities & infrastructure
│   ├── laserstream/      # Real-time data streaming utility
│   ├── websocket/        # WebSocket client/server
│   ├── types/            # Common types
│   └── utils/            # Shared utilities
├── bots/                 # Complete working bots
│   ├── jupiter-laserstream-bot/  # Bot using LaserStream + Jupiter
│   └── backtest/         # Backtesting examples
└── docs/
    └── guides/           # Implementation guides
```

## Adapters

### 1. Jupiter Vault (`adapters/jupiter-vault`)

**Status**: In Development

A maximum on-chain Jupiter vault with:

- PDA-owned custody (executor can trade but not withdraw)
- Hard on-chain risk controls (whitelist, caps, cooldown, daily limits)
- Jupiter CPI swap execution with post-trade balance verification
- DCA position management via Jupiter DCA program
- Executor/session-key model for bot trading

**Key Features**:

- ✅ On-chain whitelist enforcement
- ✅ Per-trade and daily caps
- ✅ Cooldown periods between trades
- ✅ Replay protection (monotonic order IDs)
- ✅ Post-swap balance delta verification
- ✅ Owner-only withdraw, executor-only trade
- 🚧 DCA position tracking and management

### 2. Serum DEX Adapter (`adapters/serum-dex`)

**Status**: Planned

On-chain adapter for Serum decentralized exchange.

### 3. Raydium Adapter (`adapters/raydium`)

**Status**: Planned

On-chain adapter for Raydium AMM.

## Strategies

### 1. DCA Strategies (`strategies/dca`)

**Status**: Planned

Dollar-cost averaging strategies using Jupiter vault adapter.

### 2. Arbitrage Strategies (`strategies/arbitrage`)

**Status**: Planned

Cross-DEX arbitrage strategies.

### 3. Grid Trading (`strategies/grid-trading`)

**Status**: Planned

Grid trading strategy implementations.

### 4. Momentum Strategies (`strategies/momentum`)

**Status**: Planned

Momentum-based trading strategies.

## Utilities

### 1. LaserStream (`shared/laserstream`, `laserstream-container`)

**Status**: Available

Real-time Solana data streaming utility using Helius LaserStream.

**Features**:

- Ultra-low latency data streaming
- Historical replay (up to 3000 slots)
- Multi-node reliability
- Auto-reconnection
- gRPC protocol with WebSocket broadcasting

**Deployments**:

- **Cloudflare Container**: [https://laserstream-container.eeeew.workers.dev/](https://laserstream-container.eeeew.workers.dev/)
  - Real-time slot updates via LaserStream gRPC SDK
  - Endpoints: `/health`, `/start`, `/latest`
  - Technology: Rust (Axum) + TypeScript (Hono Worker)
  - See [laserstream-container/README.md](laserstream-container/README.md) for details

**Use Case**: Get real-time price feeds from on-chain swaps for strategy signals.

### 2. WebSocket Utils (`shared/websocket`)

**Status**: Available

Reusable WebSocket client/server implementations.

**Features**:

- Auto-reconnection with exponential backoff
- Type-safe message passing
- Works with Cloudflare Workers
- Message buffering

**Use Case**: Connect strategies to data sources and broadcast signals.

## Component Separation

Following the **NautilusTrader pattern**, this repository separates:

### 1. **Adapters** (`adapters/`) - On-chain protocol interfaces

- Handle custody, permissions, and safety
- Execute transactions via CPI (Cross-Program Invocation)
- Each adapter wraps one Solana protocol (Jupiter, Serum, etc.)
- **Technology**: Rust/Anchor
- **Deployment**: Solana blockchain
- **Examples**: Jupiter vault, Serum DEX, Raydium AMM

### 2. **Utilities** (`shared/`) - Infrastructure & data sources

- Real-time data streaming (LaserStream)
- WebSocket connections
- Common types and helpers
- **Technology**: Rust
- **Deployment**: Local/serverless
- **Examples**: LaserStream client, WebSocket utils

### 3. **Strategies** (`strategies/`) - Trading logic

- Make trading decisions
- Calculate position sizes, entry/exit points
- Combine utilities for data + adapters for execution
- **Technology**: Rust (serverless) + TypeScript (APIs)
- **Deployment**: Vercel serverless
- **Examples**: DCA, arbitrage, grid trading

### 4. **Bots** (`bots/`) - Complete implementations

- Working combinations of utilities + strategies + adapters
- Best practices and patterns
- **Examples**: Jupiter bot with LaserStream

### Why This Separation?

| Component | Purpose | Location | Type |
|-----------|---------|----------|------|
| **Adapter** | Execute trades on-chain | `adapters/` | Protocol interface |
| **Utility** | Provide data/infrastructure | `shared/` | Data source |
| **Strategy** | Make trading decisions | `strategies/` | Business logic |
| **Bot** | Complete implementation | `bots/` | Application |

**Example Flow**: LaserStream (utility) → provides prices → Strategy → generates signal → Jupiter adapter → executes swap

See [Architecture Overview](docs/ARCHITECTURE_OVERVIEW.md) for detailed explanation.

### Prerequisites

- **Rust 1.83+** (for on-chain programs and containers)
- **Solana CLI 1.18+** (for Solana deployment)
- **Anchor 0.32.1+** (for Solana program framework)
- **Node.js 20+** (for TypeScript/JavaScript)
- **pnpm 9+** (package manager)
- **Docker Desktop** (for building containers)
- **Wrangler CLI** (for Cloudflare deployment)

### Quick Start

#### 1. Clone and Install

```bash
git clone <your-repo-url>
cd solana-trading-apps
pnpm install
```

#### 2. Configure Environment

```bash
# Copy environment template
cp .env.example .env

# Edit .env and fill in your secrets
# Required: HELIUS_API_KEY, PRIVATE_KEY
```

#### 3. Set Up Cloudflare Secrets

```bash
# Set Helius API key (get from https://dev.helius.xyz/)
echo "your_helius_api_key" | pnpm run container:secret HELIUS_API_KEY

# Or interactively
pnpm run container:secret HELIUS_API_KEY
# Then paste your key when prompted
```

#### 4. Deploy LaserStream Container

```bash
# Build, push, and deploy to Cloudflare
pnpm run container:deploy
```

#### 5. Monitor Deployment

```bash
# View container logs in real-time
pnpm run container:tail

# Test endpoints
curl https://laserstream-container.eeeew.workers.dev/health
```

#### 6. (Optional) Deploy On-chain Programs

```bash
# Build and deploy Jupiter vault adapter
cd adapters/jupiter-vault
anchor build
anchor test
anchor deploy --provider.cluster devnet
```

## Available Commands

### Root Commands

```bash
pnpm run dev                    # Start all services in dev mode
pnpm run build                  # Build all packages
pnpm run deploy                 # Deploy all services
pnpm run test                   # Run all tests
pnpm run lint                   # Lint all packages
pnpm run clean                  # Clean all build artifacts
```

### Container-Specific Commands

```bash
pnpm run container:dev          # Start container in dev mode
pnpm run container:build        # Build Docker image
pnpm run container:deploy       # Deploy to Cloudflare
pnpm run container:tail         # View live logs
pnpm run container:secret       # Set Cloudflare secrets (interactive)
```

See [DEPLOYMENT.md](DEPLOYMENT.md) for detailed deployment instructions.

## Development Philosophy

### NautilusTrader-Inspired Architecture

This project adapts NautilusTrader's architectural patterns to Solana:

- **Adapters**: On-chain programs that provide unified interfaces to DEXs/protocols
- **Strategies**: Off-chain or composed on-chain logic that uses adapters
- **Separation of Concerns**: Clear boundaries between execution infrastructure and strategy logic

### Maximum On-Chain Enforcement

All adapters follow the "maximum on-chain" philosophy:

1. **Custody**: All funds held in PDA-owned accounts
2. **Risk Controls**: Hard limits enforced on-chain (no off-chain trust)
3. **Executor Model**: Bot keys can trade but never withdraw
4. **Post-Trade Verification**: Balance deltas verified after every operation
5. **Replay Protection**: Monotonic order IDs prevent duplicate execution

### Component Separation

- **Adapters (on-chain programs)**: Rust Anchor programs with maximum safety guarantees
- **Strategies (serverless)**: Rust serverless functions on Vercel for strategy execution
- **APIs (serverless)**: TypeScript serverless functions on Vercel for web interfaces
- **Examples**: Reference implementations for backtesting and live trading
- **Shared utilities**: Common Rust types and helpers in `shared/`

## Testing Strategy

### Local Testing (Jupiter Programs)

Jupiter programs are mainnet-only. For local testing:

1. Clone program accounts to localnet using `solana-test-validator`
2. Use QuickNode's program forking guide
3. Or test against devnet/mainnet-fork

```bash
# Example: Clone Jupiter swap program to localnet
solana-test-validator \
  --clone JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 \
  --clone DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M \
  --url https://api.mainnet-beta.solana.com
```

## Security Considerations

### Audit Status

⚠️ **None of these programs have been audited**. Use at your own risk.

### Known Limitations

1. **Solana Programs Can't Run Autonomously**: Programs only execute when invoked by transactions. "Autonomy" requires keeper networks or scheduled transactions.

2. **DCA Escrow Trust**: When opening Jupiter DCA positions, funds move to Jupiter DCA escrow. Your program enforces safety at open/close, but not per-cycle.

3. **CPI Dependency Risk**: All bots depend on external program CPIs (Jupiter, etc.). Monitor for program upgrades.

## Contributing

Each bot should:

1. Be self-contained in `bots/<name>/`
2. Have comprehensive README with 15-min task breakdown
3. Follow the maximum on-chain philosophy
4. Include deployment checklist
5. Document all external dependencies

## Resources

### Jupiter Integration

- [Jupiter Developers Docs](https://dev.jup.ag/docs/swap/build-swap-transaction)
- [Jupiter CPI Examples](https://github.com/jup-ag/jupiter-cpi)
- [Jupiter DCA CPI](https://github.com/jup-ag/dca-cpi)
- [Jupiter Program IDs](https://dev.jup.ag/get-started)

### Anchor Framework

- [Anchor Documentation](https://www.anchor-lang.com/docs)
- [Account Space Reference](https://www.anchor-lang.com/docs/references/space)

### Solana Development

- [Solana Cookbook](https://solanacookbook.com/)
- [Solana Program Library](https://spl.solana.com/)

## License

MIT
