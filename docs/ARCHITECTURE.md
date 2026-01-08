# Architecture Diagrams

## High-Level Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                     USER / TRADING SYSTEM                      │
└────────────────────────────┬───────────────────────────────────┘
                             │
                             ▼
┌────────────────────────────────────────────────────────────────┐
│                      STRATEGIES LAYER                          │
│                  (Trading Logic - Off-Chain)                   │
│                                                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│  │   DCA    │  │   ARBIT  │  │   GRID   │  │ MOMENTUM │      │
│  │ Strategy │  │ Strategy │  │ Strategy │  │ Strategy │      │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘      │
│       │             │             │             │             │
└───────┼─────────────┼─────────────┼─────────────┼─────────────┘
        │             │             │             │
        │   ┌─────────┴─────────┐   │             │
        │   │                   │   │             │
        ▼   ▼                   ▼   ▼             ▼
┌────────────────────────────────────────────────────────────────┐
│                      ADAPTERS LAYER                            │
│                (Protocol Interface - On-Chain)                 │
│                                                                │
│  ┌─────────────────┐  ┌─────────────────┐  ┌───────────────┐ │
│  │ Jupiter Vault   │  │   Serum DEX     │  │  Raydium AMM  │ │
│  │                 │  │                 │  │               │ │
│  │ • Custody       │  │ • Order Book    │  │ • AMM Swaps   │ │
│  │ • Risk Controls │  │ • Limit Orders  │  │ • LP Mgmt     │ │
│  │ • Swap CPI      │  │ • Market Orders │  │ • Yield       │ │
│  │ • DCA Positions │  │ • Fill Tracking │  │               │ │
│  └────────┬────────┘  └────────┬────────┘  └───────┬───────┘ │
└───────────┼────────────────────┼────────────────────┼─────────┘
            │                    │                    │
            ▼                    ▼                    ▼
┌────────────────────────────────────────────────────────────────┐
│                     SOLANA PROTOCOLS                           │
│                                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐        │
│  │   Jupiter    │  │ Serum/       │  │   Raydium    │        │
│  │              │  │ OpenBook     │  │              │        │
│  │ • Aggregator │  │ • Order Book │  │ • AMM Pools  │        │
│  │ • DCA        │  │ • Matching   │  │ • Liquidity  │        │
│  │ • Limit      │  │ • Settlement │  │ • Farming    │        │
│  └──────────────┘  └──────────────┘  └──────────────┘        │
└────────────────────────────────────────────────────────────────┘
```

## Component Interaction Flow

### Example: DCA Strategy Execution

```
┌──────────────┐
│ DCA Strategy │  (Off-Chain Python/TypeScript)
└──────┬───────┘
       │
       │ 1. Should trade now?
       │    ✓ Check time interval
       │    ✓ Check market conditions
       │
       │ 2. Calculate parameters
       │    • amount_in = 100 USDC
       │    • min_out = calculate_based_on_price()
       │    • client_order_id = get_next_id()
       │
       │ 3. Call adapter.execute_swap()
       ▼
┌─────────────────────┐
│  Jupiter Vault      │  (On-Chain Anchor Program)
│  Adapter            │
├─────────────────────┤
│                     │
│ 4. Validate Request │
│    ✓ Executor authorized?
│    ✓ Mints whitelisted?
│    ✓ Amount within caps?
│    ✓ Cooldown satisfied?
│    ✓ Order ID monotonic?
│                     │
│ 5. Snapshot Balances│
│    pre_source = 1000 USDC
│    pre_dest = 5 SOL
│                     │
│ 6. CPI to Jupiter   │
│    ▼ invoke_signed()│
│      with PDA signer│
       │
       ▼
┌─────────────────────┐
│  Jupiter Aggregator │  (Protocol)
├─────────────────────┤
│                     │
│ 7. Execute Swap     │
│    • Route across DEXs
│    • Optimize for price
│    • Execute trades
│    • Return results
│                     │
└──────┬──────────────┘
       │
       │ 8. Return
       ▼
