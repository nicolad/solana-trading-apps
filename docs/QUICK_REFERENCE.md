# Solana Trading Apps - Quick Reference

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│            VERCEL SERVERLESS LAYER                  │
│                                                     │
│  Strategies (Rust)     APIs (TypeScript)            │
│  ┌──────────────┐    ┌──────────────┐             │
│  │ DCA Cron     │    │ /api/status  │             │
│  │ Arbitrage    │    │ /api/trades  │             │
│  │ Grid Bot     │    │ /api/metrics │             │
│  └──────┬───────┘    └──────────────┘             │
└─────────┼──────────────────────────────────────────┘
          │ calls adapters
          ▼
┌─────────────────────────────────────────────────────┐
│              SOLANA ON-CHAIN LAYER                  │
│                                                     │
│  Adapters (Rust/Anchor)                             │
│  ┌──────────────┐  ┌──────────────┐               │
│  │ Jupiter      │  │ Serum DEX    │               │
│  │ Vault        │  │ Adapter      │               │
│  └──────┬───────┘  └──────┬───────┘               │
└─────────┼──────────────────┼───────────────────────┘
          │ CPI              │ CPI
          ▼                  ▼
┌─────────────────────────────────────────────────────┐
│              SOLANA PROTOCOLS                       │
│                                                     │
│  • Jupiter Aggregator                               │
│  • Jupiter DCA                                      │
│  • Serum/OpenBook                                   │
│  • Raydium AMM                                      │
└─────────────────────────────────────────────────────┘
```

## Directory Structure

```
solana-trading-apps/
├── adapters/           # On-chain programs (Rust/Anchor)
│   ├── jupiter_vault/  # Jupiter swap/DCA adapter
│   ├── serum-dex/      # Serum order book adapter
│   └── raydium-amm/    # Raydium AMM adapter
│
├── strategies/         # Serverless strategies (Rust on Vercel)
│   ├── dca/            # Vercel functions for DCA
│   │   ├── api/*.rs    # Rust serverless endpoints
│   │   └── api/*.ts    # TypeScript monitoring APIs
│   ├── arbitrage/      # Cross-DEX arbitrage
│   ├── grid-trading/   # Grid trading
│   └── momentum/       # Momentum strategies
│
├── shared/             # Utilities & infrastructure
│   ├── laserstream/    # Real-time data utility (Helius)
│   ├── websocket/      # WebSocket client/server
│   ├── types/          # Shared types
│   └── utils/          # Shared utilities
│
├── examples/           # Reference implementations
│   ├── jupiter-laserstream-bot/  # Bot using utility + adapter
│   └── backtest/       # Backtesting examples
│
└── docs/               # Documentation
    └── guides/         # Development guides
```

## Quick Start

### 1. Initialize New Adapter

```bash
cd adapters
anchor init my-adapter
cd my-adapter

# Follow the adapter template:
# docs/guides/ADAPTER_TEMPLATE.md
```

### 2. Initialize New Strategy

```bash
cd strategies
mkdir my-strategy
cd my-strategy

# Create strategy structure
mkdir -p src/{signals,risk} backtests tests

# Follow the strategy template:
# docs/guides/STRATEGY_TEMPLATE.md
```

### 3. Build and Test

```bash
# Build adapter
cd adapters/my-adapter
anchor build
anchor test

# Test strategy (backtesting)
cd ../../strategies/my-strategy
python backtests/backtest.py

# Test strategy (paper trading)
python src/strategy.py --paper-trading
```

## Component Responsibilities

| Component | Technology | What It Does | Deployment |
|-----------|-----------|-------------|------------|
| **Adapter** | Rust (Anchor) | • Provides protocol interface<br>• Enforces risk controls<br>• Manages custody<br>• Verifies transactions | Solana on-chain |
| **Strategy** | Rust | • Generate signals<br>• Execute trades via adapters<br>• Manage positions<br>• Risk management | Vercel serverless |
| **API** | TypeScript | • Web interface<br>• Monitoring<br>• Trade history<br>• Metrics | Vercel serverless |

## Development Workflow

### For Adapters (On-Chain)

1. **Design**
   - Define protocol interface
   - Identify risk controls needed
   - Plan PDA structure

2. **Implement**
   - Follow 15-min task breakdown
   - Write state accounts
   - Implement instructions
   - Add safety checks

3. **Test**
   - Unit tests for helpers
   - Integration tests
   - Mainnet-fork tests for CPIs

4. **Deploy**
   - Deploy to devnet
   - Verify with strategies
   - Deploy to mainnet

### For Strategies (Off-Chain)

1. **Research**
   - Define hypothesis
   - Identify data requirements
   - Choose adapters

2. **Backtest**
   - Collect historical data
   - Implement strategy
   - Run backtests
   - Optimize parameters

3. **Paper Trade**
   - Deploy with test accounts
   - Monitor execution
   - Verify performance

4. **Go Live**
   - Start small
   - Gradually increase size
   - Monitor continuously

## Key Principles

### Maximum On-Chain Enforcement

All adapters must enforce:

- ✅ PDA custody (no direct owner access)
- ✅ Whitelist checks
- ✅ Per-trade caps
- ✅ Daily limits
- ✅ Cooldown periods
- ✅ Replay protection
- ✅ Post-transaction verification

### Separation of Concerns

- **Adapters**: Security, custody, protocol interface
- **Strategies**: Logic, signals, optimization
- **Never**: Mix strategy logic in adapters

### NautilusTrader Alignment

We follow NautilusTrader's architecture:

- Clear adapter/strategy separation
- Standardized interfaces
- Composable components
- Testable design

## Common Patterns

### Adapter Pattern: Gated Swap

```rust
pub fn execute_swap(
    ctx: Context<ExecuteSwap>,
    input_mint: Pubkey,
    output_mint: Pubkey,
    amount_in: u64,
    min_out: u64,
    client_order_id: u64,
    ix_data: Vec<u8>,
) -> Result<()> {
    // 1. Whitelist check
    require_whitelisted(&ctx.accounts.whitelist, &input_mint)?;
    require_whitelisted(&ctx.accounts.whitelist, &output_mint)?;
    
    // 2. Risk checks (caps, cooldown, replay)
    authorize_trade(cfg, exec, amount_in, client_order_id)?;
    
    // 3. Snapshot balances
    let pre_source = ctx.accounts.source.amount;
    let pre_dest = ctx.accounts.dest.amount;
    
    // 4. CPI to protocol
    invoke_signed(&ix, &infos, signer_seeds)?;
    
    // 5. Verify deltas
    ctx.accounts.source.reload()?;
    ctx.accounts.dest.reload()?;
    
    let spent = pre_source - ctx.accounts.source.amount;
    let received = ctx.accounts.dest.amount - pre_dest;
    
    require!(spent <= amount_in, Error::SpentTooMuch);
    require!(received >= min_out, Error::ReceivedTooLittle);
    
    Ok(())
}
```

### Strategy Pattern: DCA

```python
class DCAStrategy:
    async def run(self):
        while self.is_running:
            # 1. Check conditions
            if not self.should_trade():
                await asyncio.sleep(60)
                continue
            
            # 2. Calculate parameters
            amount = self.calculate_amount()
            min_out = self.calculate_min_out()
            
            # 3. Execute through adapter
            await self.adapter.execute_swap(
                input_mint=self.config.input_mint,
                output_mint=self.config.output_mint,
                amount_in=amount,
                min_out=min_out,
                client_order_id=self.next_order_id()
            )
            
            # 4. Log and wait
            self.log_trade()
            await asyncio.sleep(self.config.interval)
```

## Testing Checklist

### Adapter Testing

- [ ] Unit tests for all helper functions
- [ ] Integration tests for each instruction
- [ ] Mainnet-fork tests for protocol CPIs
- [ ] Error condition tests
- [ ] Access control tests
- [ ] PDA derivation tests

### Strategy Testing

- [ ] Backtest on historical data
- [ ] Walk-forward validation
- [ ] Parameter sensitivity analysis
- [ ] Paper trading period (minimum 30 days)
- [ ] Risk scenario testing
- [ ] Live trading with small size

## Resources

### Documentation

- [Adapter Template](./guides/ADAPTER_TEMPLATE.md)
- [Strategy Template](./guides/STRATEGY_TEMPLATE.md)
- [Jupiter Vault Tasks](./guides/JUPITER_VAULT_TASKS.md)

### External

- [NautilusTrader Docs](https://nautilustrader.io/docs/)
- [Anchor Book](https://www.anchor-lang.com/docs)
- [Solana Cookbook](https://solanacookbook.com/)
- [Jupiter Developers](https://dev.jup.ag/)

## FAQ

**Q: Should my trading logic be in the adapter or strategy?**
A: Strategy. Adapters only provide a safe interface to protocols.

**Q: Can I mix on-chain and off-chain strategy logic?**
A: Yes! Use hybrid pattern: off-chain analysis → on-chain execution.

**Q: How do I test adapter CPIs locally?**
A: Clone mainnet programs to local validator using `solana-test-validator --clone`.

**Q: What's the difference from traditional bots?**
A: We separate protocol interface (adapter) from trading logic (strategy), following NautilusTrader architecture.

**Q: Can strategies use multiple adapters?**
A: Yes! Arbitrage strategies typically compose multiple DEX adapters.

**Q: Do I need an adapter for every protocol?**
A: Only if you need on-chain risk controls. Simple protocols can be used directly in strategies.
