# Project Summary

## Repository: Solana Trading Apps

A collection of **adapters** and **strategies** for algorithmic trading on Solana, following [NautilusTrader](https://github.com/nautechsystems/nautilus_trader) architectural patterns.

---

## ✅ What Has Been Created

### 1. Repository Structure

```
solana-trading-apps/
├── adapters/              # On-chain protocol interfaces
├── strategies/            # Trading logic implementations
├── examples/              # Reference implementations
├── shared/                # Common utilities and types
└── docs/                  # Documentation
    ├── guides/           # Development templates
    │   ├── ADAPTER_TEMPLATE.md
    │   ├── STRATEGY_TEMPLATE.md
    │   └── JUPITER_VAULT_TASKS.md
    └── QUICK_REFERENCE.md
```

### 2. Documentation

#### Main README ([README.md](../README.md))

- Project overview
- NautilusTrader-inspired architecture explanation
- Adapter and strategy listings
- Getting started guide
- Development philosophy
- Testing strategy
- Security considerations

#### Quick Reference ([docs/QUICK_REFERENCE.md](./QUICK_REFERENCE.md))

- Architecture diagram
- Directory structure
- Component responsibilities
- Development workflows
- Common patterns (code examples)
- Testing checklists
- FAQ

#### Adapter Template ([docs/guides/ADAPTER_TEMPLATE.md](./guides/ADAPTER_TEMPLATE.md))

- What is an adapter
- Adapter vs strategy comparison
- Directory structure
- Development checklist
- Common adapter patterns (swap, position management, order book)
- Naming conventions
- Integration guide

#### Strategy Template ([docs/guides/STRATEGY_TEMPLATE.md](./guides/STRATEGY_TEMPLATE.md))

- What is a strategy
- Strategy types (off-chain, on-chain, hybrid)
- Directory structure
- Development checklist
- Common strategy patterns (DCA, grid trading, arbitrage, momentum)
- Parameter optimization guide
- Backtesting workflow
- Risk disclosure template

#### Jupiter Vault Tasks ([docs/guides/JUPITER_VAULT_TASKS.md](./guides/JUPITER_VAULT_TASKS.md))

- 12 tasks × 15 minutes = 3-hour implementation guide
- Complete code for first adapter (Jupiter vault)
- Step-by-step with verification commands
- Testing strategy for Jupiter mainnet-only programs
- Deployment checklist

### 3. Git Configuration

- `.gitignore` with Solana/Anchor/Rust exclusions

---

## 🏗️ Architecture: NautilusTrader Pattern

### Core Concept

**Separation of Concerns:**

```
┌─────────────┐
│  STRATEGY   │  ← WHEN/WHY to trade (off-chain or composed on-chain)
└──────┬──────┘
       │ uses
       ▼
┌─────────────┐
│   ADAPTER   │  ← HOW to interface with protocol (on-chain Anchor program)
└──────┬──────┘
       │ calls
       ▼
┌─────────────┐
│  PROTOCOL   │  ← Solana DEX/protocol (Jupiter, Serum, Raydium, etc.)
└─────────────┘
```

### Key Differences from "Bot" Architecture

| Traditional "Bot" | NautilusTrader Pattern |
|-------------------|------------------------|
| Trading logic mixed with execution | Clear separation: adapter (execution) + strategy (logic) |
| Hard to reuse across strategies | Adapters reusable by multiple strategies |
| Testing requires full integration | Can test adapters and strategies independently |
| Difficult to compose | Easy to compose multiple adapters in one strategy |

### Example: DCA Implementation

**Without NautilusTrader pattern (monolithic bot):**

```
dca-bot/
└── src/
    └── dca_bot.rs   # Everything: Jupiter integration, caps, scheduling, logic
```

**With NautilusTrader pattern:**

```
adapters/jupiter-vault/         # HOW to safely use Jupiter
└── programs/jupiter-vault/
    └── src/
        └── lib.rs              # Swap CPI, safety checks, custody

strategies/dca/                 # WHEN to execute DCA
└── src/
    └── dca_strategy.py         # Scheduling, amount calculation, signal logic
```

**Benefits:**

- ✅ Jupiter vault adapter can be used by arbitrage, grid trading, momentum strategies
- ✅ DCA strategy can use multiple adapters (Jupiter for execution, Pyth for price feeds)
- ✅ Test adapter safety independently from strategy performance
- ✅ Backtest strategies without deploying on-chain programs

---

## 📦 Current Components

### Adapters (Planned)

| Adapter | Status | Purpose |
|---------|--------|---------|
| `jupiter-vault` | In Development | Gated access to Jupiter swap & DCA with on-chain risk controls |
| `serum-dex` | Planned | Interface to Serum/OpenBook order book |
| `raydium-amm` | Planned | Interface to Raydium AMM and liquidity pools |

### Strategies (Planned)

| Strategy | Status | Adapters Used | Type |
|----------|--------|---------------|------|
| `dca` | Planned | jupiter-vault | Off-chain (scheduled) |
| `arbitrage` | Planned | Multiple DEX adapters | Off-chain (event-driven) |
| `grid-trading` | Planned | Order book adapter | Hybrid (on-chain + off-chain) |
| `momentum` | Planned | DEX adapter | Off-chain (signal-based) |

---

## 🎯 Jupiter Vault Adapter (First Implementation)

### Overview

Maximum on-chain enforcement adapter for Jupiter protocols:

**Capabilities:**

- ✅ Swap execution with post-trade verification
- ✅ DCA position management
- ✅ Whitelist enforcement (input/output mints)
- ✅ Per-trade caps
- ✅ Daily caps with automatic reset
- ✅ Cooldown between trades
- ✅ Executor model (bot can trade but not withdraw)
- ✅ Replay protection (monotonic order IDs)
- ✅ Post-CPI balance delta verification

**Safety Guarantees:**

- Funds held in PDA (program-derived address)
- Owner can withdraw, executor cannot
- Even if executor is compromised: caps, whitelist, cooldown still enforced
- Post-transaction checks ensure Jupiter didn't drain more than expected

### Implementation Plan (12 × 15-min tasks)

See [JUPITER_VAULT_TASKS.md](./guides/JUPITER_VAULT_TASKS.md) for complete breakdown:

1. ✅ Create Anchor workspace
2. ✅ Define state accounts (VaultConfig, MintWhitelist, ExecutorConfig, DcaPosition)
3. ✅ Create error codes
4. ✅ Implement helper functions (authorize_trade, whitelist checks, etc.)
5. ✅ Initialize instruction
6. ✅ Admin functions (pause, unpause, set_params)
7. ✅ Whitelist management (add_mint, remove_mint)
8. ✅ Executor management (set_executor)
9. ✅ Deposit/withdraw (owner-only custody operations)
10. ✅ Execute Jupiter swap (CPI with safety checks)
11. ✅ Open DCA position (escrow tracking)
12. ✅ Close DCA position (refund reconciliation)

**Total estimated time:** 3 hours for core implementation

---

## 🔧 Development Workflow

### For Adapters

1. **Design** (30 min)
   - Identify protocol to interface with
   - Define risk controls needed
   - Plan PDA structure

2. **Implement** (3-4 hours)
   - Follow task breakdown (15-min increments)
   - Write tests alongside code
   - Verify at each step

3. **Test** (1-2 hours)
   - Unit tests for helpers
   - Integration tests for instructions
   - Mainnet-fork tests for CPIs

4. **Deploy**
   - Devnet → verify → Mainnet

### For Strategies

1. **Research** (varies)
   - Define hypothesis
   - Collect data requirements
   - Choose adapters

2. **Backtest** (1-2 weeks)
   - Implement strategy logic
   - Run on historical data
   - Optimize parameters

3. **Paper Trade** (2-4 weeks)
   - Deploy with test accounts
   - Monitor vs backtest

4. **Go Live**
   - Start small
   - Scale gradually

---

## 📊 Testing Strategy

### Adapter Testing

**Challenge:** Jupiter programs are mainnet-only

**Solution:** Clone programs to localnet:

```bash
solana-test-validator \
  --clone JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 \
  --clone DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M \
  --url https://api.mainnet-beta.solana.com
```

**Test Levels:**

1. Unit tests (pure Rust functions)
2. Integration tests (full instruction flows)
3. Mainnet-fork tests (CPI validation)
4. Devnet tests (end-to-end)

### Strategy Testing

**Backtesting:**

- Use historical OHLCV data
- Simulate adapter responses
- Measure: Sharpe ratio, max drawdown, win rate

**Paper Trading:**

- Live market data, simulated execution
- Verify performance matches backtest
- Minimum 30 days before live

**Live Trading:**

- Start with 1-5% of target capital
- Scale up over 90 days if metrics hold

---

## 🔐 Security Model

### Adapter Security (On-Chain)

**Enforced by Program:**

- ✅ PDA custody (owner cannot directly access escrowed funds)
- ✅ Whitelist checks (only approved mints)
- ✅ Caps (per-trade, daily)
- ✅ Cooldown (time between trades)
- ✅ Replay protection (monotonic order IDs)
- ✅ Post-CPI verification (balance deltas)
- ✅ Pause mechanism (emergency stop)

**Owner Privileges:**

- Can deposit/withdraw funds
- Can configure parameters
- Can enable/disable executors
- Can pause vault

**Executor Privileges:**

- Can execute trades (within caps/whitelist/cooldown)
- **Cannot** withdraw funds
- **Cannot** change configuration

**Threat Model:**

- ✅ Compromised executor key → Limited blast radius (caps + whitelist)
- ✅ Jupiter program bug → Balance delta checks provide secondary verification
- ✅ Replay attacks → Monotonic order IDs prevent duplicates
- ⚠️ Owner key compromise → Full control (use multisig for production)

### Strategy Security (Off-Chain)

**Strategies should:**

- Validate RPC responses
- Implement circuit breakers (max loss per day)
- Monitor for anomalies
- Log all decisions
- Have kill switch

---

## 📚 Resources

### NautilusTrader

- [Repository](https://github.com/nautechsystems/nautilus_trader)
- [Documentation](https://nautilustrader.io/docs/)
- [Architecture](https://nautilustrader.io/docs/latest/concepts/architecture/)

### Solana Development

- [Anchor Book](https://www.anchor-lang.com/docs)
- [Solana Cookbook](https://solanacookbook.com/)
- [Program Examples](https://github.com/solana-developers/program-examples)

### Jupiter Integration

- [Swap API Docs](https://dev.jup.ag/docs/swap/build-swap-transaction)
- [DCA CPI Example](https://github.com/jup-ag/dca-cpi-example)
- [Program IDs](https://dev.jup.ag/get-started)

---

## 🚀 Next Steps

### Immediate (Week 1)

1. **Implement Jupiter Vault Adapter**
   - Follow [JUPITER_VAULT_TASKS.md](./guides/JUPITER_VAULT_TASKS.md)
   - Complete all 12 tasks
   - Deploy to devnet

2. **Create First Strategy**
   - Simple DCA strategy using Jupiter vault
   - Off-chain Python client
   - Backtest on historical data

### Short-term (Month 1)

1. **Add Serum DEX Adapter**
   - Order placement with risk controls
   - Fill tracking
   - Integration tests

2. **Implement Arbitrage Strategy**
   - Use Jupiter + Serum adapters
   - Real-time opportunity detection
   - Paper trading

### Medium-term (Quarter 1)

1. **Add Raydium Adapter**
   - AMM swaps
   - LP position management

2. **Build Grid Trading Strategy**
   - Use Serum adapter
   - On-chain or hybrid implementation

3. **Production Hardening**
   - Comprehensive test suite
   - Monitoring & alerting
   - Security audit

---

## ⚠️ Important Notes

### Not Audited

None of these programs have been audited. Use at your own risk.

### Mainnet Testing

Jupiter programs are mainnet-only. Local testing requires cloning mainnet accounts.

### Executor Model

Executor keys should be treated as "hot wallets" with limited privileges. Owner keys should be kept offline (hardware wallet or multisig).

### Risk Disclosure

This is experimental software. Start small, test thoroughly, never risk more than you can afford to lose.

---

## 📞 Support

- **Issues**: GitHub Issues
- **Discussions**: GitHub Discussions
- **Documentation**: `docs/` folder

---

## 📄 License

MIT (see LICENSE file)

---

## 🙏 Acknowledgments

- **NautilusTrader**: For the architectural inspiration
- **Anchor**: For the Solana development framework
- **Jupiter**: For the swap and DCA protocols
- **Solana Foundation**: For the blockchain platform