┌─────────────────────┐
│  Jupiter Vault      │
│  (continued)        │
├─────────────────────┤
│                     │
│ 9. Reload Balances  │
│    post_source = 900 USDC  (-100)
│    post_dest = 5.19 SOL    (+0.19)
│                     │
│ 10. Verify Deltas   │
│     spent = 100 ✓ <= amount_in
│     received = 0.19 SOL ✓ >= min_out
│                     │
│ 11. Update State    │
│     • Increment trade count
│     • Update daily usage
│     • Record timestamp
│     • Emit trade event
│                     │
│ 12. Return Success  │
└──────┬──────────────┘
       │
       │ 13. Transaction confirmed
       ▼
┌──────────────┐
│ DCA Strategy │
├──────────────┤
│              │
│ 14. Log Trade│
│     • Record in database
│     • Update metrics
│     • Send alerts
│              │
│ 15. Wait     │
│     await sleep(interval)
│              │
│ 16. Repeat   │
└──────────────┘
```

## Security Model

### Custody & Authority Flow

```
┌────────────────────────────────────────────────┐
│               OWNER (Human)                    │
│         (Hardware Wallet / Multisig)           │
└───────┬────────────────────────────────────────┘
        │
        │ Can:
        │ • Initialize vault
        │ • Deposit funds
        │ • Withdraw funds ← ONLY OWNER
        │ • Configure parameters
        │ • Add/remove from whitelist
        │ • Enable/disable executors
        │ • Pause vault
        │
        ▼
┌────────────────────────────────────────────────┐
│            VAULT PROGRAM (PDA)                 │
│                                                │
│  ┌──────────────────────────────────────┐     │
│  │   VAULT AUTHORITY (PDA)              │     │
│  │                                      │     │
│  │   Holds:                             │     │
│  │   • Token Account (USDC)             │     │
│  │   • Token Account (SOL)              │     │
│  │   • Token Account (...)              │     │
│  │                                      │     │
│  │   No private key (derived from seed) │     │
│  └──────────────────────────────────────┘     │
│                                                │
│  ┌──────────────────────────────────────┐     │
│  │   RISK CONTROLS                      │     │
│  │                                      │     │
│  │   • Whitelist                        │     │
│  │   • Per-trade cap                    │     │
│  │   • Daily cap                        │     │
│  │   • Cooldown timer                   │     │
│  │   • Pause flag                       │     │
│  └──────────────────────────────────────┘     │
└───────┬────────────────────────────────────────┘
        │
        │ Grants limited access to:
        ▼
┌────────────────────────────────────────────────┐
│        EXECUTOR (Bot Hot Wallet)               │
│                                                │
│  Can ONLY:                                     │
│  • Call execute_swap()                         │
│  • Call open_dca_position()                    │
│  • Call close_dca_position() (if configured)   │
│                                                │
│  Subject to:                                   │
│  • All risk controls                           │
│  • Cannot withdraw                             │
│  • Cannot change configuration                 │
│                                                │
│  If compromised:                               │
│  • Blast radius limited by caps                │
│  • Whitelist prevents unauthorized tokens      │
│  • Daily cap limits total exposure             │
│  • Owner can immediately pause vault           │
└────────────────────────────────────────────────┘
```

## Data Flow: Adapter State Management

```
┌─────────────────────────────────────────────────────────┐
│                   VAULT CONFIG PDA                      │
├─────────────────────────────────────────────────────────┤
│ • owner: Pubkey                                         │
│ • allowed_swap_program: Pubkey (Jupiter ID)             │
│ • allowed_dca_program: Pubkey (Jupiter DCA ID)          │
│ • global_per_trade_cap: 1000 USDC                       │
│ • global_daily_cap: 10000 USDC                          │
│ • cooldown_secs: 60                                     │
│ • paused: false                                         │
│ • day_index: 19738                                      │
│ • daily_used: 3500 USDC                                 │
│ • last_trade_ts: 1704844234                             │
│ • last_client_order_id: 42                              │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│                  MINT WHITELIST PDA                     │
├─────────────────────────────────────────────────────────┤
│ mints: Vec<Pubkey> [                                    │
│   USDC_MINT,  // EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
│   SOL_MINT,   // So11111111111111111111111111111111111112
│   BONK_MINT,  // DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263
│   ...                                                   │
│ ]                                                       │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│            EXECUTOR CONFIG PDA (per executor)           │
├─────────────────────────────────────────────────────────┤
│ • vault: Pubkey                                         │
│ • executor: Pubkey (bot pubkey)                         │
│ • enabled: true                                         │
│ • per_trade_cap: 500 USDC                               │
│ • daily_cap: 5000 USDC                                  │
│ • day_index: 19738                                      │
│ • daily_used: 1500 USDC                                 │
│ • last_trade_ts: 1704844234                             │
└─────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────┐
│          DCA POSITION PDA (per DCA position)            │
├─────────────────────────────────────────────────────────┤
│ • vault: Pubkey                                         │
│ • position_id: 0                                        │
│ • dca_position_pubkey: Pubkey (Jupiter DCA account)     │
│ • input_mint: USDC                                      │
│ • output_mint: SOL                                      │
│ • deposited_in: 1000 USDC                               │
│ • created_ts: 1704840000                                │
│ • active: true                                          │
└─────────────────────────────────────────────────────────┘
```

## Composability: Multiple Strategies, One Adapter

```
┌──────────────┐
│ DCA Strategy │─────┐
└──────────────┘     │
                     │
