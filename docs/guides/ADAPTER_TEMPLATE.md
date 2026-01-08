# Adapter Development Template

## What is an Adapter?

In NautilusTrader terminology, an **adapter** provides a unified interface to an external trading venue, exchange, or protocol. For Solana on-chain trading, an adapter is typically an Anchor program that:

1. **Abstracts protocol complexity**: Provides a clean, standardized interface to underlying DEX/protocol
2. **Enforces risk controls**: Implements hard on-chain safety constraints
3. **Enables composability**: Can be composed with other adapters in strategies
4. **Maintains custody**: Manages funds with PDA-owned accounts

## Adapter vs Strategy

| Component | Purpose | Implementation | Location |
|-----------|---------|----------------|----------|
| **Adapter** | Interface to protocol/DEX | On-chain Anchor program (Rust) | `adapters/<name>/` |
| **Strategy** | Trading logic/behavior | Off-chain client or composed program | `strategies/<name>/` |

**Example**:

- `adapters/jupiter-vault/`: On-chain program providing gated access to Jupiter swaps/DCA
- `strategies/dca/`: Off-chain bot that uses Jupiter vault adapter to execute DCA strategy

## Adapter Directory Structure

```
adapters/
└── <adapter-name>/
    ├── README.md                   # Adapter documentation
    ├── Anchor.toml                 # Anchor configuration
    ├── Cargo.toml                  # Workspace manifest
    ├── programs/
    │   └── <adapter-name>/
    │       ├── Cargo.toml          # Program dependencies
    │       └── src/
    │           ├── lib.rs          # Main program entry
    │           ├── state.rs        # State accounts
    │           ├── errors.rs       # Custom errors
    │           ├── utils.rs        # Helper functions
    │           └── instructions/   # (Optional) Modular instructions
    ├── tests/                      # Integration tests
    └── client/                     # (Optional) TypeScript/Rust client SDK
```

## Adapter Checklist

### Core Requirements

- [ ] PDA-owned custody (no direct owner control over escrow)
- [ ] Executor/session-key model (separate trade vs withdraw authority)
- [ ] Hard on-chain risk controls (whitelist, caps, cooldown)
- [ ] Replay protection (monotonic order IDs or nonces)
- [ ] Post-operation delta checks (verify balance changes)
- [ ] Pause/emergency stop mechanism
- [ ] Comprehensive events for monitoring

### Documentation

- [ ] README with adapter overview
- [ ] Task breakdown (15-min increments)
- [ ] Deployment checklist
- [ ] Integration guide for strategies
- [ ] External dependency documentation

### Testing

- [ ] Unit tests for helper functions
- [ ] Integration tests for all instructions
- [ ] Mainnet-fork tests for protocol CPIs
- [ ] Edge case and error condition tests

### Security

- [ ] Input validation on all parameters
- [ ] Access control on all instructions
- [ ] Safe math (no overflows/underflows)
- [ ] PDA derivation verified
- [ ] Account ownership/signer checks

## Common Adapter Patterns

### 1. Swap Adapter Pattern

Provides gated access to DEX swaps with safety checks.

**Key Features**:

- Pre-swap whitelist check
- Pre-swap balance snapshot
- CPI to DEX swap instruction
- Post-swap delta verification
- Caps and cooldown enforcement

**Example**: Jupiter vault swap execution

### 2. Position Management Adapter Pattern

Manages long-lived positions in external protocols (DCA, lending, etc.).

**Key Features**:

- Position tracking in adapter state
- Open position: escrow funds + create position
- Close position: reconcile refunds + update state
- Exposure limits (max positions, max total escrowed)

**Example**: Jupiter DCA position management

### 3. Order Book Adapter Pattern

Interfaces with order book DEXs (Serum, OpenBook, etc.).

**Key Features**:

- Place order with risk checks
- Cancel order functionality
- Fill tracking and reconciliation
- Order history ledger

**Example**: Serum/OpenBook adapter (planned)

## Adapter Naming Convention

Follow NautilusTrader conventions:

- Use protocol/venue name (not strategy name)
- Lowercase with hyphens: `jupiter-vault`, `serum-dex`, `raydium-amm`
- Program name snake_case: `jupiter_vault`, `serum_dex`, `raydium_amm`
- Avoid generic names like "trading-bot" or "dex-adapter"

## Example Adapters

### Jupiter Vault (`adapters/jupiter-vault`)

On-chain adapter providing controlled access to Jupiter swap and DCA protocols.

**Capabilities**:

- ✅ Swap execution with post-trade verification
- ✅ DCA position management
- ✅ Whitelist enforcement
- ✅ Per-trade and daily caps
- ✅ Executor model (trade but not withdraw)

### Serum DEX (`adapters/serum-dex`) - Planned

On-chain adapter for Serum order book DEX.

**Capabilities**:

- Order placement with risk controls
- Order cancellation
- Fill reconciliation
- Multiple market support

### Raydium AMM (`adapters/raydium-amm`) - Planned

On-chain adapter for Raydium AMM swaps and liquidity provision.

**Capabilities**:

- Swap execution
- LP position management
- Yield harvesting
- Impermanent loss tracking

## Integration with Strategies

Strategies compose adapters to implement trading logic:

```
Strategy: DCA Strategy
├─ Uses: Jupiter Vault Adapter
│  ├─ execute_jupiter_swap()
│  └─ open_dca_position()
├─ Logic: Off-chain scheduling + execution
└─ Configuration: target pairs, amounts, intervals
```

Strategies can:

1. Use a single adapter (simple)
2. Compose multiple adapters (arbitrage, routing)
3. Implement additional off-chain logic (signals, backtesting)

## Resources

- [NautilusTrader Adapters](https://github.com/nautechsystems/nautilus_trader)
- [Anchor Documentation](https://www.anchor-lang.com/docs)
- [Solana Cookbook: PDAs](https://solanacookbook.com/core-concepts/pdas.html)
- [Jupiter Integration Docs](https://dev.jup.ag/)