┌──────────────┐     │
│ Grid Strategy│─────┤
└──────────────┘     │
                     │     ┌─────────────────┐
┌──────────────┐     ├────▶│ Jupiter Vault   │
│Arb Strategy  │─────┤     │    Adapter      │
└──────────────┘     │     └────────┬────────┘
                     │              │
┌──────────────┐     │              │
│Momentum Strat│─────┘              │
└──────────────┘                    │
                                    ▼
                            ┌────────────────┐
                            │    Jupiter     │
                            │   Protocol     │
                            └────────────────┘

Benefits:
• One adapter, multiple strategies
• Shared risk controls across all strategies
• Centralized custody
• Unified monitoring
```

## Multi-Adapter Strategy: Arbitrage

```
┌────────────────────────────────────────────────┐
│        ARBITRAGE STRATEGY                      │
│                                                │
│  1. Monitor prices across DEXs                 │
│  2. Detect profitable spread                   │
│  3. Execute simultaneous trades                │
└───────────┬───────────────────┬────────────────┘
            │                   │
            │                   │
        DEX A               DEX B
            │                   │
            ▼                   ▼
┌─────────────────┐   ┌─────────────────┐
│ Jupiter Vault   │   │   Raydium       │
│   Adapter       │   │   Adapter       │
└────────┬────────┘   └────────┬────────┘
         │                     │
         │                     │
         ▼                     ▼
┌─────────────────┐   ┌─────────────────┐
│    Jupiter      │   │    Raydium      │
│   Aggregator    │   │      AMM        │
└─────────────────┘   └─────────────────┘

Execution Flow:
1. Buy SOL on Jupiter  (lower price)
2. Sell SOL on Raydium (higher price)
3. Profit = price_diff - fees

Risk Controls:
• Both adapters enforce whitelists
• Both enforce caps independently
• Arbitrage strategy has its own circuit breaker
```

## Testing Architecture

```
┌────────────────────────────────────────────────┐
│              DEVELOPMENT PHASES                │
└────────────────────────────────────────────────┘

Phase 1: ADAPTER UNIT TESTS
├─ Pure Rust function tests
├─ No blockchain required
└─ Test: authorize_trade(), whitelist_contains(), etc.

Phase 2: ADAPTER INTEGRATION TESTS (Localnet)
├─ Anchor test framework
├─ Local validator with cloned Jupiter programs
└─ Test: Full instruction flows

Phase 3: STRATEGY BACKTESTS
├─ Historical data
├─ Simulated adapter responses
└─ Test: Strategy logic, parameter optimization

Phase 4: PAPER TRADING (Devnet/Mainnet)
├─ Live market data
├─ Test accounts
└─ Test: End-to-end flow without real capital

Phase 5: LIVE TRADING (Mainnet)
├─ Real capital (start small)
├─ Gradual scaling
└─ Test: Production reliability

┌────────────────────────────────────────────────┐
│           TEST DATA FLOW                       │
└────────────────────────────────────────────────┘

Backtest:
  Historical Data → Strategy Logic → Simulated Results

Paper Trading:
  Live Data → Strategy Logic → Test Account Execution → Verify

Live Trading:
  Live Data → Strategy Logic → Real Account Execution → Monitor
```

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    PRODUCTION SETUP                     │
└─────────────────────────────────────────────────────────┘

MAINNET:
├─ Jupiter Vault Adapter (Program ID: JupVau1t...)
│  ├─ Deployed via: anchor deploy
│  └─ Verified on: Solana Explorer
│
├─ Owner Wallet (Multisig)
│  ├─ Holds: Vault configuration authority
│  └─ Secured: Hardware wallet / Squads Protocol
│
└─ Executor Wallet (Hot)
   ├─ Holds: Limited trading authority
   └─ Secured: Encrypted key management

OFF-CHAIN INFRASTRUCTURE:
├─ Strategy Executor (Server/Container)
│  ├─ Runs: DCA strategy Python script
│  ├─ Connects: RPC node (QuickNode/Helius)
│  └─ Monitors: Events, errors, metrics
│
├─ Database (PostgreSQL)
│  ├─ Stores: Trade history, performance metrics
│  └─ Used for: Backtesting, reporting
│
├─ Monitoring (Prometheus + Grafana)
│  ├─ Tracks: Transaction success rate, latency
│  └─ Alerts: Failures, anomalies
│
└─ Backup Executor (Standby)
   └─ Failover: Automatic or manual switchover

┌─────────────────────────────────────────────────────────┐
│               DATA FLOW IN PRODUCTION                   │
└─────────────────────────────────────────────────────────┘

1. Cron triggers strategy.run()
2. Strategy queries RPC for market data
3. Strategy decides: execute trade
4. Strategy calls adapter.execute_swap()
5. Adapter validates + executes on-chain
6. Adapter emits trade event
7. Strategy logs to database
8. Monitoring captures metrics
9. Alerts trigger if anomalies
```

## Risk Layering

```
┌─────────────────────────────────────────────────────────┐
│                DEFENSE IN DEPTH                         │
└─────────────────────────────────────────────────────────┘

Layer 1: ADAPTER ON-CHAIN CONTROLS
├─ Whitelist: Only approved tokens
├─ Per-trade cap: Maximum single trade
├─ Daily cap: Maximum per 24 hours
├─ Cooldown: Minimum time between trades
├─ Replay protection: Monotonic order IDs
└─ Balance verification: Post-CPI delta checks

Layer 2: STRATEGY OFF-CHAIN CONTROLS
├─ Circuit breaker: Max daily loss limit
├─ Volatility filter: Pause during high volatility
├─ Slippage limit: Cancel if price moves too much
├─ Position size: Kelly criterion or fixed fraction
└─ Heartbeat: Auto-pause if monitoring fails

Layer 3: OPERATIONAL CONTROLS
├─ Multisig owner: Requires multiple approvals
├─ Executor rotation: Regular key rotation
├─ Monitoring: 24/7 alerts
├─ Audit logs: Complete trade history
└─ Kill switch: Immediate pause capability

Layer 4: PORTFOLIO CONTROLS
├─ Diversification: Multiple strategies
├─ Correlation analysis: Avoid correlated bets
├─ Maximum allocation: Per strategy/adapter
└─ Rebalancing: Periodic portfolio review
```

---

## Summary

This architecture provides:

✅ **Clear separation of concerns** (adapter vs strategy)  
✅ **Composability** (one adapter, many strategies)  
✅ **Defense in depth** (multiple risk layers)  
✅ **Testability** (unit → integration → backtest → paper → live)  
✅ **Maintainability** (modular, well-documented)  
✅ **Scalability** (add new adapters/strategies independently)  

Following NautilusTrader's proven patterns adapted to Solana's on-chain environment.
