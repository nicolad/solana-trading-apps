# Jupiter Vault Adapter - 15-Minute Task Breakdown

Complete implementation guide broken into focused 15-minute tasks.

## Overview

Building an on-chain adapter for Jupiter swap and DCA protocols with:

- PDA custody
- Hard risk controls (whitelist, caps, cooldown, daily limits)
- Executor/session-key model
- Jupiter CPI swap execution
- DCA position management

---

## ⚙️ Task 1: Create Anchor Workspace (15 min)

**Goal**: Initialize the Anchor project structure

**Commands**:

```bash
cd bots
anchor init dca-vault
cd dca-vault
```

**Files to modify**:

1. **`Anchor.toml`** - Update configuration:

```toml
[features]
seeds = false
skip-lint = false

[programs.localnet]
jupiter_vault = "JupVau1t11111111111111111111111111111111111"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "localnet"
wallet = "~/.config/solana/id.json"

[scripts]
test = "echo \"Rust-only: add your own harness if desired\""
```

1. **`Cargo.toml`** (workspace root):

```toml
[workspace]
members = [
  "programs/jupiter-vault"
]
resolver = "2"
```

1. **`programs/jupiter-vault/Cargo.toml`**:

```toml
[package]
name = "jupiter-vault"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]
name = "jupiter_vault"

[features]
no-entrypoint = []
no-idl = []
no-log-ix-name = []
cpi = ["no-entrypoint"]
default = []

[dependencies]
anchor-lang = "0.32.1"
anchor-spl = "0.32.1"
```

**Verification**:

```bash
anchor build
# Should compile successfully
```

**Done when**: `anchor build` completes without errors

---

## 📦 Task 2: Create State Accounts Module (15 min)

**Goal**: Define all state account structures with proper sizing

**Create**: `programs/jupiter-vault/src/state.rs`

```rust
use anchor_lang::prelude::*;

pub const SEED_VAULT: &[u8] = b"vault";
pub const SEED_AUTH: &[u8] = b"auth";
pub const SEED_MINTS: &[u8] = b"mints";
pub const SEED_EXEC: &[u8] = b"exec";
pub const SEED_DCA_POS: &[u8] = b"dca_pos";

pub const MAX_MINTS: usize = 64;
pub const MAX_POSITIONS: usize = 32;

#[account]
#[derive(InitSpace)]
pub struct VaultConfig {
    pub owner: Pubkey,

    /// PDA bumps
    pub vault_bump: u8,
    pub auth_bump: u8,

    /// Safety toggles
    pub paused: bool,

    /// Which programs we allow CPI into
    pub allowed_swap_program: Pubkey,
    pub allowed_dca_program: Pubkey,

    /// Risk controls
    pub cooldown_secs: u32,
    pub global_per_trade_cap: u64,
    pub global_daily_cap: u64,

    /// DCA specific limits
    pub max_active_positions: u32,
    pub max_total_escrowed: u64,
    pub total_escrowed: u64,

    /// Global counters
    pub day_index: i64,
    pub daily_used: u64,
    pub last_trade_ts: i64,
    pub last_client_order_id: u64,
    pub trade_count: u64,
}

#[account]
#[derive(InitSpace)]
pub struct MintWhitelist {
    #[max_len(MAX_MINTS)]
    pub mints: Vec<Pubkey>,
}

#[account]
#[derive(InitSpace)]
pub struct ExecutorConfig {
    pub vault: Pubkey,
    pub executor: Pubkey,
    pub enabled: bool,

    /// Per-executor risk
    pub per_trade_cap: u64,
    pub daily_cap: u64,

    /// Per-executor counters
    pub day_index: i64,
    pub daily_used: u64,
    pub last_trade_ts: i64,
}

#[account]
#[derive(InitSpace)]
pub struct DcaPosition {
    pub vault: Pubkey,
    pub position_id: u64,
    
    /// Jupiter DCA position pubkey
    pub dca_position_pubkey: Pubkey,
    
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    
    pub deposited_in: u64,
    pub created_ts: i64,
    pub active: bool,
}
```

**Update**: `programs/jupiter-vault/src/lib.rs` to include the module:

```rust
mod state;
use state::*;
```

**Verification**:

```bash
anchor build
```

**Done when**: Program compiles with all state accounts defined

---

## 🚨 Task 3: Create Errors Module (15 min)

**Goal**: Define all custom error codes

**Create**: `programs/jupiter-vault/src/errors.rs`

```rust
use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Unauthorized")]
    Unauthorized,

    #[msg("Vault is paused")]
    Paused,

    #[msg("Executor disabled")]
    ExecutorDisabled,

    #[msg("Mint not whitelisted")]
    MintNotWhitelisted,

    #[msg("Per-trade cap exceeded")]
    PerTradeCapExceeded,

    #[msg("Daily cap exceeded")]
    DailyCapExceeded,

    #[msg("Cooldown not satisfied")]
    Cooldown,

    #[msg("Invalid amount")]
    InvalidAmount,

    #[msg("Client order id must be strictly increasing")]
    ClientOrderIdNotIncreasing,

    #[msg("Jupiter program id mismatch")]
    JupiterProgramMismatch,

    #[msg("Required account missing from remaining accounts")]
    MissingRemainingAccount,

    #[msg("Post-trade check failed: received_out < min_out")]
    MinOutNotMet,

    #[msg("Post-trade check failed: spent_in > amount_in")]
    SpentInTooHigh,

    #[msg("Instruction data too large")]
    IxDataTooLarge,

    #[msg("Whitelist capacity reached")]
    WhitelistFull,

    #[msg("Mint already whitelisted")]
    MintAlreadyWhitelisted,

    #[msg("Mint not present in whitelist")]
    MintNotInWhitelist,

    #[msg("Max active positions reached")]
    MaxPositionsReached,

    #[msg("Max total escrowed exceeded")]
    MaxEscrowExceeded,

    #[msg("Position not active")]
    PositionNotActive,

    #[msg("DCA program id mismatch")]
    DcaProgramMismatch,
}
```

**Update**: `programs/dca-vault/src/lib.rs`:

```rust
mod errors;
use errors::VaultError;
```

**Verification**:

```bash
anchor build
```

**Done when**: All errors compile successfully

---

## 🛠️ Task 4: Create Utils Module (15 min)

**Goal**: Implement helper functions for risk checks

**Create**: `programs/jupiter-vault/src/utils.rs`

```rust
use anchor_lang::prelude::*;
use crate::errors::VaultError;
use crate::state::{ExecutorConfig, VaultConfig, MintWhitelist};

pub const SECONDS_PER_DAY: i64 = 86_400;
pub const MAX_IX_DATA: usize = 8_192;

pub fn current_day_index(now_ts: i64) -> i64 {
    now_ts.div_euclid(SECONDS_PER_DAY)
}

pub fn whitelist_contains(w: &MintWhitelist, mint: &Pubkey) -> bool {
    w.mints.iter().any(|m| m == mint)
}

pub fn require_whitelisted(w: &MintWhitelist, mint: &Pubkey) -> Result<()> {
    require!(whitelist_contains(w, mint), VaultError::MintNotWhitelisted);
    Ok(())
}

pub fn reset_daily_if_needed(day_index: i64, used: &mut u64, stored_day: &mut i64) {
    if *stored_day != day_index {
        *stored_day = day_index;
        *used = 0;
    }
}

pub fn authorize_trade(
    cfg: &mut VaultConfig,
    exec: &mut ExecutorConfig,
    now_ts: i64,
    amount_in: u64,
    client_order_id: u64,
) -> Result<()> {
    require!(!cfg.paused, VaultError::Paused);
    require!(exec.enabled, VaultError::ExecutorDisabled);
    require!(amount_in > 0, VaultError::InvalidAmount);

    // Monotonic anti-replay
    require!(
        client_order_id > cfg.last_client_order_id,
        VaultError::ClientOrderIdNotIncreasing
    );

    // Cooldown
    let cooldown = cfg.cooldown_secs as i64;
    if cooldown > 0 {
        require!(
            now_ts.saturating_sub(cfg.last_trade_ts) >= cooldown,
            VaultError::Cooldown
        );
        require!(
            now_ts.saturating_sub(exec.last_trade_ts) >= cooldown,
            VaultError::Cooldown
        );
    }

    // Per-trade caps
    require!(
        amount_in <= cfg.global_per_trade_cap,
        VaultError::PerTradeCapExceeded
    );
    require!(
        amount_in <= exec.per_trade_cap,
        VaultError::PerTradeCapExceeded
    );

    // Daily caps (reset on day change)
    let day = current_day_index(now_ts);
    reset_daily_if_needed(day, &mut cfg.daily_used, &mut cfg.day_index);
    reset_daily_if_needed(day, &mut exec.daily_used, &mut exec.day_index);

    let cfg_next = cfg.daily_used.saturating_add(amount_in);
    require!(cfg_next <= cfg.global_daily_cap, VaultError::DailyCapExceeded);

    let exec_next = exec.daily_used.saturating_add(amount_in);
    require!(exec_next <= exec.daily_cap, VaultError::DailyCapExceeded);

    // Commit counters
    cfg.daily_used = cfg_next;
    exec.daily_used = exec_next;

    cfg.last_trade_ts = now_ts;
    exec.last_trade_ts = now_ts;

    cfg.last_client_order_id = client_order_id;
    cfg.trade_count = cfg.trade_count.saturating_add(1);

    Ok(())
}

pub fn require_ix_data_size(ix_data: &[u8]) -> Result<()> {
    require!(ix_data.len() <= MAX_IX_DATA, VaultError::IxDataTooLarge);
    Ok(())
}

pub fn require_present_in_remaining(
    remaining: &[AccountInfo],
    key: &Pubkey,
) -> Result<()> {
    let found = remaining.iter().any(|ai| ai.key == key);
    require!(found, VaultError::MissingRemainingAccount);
    Ok(())
}
```

**Update**: `programs/dca-vault/src/lib.rs`:

```rust
mod utils;
use utils::*;
```

**Verification**:

```bash
anchor build
```

**Done when**: Utils module compiles

---

## 🏗️ Task 5: Implement Initialize Instruction (15 min)

**Goal**: Create the vault initialization logic

**Update**: `programs/dca-vault/src/lib.rs`

Add after the `use` statements:

```rust
declare_id!("JupVau1t11111111111111111111111111111111111");

#[program]
pub mod jupiter_vault {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        allowed_swap_program: Pubkey,
        allowed_dca_program: Pubkey,
        cooldown_secs: u32,
        global_per_trade_cap: u64,
        global_daily_cap: u64,
        max_active_positions: u32,
        max_total_escrowed: u64,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.vault_config;
        cfg.owner = ctx.accounts.owner.key();
        cfg.vault_bump = ctx.bumps.vault_config;
        cfg.auth_bump = ctx.bumps.vault_authority;
        cfg.paused = false;
        cfg.allowed_swap_program = allowed_swap_program;
        cfg.allowed_dca_program = allowed_dca_program;
        cfg.cooldown_secs = cooldown_secs;
        cfg.global_per_trade_cap = global_per_trade_cap;
        cfg.global_daily_cap = global_daily_cap;
        cfg.max_active_positions = max_active_positions;
        cfg.max_total_escrowed = max_total_escrowed;
        cfg.total_escrowed = 0;
        cfg.day_index = current_day_index(Clock::get()?.unix_timestamp);
        cfg.daily_used = 0;
        cfg.last_trade_ts = 0;
        cfg.last_client_order_id = 0;
        cfg.trade_count = 0;

        let w = &mut ctx.accounts.mint_whitelist;
        w.mints = Vec::new();

        emit!(VaultInitialized {
            owner: cfg.owner,
            allowed_swap_program,
            allowed_dca_program,
            cooldown_secs,
            global_per_trade_cap,
            global_daily_cap,
            max_active_positions,
            max_total_escrowed,
        });

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + VaultConfig::INIT_SPACE,
        seeds = [SEED_VAULT, owner.key().as_ref()],
        bump
    )]
    pub vault_config: Account<'info, VaultConfig>,

    /// CHECK: PDA signer only; no data.
    #[account(
        seeds = [SEED_AUTH, vault_config.key().as_ref()],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + MintWhitelist::INIT_SPACE,
        seeds = [SEED_MINTS, vault_config.key().as_ref()],
        bump
    )]
    pub mint_whitelist: Account<'info, MintWhitelist>,

    pub system_program: Program<'info, System>,
}

#[event]
pub struct VaultInitialized {
    pub owner: Pubkey,
    pub allowed_swap_program: Pubkey,
    pub allowed_dca_program: Pubkey,
    pub cooldown_secs: u32,
    pub global_per_trade_cap: u64,
    pub global_daily_cap: u64,
    pub max_active_positions: u32,
    pub max_total_escrowed: u64,
}
```

**Verification**:

```bash
anchor build
```

**Done when**: Initialize instruction compiles

---

## 🔐 Task 6: Implement Admin Functions (15 min)

**Goal**: Add pause/unpause and parameter updates

**Add to** `#[program] mod jupiter_vault` in `lib.rs`:

```rust
    pub fn pause(ctx: Context<OwnerOnly>) -> Result<()> {
        ctx.accounts.vault_config.paused = true;
        emit!(VaultPaused { owner: ctx.accounts.owner.key() });
        Ok(())
    }

    pub fn unpause(ctx: Context<OwnerOnly>) -> Result<()> {
        ctx.accounts.vault_config.paused = false;
        emit!(VaultUnpaused { owner: ctx.accounts.owner.key() });
        Ok(())
    }

    pub fn set_params(
        ctx: Context<OwnerOnly>,
        allowed_swap_program: Pubkey,
        allowed_dca_program: Pubkey,
        cooldown_secs: u32,
        global_per_trade_cap: u64,
        global_daily_cap: u64,
        max_active_positions: u32,
        max_total_escrowed: u64,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.vault_config;
        cfg.allowed_swap_program = allowed_swap_program;
        cfg.allowed_dca_program = allowed_dca_program;
        cfg.cooldown_secs = cooldown_secs;
        cfg.global_per_trade_cap = global_per_trade_cap;
        cfg.global_daily_cap = global_daily_cap;
        cfg.max_active_positions = max_active_positions;
        cfg.max_total_escrowed = max_total_escrowed;

        emit!(ParamsUpdated {
            owner: ctx.accounts.owner.key(),
            allowed_swap_program,
            allowed_dca_program,
            cooldown_secs,
            global_per_trade_cap,
            global_daily_cap,
            max_active_positions,
            max_total_escrowed,
        });
        Ok(())
    }
```

**Add context and events** after the `Initialize` struct:

```rust
#[derive(Accounts)]
pub struct OwnerOnly<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_VAULT, owner.key().as_ref()],
        bump = vault_config.vault_bump,
        constraint = vault_config.owner == owner.key() @ VaultError::Unauthorized
    )]
    pub vault_config: Account<'info, VaultConfig>,
}

#[event]
pub struct VaultPaused {
    pub owner: Pubkey,
}

#[event]
pub struct VaultUnpaused {
    pub owner: Pubkey,
}

#[event]
pub struct ParamsUpdated {
    pub owner: Pubkey,
    pub allowed_swap_program: Pubkey,
    pub allowed_dca_program: Pubkey,
    pub cooldown_secs: u32,
    pub global_per_trade_cap: u64,
    pub global_daily_cap: u64,
    pub max_active_positions: u32,
    pub max_total_escrowed: u64,
}
```

**Verification**:

```bash
anchor build
```

**Done when**: Admin functions compile

---

## 📝 Task 7: Implement Whitelist Management (15 min)

**Goal**: Add mint whitelist operations

**Add to** `#[program] mod dca_vault`:

```rust
    pub fn add_mint(ctx: Context<OwnerAndWhitelist>, mint: Pubkey) -> Result<()> {
        let w = &mut ctx.accounts.mint_whitelist;

        require!(w.mints.len() < MAX_MINTS, VaultError::WhitelistFull);
        require!(!whitelist_contains(w, &mint), VaultError::MintAlreadyWhitelisted);

        w.mints.push(mint);
        emit!(MintAdded { owner: ctx.accounts.owner.key(), mint });
        Ok(())
    }

    pub fn remove_mint(ctx: Context<OwnerAndWhitelist>, mint: Pubkey) -> Result<()> {
        let w = &mut ctx.accounts.mint_whitelist;
        let idx = w.mints.iter().position(|m| m == &mint).ok_or(VaultError::MintNotInWhitelist)?;
        w.mints.swap_remove(idx);
        emit!(MintRemoved { owner: ctx.accounts.owner.key(), mint });
        Ok(())
    }
```

**Add context and events**:

```rust
#[derive(Accounts)]
pub struct OwnerAndWhitelist<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        seeds = [SEED_VAULT, owner.key().as_ref()],
        bump = vault_config.vault_bump,
        constraint = vault_config.owner == owner.key() @ VaultError::Unauthorized
    )]
    pub vault_config: Account<'info, VaultConfig>,
    #[account(
        mut,
        seeds = [SEED_MINTS, vault_config.key().as_ref()],
        bump
    )]
    pub mint_whitelist: Account<'info, MintWhitelist>,
}

#[event]
pub struct MintAdded {
    pub owner: Pubkey,
    pub mint: Pubkey,
}

#[event]
pub struct MintRemoved {
    pub owner: Pubkey,
    pub mint: Pubkey,
}
```

**Verification**:

```bash
anchor build
```

**Done when**: Whitelist management compiles

---

## 👤 Task 8: Implement Executor Management (15 min)

**Goal**: Add executor configuration

**Add to** `#[program] mod dca_vault`:

```rust
    pub fn set_executor(
        ctx: Context<SetExecutor>,
        enabled: bool,
        per_trade_cap: u64,
        daily_cap: u64,
    ) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let day = current_day_index(now);

        let e = &mut ctx.accounts.executor_config;
        e.vault = ctx.accounts.vault_config.key();
        e.executor = ctx.accounts.executor.key();
        e.enabled = enabled;
        e.per_trade_cap = per_trade_cap;
        e.daily_cap = daily_cap;

        // reset counters
        e.day_index = day;
        e.daily_used = 0;
        e.last_trade_ts = 0;

        emit!(ExecutorSet {
            owner: ctx.accounts.owner.key(),
            executor: e.executor,
            enabled,
            per_trade_cap,
            daily_cap,
        });
        Ok(())
    }
```

**Add context and event**:

```rust
#[derive(Accounts)]
#[instruction()]
pub struct SetExecutor<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    /// CHECK: any pubkey; stored inside executor_config
    pub executor: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_VAULT, owner.key().as_ref()],
        bump = vault_config.vault_bump,
        constraint = vault_config.owner == owner.key() @ VaultError::Unauthorized
    )]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        init_if_needed,
        payer = owner,
        space = 8 + ExecutorConfig::INIT_SPACE,
        seeds = [SEED_EXEC, vault_config.key().as_ref(), executor.key().as_ref()],
        bump
    )]
    pub executor_config: Account<'info, ExecutorConfig>,

    pub system_program: Program<'info, System>,
}

#[event]
pub struct ExecutorSet {
    pub owner: Pubkey,
    pub executor: Pubkey,
    pub enabled: bool,
    pub per_trade_cap: u64,
    pub daily_cap: u64,
}
```

**Verification**:

```bash
anchor build
```

**Done when**: Executor management compiles

---

## 💰 Task 9: Implement Deposit/Withdraw (15 min)

**Goal**: Add fund custody operations

**Add imports** at top of `lib.rs`:

```rust
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    Mint, TokenAccount, TokenInterface, TransferChecked,
};
```

**Add to** `#[program] mod dca_vault`:

```rust
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        let mint_decimals = ctx.accounts.mint.decimals;
        let ix = TransferChecked {
            from: ctx.accounts.user_ata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.vault_ata.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        };

        anchor_spl::token_interface::transfer_checked(
            CpiContext::new(ctx.accounts.token_program.to_account_info(), ix),
            amount,
            mint_decimals,
        )?;

        emit!(DepositEvent {
            owner: ctx.accounts.owner.key(),
            mint: ctx.accounts.mint.key(),
            amount,
        });

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);

        let cfg = &ctx.accounts.vault_config;

        let signer_seeds: &[&[&[u8]]] = &[&[
            SEED_AUTH,
            cfg.key().as_ref(),
            &[cfg.auth_bump],
        ]];

        let mint_decimals = ctx.accounts.mint.decimals;
        let ix = TransferChecked {
            from: ctx.accounts.vault_ata.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
            to: ctx.accounts.user_ata.to_account_info(),
            authority: ctx.accounts.vault_authority.to_account_info(),
        };

        anchor_spl::token_interface::transfer_checked(
            CpiContext::new_with_signer(ctx.accounts.token_program.to_account_info(), ix, signer_seeds),
            amount,
            mint_decimals,
        )?;

        emit!(WithdrawEvent {
            owner: ctx.accounts.owner.key(),
            mint: ctx.accounts.mint.key(),
            amount,
        });

        Ok(())
    }
```

**Add contexts and events**:

```rust
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [SEED_VAULT, owner.key().as_ref()],
        bump = vault_config.vault_bump,
        constraint = vault_config.owner == owner.key() @ VaultError::Unauthorized
    )]
    pub vault_config: Account<'info, VaultConfig>,

    /// CHECK: PDA authority signer only
    #[account(
        seeds = [SEED_AUTH, vault_config.key().as_ref()],
        bump = vault_config.auth_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = mint,
        associated_token::authority = vault_authority
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        seeds = [SEED_VAULT, owner.key().as_ref()],
        bump = vault_config.vault_bump,
        constraint = vault_config.owner == owner.key() @ VaultError::Unauthorized
    )]
    pub vault_config: Account<'info, VaultConfig>,

    /// CHECK: PDA authority signer only
    #[account(
        seeds = [SEED_AUTH, vault_config.key().as_ref()],
        bump = vault_config.auth_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = vault_authority
    )]
    pub vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,
}

#[event]
pub struct DepositEvent {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}

#[event]
pub struct WithdrawEvent {
    pub owner: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
}
```

**Verification**:

```bash
anchor build
```

**Done when**: Deposit and withdraw compile

---

## 🔄 Task 10: Implement Jupiter Swap Execution (15 min)

**Goal**: Add the CPI swap gate with post-trade verification

**Add imports** at top of `lib.rs`:

```rust
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke_signed;
```

**Add to** `#[program] mod dca_vault`:

```rust
    pub fn execute_jupiter_swap(
        ctx: Context<ExecuteJupiterSwap>,
        input_mint: Pubkey,
        output_mint: Pubkey,
        amount_in: u64,
        min_out: u64,
        client_order_id: u64,
        jupiter_ix_data: Vec<u8>,
    ) -> Result<()> {
        require_ix_data_size(&jupiter_ix_data)?;
        require!(amount_in > 0, VaultError::InvalidAmount);
        require!(min_out > 0, VaultError::InvalidAmount);

        // Whitelist both sides
        require_whitelisted(&ctx.accounts.mint_whitelist, &input_mint)?;
        require_whitelisted(&ctx.accounts.mint_whitelist, &output_mint)?;

        // Ensure the passed named accounts match the mints
        require_keys_eq!(ctx.accounts.input_mint.key(), input_mint);
        require_keys_eq!(ctx.accounts.output_mint.key(), output_mint);

        // Ensure Jupiter program matches what owner configured
        require_keys_eq!(
            ctx.accounts.jupiter_program.key(),
            ctx.accounts.vault_config.allowed_swap_program,
            VaultError::JupiterProgramMismatch
        );

        // Require critical accounts appear in remaining_accounts
        let remaining = ctx.remaining_accounts;
        require_present_in_remaining(remaining, &ctx.accounts.vault_authority.key())?;
        require_present_in_remaining(remaining, &ctx.accounts.source_vault_ata.key())?;
        require_present_in_remaining(remaining, &ctx.accounts.dest_vault_ata.key())?;

        // Snapshot balances
        let pre_source = ctx.accounts.source_vault_ata.amount;
        let pre_dest = ctx.accounts.dest_vault_ata.amount;

        // Authorize (mutates counters)
        let now_ts = Clock::get()?.unix_timestamp;
        {
            let cfg = &mut ctx.accounts.vault_config;
            let exec = &mut ctx.accounts.executor_config;
            authorize_trade(cfg, exec, now_ts, amount_in, client_order_id)?;
        }

        // Build metas from remaining accounts
        let vault_auth_key = ctx.accounts.vault_authority.key();
        let metas: Vec<AccountMeta> = remaining
            .iter()
            .map(|ai| AccountMeta {
                pubkey: *ai.key,
                is_signer: ai.is_signer || *ai.key == vault_auth_key,
                is_writable: ai.is_writable,
            })
            .collect();

        let ix = Instruction {
            program_id: ctx.accounts.jupiter_program.key(),
            accounts: metas,
            data: jupiter_ix_data,
        };

        let mut infos: Vec<AccountInfo> = Vec::with_capacity(1 + remaining.len());
        infos.push(ctx.accounts.jupiter_program.to_account_info());
        infos.extend_from_slice(remaining);

        // PDA signer seeds
        let cfg = &ctx.accounts.vault_config;
        let signer_seeds: &[&[&[u8]]] = &[&[
            SEED_AUTH,
            cfg.key().as_ref(),
            &[cfg.auth_bump],
        ]];

        invoke_signed(&ix, &infos, signer_seeds)?;

        // Reload to get post-CPI balances
        ctx.accounts.source_vault_ata.reload()?;
        ctx.accounts.dest_vault_ata.reload()?;

        let post_source = ctx.accounts.source_vault_ata.amount;
        let post_dest = ctx.accounts.dest_vault_ata.amount;

        let spent_in = pre_source.saturating_sub(post_source);
        let received_out = post_dest.saturating_sub(pre_dest);

        require!(spent_in <= amount_in, VaultError::SpentInTooHigh);
        require!(received_out >= min_out, VaultError::MinOutNotMet);

        emit!(TradeEvent {
            executor: ctx.accounts.executor.key(),
            input_mint,
            output_mint,
            amount_in,
            min_out,
            spent_in,
            received_out,
            ts: now_ts,
            client_order_id,
        });

        Ok(())
    }
```

**Add context and event**:

```rust
#[derive(Accounts)]
pub struct ExecuteJupiterSwap<'info> {
    pub executor: Signer<'info>,

    #[account(mut)]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [SEED_EXEC, vault_config.key().as_ref(), executor.key().as_ref()],
        bump,
        constraint = executor_config.vault == vault_config.key() @ VaultError::Unauthorized,
        constraint = executor_config.executor == executor.key() @ VaultError::Unauthorized
    )]
    pub executor_config: Account<'info, ExecutorConfig>,

    #[account(
        seeds = [SEED_MINTS, vault_config.key().as_ref()],
        bump
    )]
    pub mint_whitelist: Account<'info, MintWhitelist>,

    /// CHECK: PDA authority signer only
    #[account(
        seeds = [SEED_AUTH, vault_config.key().as_ref()],
        bump = vault_config.auth_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub input_mint: InterfaceAccount<'info, Mint>,
    pub output_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub source_vault_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(mut)]
    pub dest_vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    /// CHECK: Verified against vault_config.allowed_swap_program
    pub jupiter_program: UncheckedAccount<'info>,
}

#[event]
pub struct TradeEvent {
    pub executor: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub amount_in: u64,
    pub min_out: u64,
    pub spent_in: u64,
    pub received_out: u64,
    pub ts: i64,
    pub client_order_id: u64,
}
```

**Verification**:

```bash
anchor build
```

**Done when**: Swap execution compiles

---

## 📊 Task 11: Implement Open DCA Position (15 min)

**Goal**: Add DCA position opening with escrow tracking

**Add to** `#[program] mod dca_vault`:

```rust
    pub fn open_dca_position(
        ctx: Context<OpenDcaPosition>,
        position_id: u64,
        input_mint: Pubkey,
        output_mint: Pubkey,
        total_deposit: u64,
        client_order_id: u64,
        dca_ix_data: Vec<u8>,
    ) -> Result<()> {
        require_ix_data_size(&dca_ix_data)?;
        require!(total_deposit > 0, VaultError::InvalidAmount);

        let cfg = &mut ctx.accounts.vault_config;

        // Check position limits
        require!(
            position_id < cfg.max_active_positions as u64,
            VaultError::MaxPositionsReached
        );

        let new_escrowed = cfg.total_escrowed.saturating_add(total_deposit);
        require!(
            new_escrowed <= cfg.max_total_escrowed,
            VaultError::MaxEscrowExceeded
        );

        // Whitelist both sides
        require_whitelisted(&ctx.accounts.mint_whitelist, &input_mint)?;
        require_whitelisted(&ctx.accounts.mint_whitelist, &output_mint)?;

        require_keys_eq!(ctx.accounts.input_mint.key(), input_mint);
        require_keys_eq!(ctx.accounts.output_mint.key(), output_mint);

        // Ensure DCA program matches config
        require_keys_eq!(
            ctx.accounts.dca_program.key(),
            cfg.allowed_dca_program,
            VaultError::DcaProgramMismatch
        );

        // Snapshot balance
        let pre_source = ctx.accounts.source_vault_ata.amount;

        // Authorize (uses normal trade limits)
        let now_ts = Clock::get()?.unix_timestamp;
        {
            let exec = &mut ctx.accounts.executor_config;
            authorize_trade(cfg, exec, now_ts, total_deposit, client_order_id)?;
        }

        // Build CPI
        let remaining = ctx.remaining_accounts;
        let vault_auth_key = ctx.accounts.vault_authority.key();
        
        let metas: Vec<AccountMeta> = remaining
            .iter()
            .map(|ai| AccountMeta {
                pubkey: *ai.key,
                is_signer: ai.is_signer || *ai.key == vault_auth_key,
                is_writable: ai.is_writable,
            })
            .collect();

        let ix = Instruction {
            program_id: ctx.accounts.dca_program.key(),
            accounts: metas,
            data: dca_ix_data,
        };

        let mut infos: Vec<AccountInfo> = Vec::with_capacity(1 + remaining.len());
        infos.push(ctx.accounts.dca_program.to_account_info());
        infos.extend_from_slice(remaining);

        let signer_seeds: &[&[&[u8]]] = &[&[
            SEED_AUTH,
            cfg.key().as_ref(),
            &[cfg.auth_bump],
        ]];

        invoke_signed(&ix, &infos, signer_seeds)?;

        // Reload and verify
        ctx.accounts.source_vault_ata.reload()?;
        let post_source = ctx.accounts.source_vault_ata.amount;
        let spent_in = pre_source.saturating_sub(post_source);

        require!(spent_in <= total_deposit, VaultError::SpentInTooHigh);

        // Update escrow tracking
        cfg.total_escrowed = new_escrowed;

        // Store position
        let pos = &mut ctx.accounts.dca_position;
        pos.vault = cfg.key();
        pos.position_id = position_id;
        pos.dca_position_pubkey = ctx.accounts.dca_position_account.key();
        pos.input_mint = input_mint;
        pos.output_mint = output_mint;
        pos.deposited_in = spent_in;
        pos.created_ts = now_ts;
        pos.active = true;

        emit!(DcaPositionOpened {
            executor: ctx.accounts.executor.key(),
            position_id,
            dca_position_pubkey: pos.dca_position_pubkey,
            input_mint,
            output_mint,
            deposited_in: spent_in,
            ts: now_ts,
        });

        Ok(())
    }
```

**Add context and event**:

```rust
#[derive(Accounts)]
#[instruction(position_id: u64)]
pub struct OpenDcaPosition<'info> {
    pub executor: Signer<'info>,

    #[account(mut)]
    pub vault_config: Account<'info, VaultConfig>,

    #[account(
        mut,
        seeds = [SEED_EXEC, vault_config.key().as_ref(), executor.key().as_ref()],
        bump,
        constraint = executor_config.vault == vault_config.key() @ VaultError::Unauthorized,
        constraint = executor_config.executor == executor.key() @ VaultError::Unauthorized
    )]
    pub executor_config: Account<'info, ExecutorConfig>,

    #[account(
        seeds = [SEED_MINTS, vault_config.key().as_ref()],
        bump
    )]
    pub mint_whitelist: Account<'info, MintWhitelist>,

    /// CHECK: PDA authority signer
    #[account(
        seeds = [SEED_AUTH, vault_config.key().as_ref()],
        bump = vault_config.auth_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    pub input_mint: InterfaceAccount<'info, Mint>,
    pub output_mint: InterfaceAccount<'info, Mint>,

    #[account(mut)]
    pub source_vault_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Jupiter DCA position account (will be created by DCA program)
    pub dca_position_account: UncheckedAccount<'info>,

    #[account(
        init,
        payer = executor,
        space = 8 + DcaPosition::INIT_SPACE,
        seeds = [SEED_DCA_POS, vault_config.key().as_ref(), &position_id.to_le_bytes()],
        bump
    )]
    pub dca_position: Account<'info, DcaPosition>,

    /// CHECK: Verified against vault_config.allowed_dca_program
    pub dca_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[event]
pub struct DcaPositionOpened {
    pub executor: Pubkey,
    pub position_id: u64,
    pub dca_position_pubkey: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub deposited_in: u64,
    pub ts: i64,
}
```

**Verification**:

```bash
anchor build
```

**Done when**: Open DCA position compiles

---

## 🔚 Task 12: Implement Close DCA Position (15 min)

**Goal**: Add DCA position closing with refund tracking

**Add to** `#[program] mod dca_vault`:

```rust
    pub fn close_dca_position(
        ctx: Context<CloseDcaPosition>,
        close_ix_data: Vec<u8>,
    ) -> Result<()> {
        require_ix_data_size(&close_ix_data)?;

        let pos = &ctx.accounts.dca_position;
        require!(pos.active, VaultError::PositionNotActive);

        let cfg = &ctx.accounts.vault_config;

        // Ensure DCA program matches
        require_keys_eq!(
            ctx.accounts.dca_program.key(),
            cfg.allowed_dca_program,
            VaultError::DcaProgramMismatch
        );

        // Snapshot input ATA balance (to measure refund)
        let pre_balance = ctx.accounts.vault_input_ata.amount;

        // Build CPI
        let remaining = ctx.remaining_accounts;
        let vault_auth_key = ctx.accounts.vault_authority.key();

        let metas: Vec<AccountMeta> = remaining
            .iter()
            .map(|ai| AccountMeta {
                pubkey: *ai.key,
                is_signer: ai.is_signer || *ai.key == vault_auth_key,
                is_writable: ai.is_writable,
            })
            .collect();

        let ix = Instruction {
            program_id: ctx.accounts.dca_program.key(),
            accounts: metas,
            data: close_ix_data,
        };

        let mut infos: Vec<AccountInfo> = Vec::with_capacity(1 + remaining.len());
        infos.push(ctx.accounts.dca_program.to_account_info());
        infos.extend_from_slice(remaining);

        let signer_seeds: &[&[&[u8]]] = &[&[
            SEED_AUTH,
            cfg.key().as_ref(),
            &[cfg.auth_bump],
        ]];

        invoke_signed(&ix, &infos, signer_seeds)?;

        // Reload and compute refund
        ctx.accounts.vault_input_ata.reload()?;
        let post_balance = ctx.accounts.vault_input_ata.amount;
        let refund = post_balance.saturating_sub(pre_balance);

        // Update escrow tracking
        let cfg = &mut ctx.accounts.vault_config;
        cfg.total_escrowed = cfg.total_escrowed.saturating_sub(pos.deposited_in);

        // Mark position inactive
        let pos = &mut ctx.accounts.dca_position;
        pos.active = false;

        emit!(DcaPositionClosed {
            owner: ctx.accounts.owner.key(),
            position_id: pos.position_id,
            dca_position_pubkey: pos.dca_position_pubkey,
            refund,
            ts: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
```

**Add context and event**:

```rust
#[derive(Accounts)]
pub struct CloseDcaPosition<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [SEED_VAULT, owner.key().as_ref()],
        bump = vault_config.vault_bump,
        constraint = vault_config.owner == owner.key() @ VaultError::Unauthorized
    )]
    pub vault_config: Account<'info, VaultConfig>,

    /// CHECK: PDA authority signer
    #[account(
        seeds = [SEED_AUTH, vault_config.key().as_ref()],
        bump = vault_config.auth_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_DCA_POS, vault_config.key().as_ref(), &dca_position.position_id.to_le_bytes()],
        bump,
        constraint = dca_position.vault == vault_config.key() @ VaultError::Unauthorized
    )]
    pub dca_position: Account<'info, DcaPosition>,

    #[account(mut)]
    pub vault_input_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Verified against vault_config.allowed_dca_program
    pub dca_program: UncheckedAccount<'info>,
}

#[event]
pub struct DcaPositionClosed {
    pub owner: Pubkey,
    pub position_id: u64,
    pub dca_position_pubkey: Pubkey,
    pub refund: u64,
    pub ts: i64,
}
```

**Verification**:

```bash
anchor build
anchor keys list
# Update declare_id!() with the new program ID
anchor build
```

**Done when**: Full program compiles successfully

---

## 🧪 Testing Strategy

### Local Testing with Jupiter Programs

Jupiter programs are mainnet-only. To test locally:

1. **Fork mainnet programs to localnet**:

```bash
solana-test-validator \
  --clone JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 \
  --clone DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M \
  --url https://api.mainnet-beta.solana.com \
  --reset
```

1. **Build test client** (TypeScript recommended for Jupiter SDK integration)

2. **Test sequence**:
   - Initialize vault
   - Add mints to whitelist
   - Set executor
   - Deposit funds
   - Execute swaps (verify delta checks)
   - Open DCA position
   - Close DCA position
   - Withdraw funds

---

## 📋 Deployment Checklist

- [ ] Update `declare_id!()` with deployed program ID
- [ ] Audit all PDA seeds and bumps
- [ ] Test on devnet first
- [ ] Verify Jupiter program IDs match mainnet
- [ ] Document all external dependencies
- [ ] Set conservative caps for initial deployment
- [ ] Test pause/unpause emergency functions
- [ ] Monitor events for all operations

---

## 🔗 Key Program IDs (Mainnet)

```rust
// Jupiter Swap v6
// JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4

// Jupiter DCA
// DCA265Vj8a9CEuX1eb1LWRnDT7uK6q1xMipnNyatn23M

// Jupiter Limit Order v2
// j1o2qRpjcyUwEvwtcfhEQefh773ZgjxcVRry7LDqg5X
```

---

## 🎯 Success Criteria

**Core Vault** (Tasks 1-10):

- ✅ Funds custody in PDA
- ✅ Owner-only withdraw
- ✅ Executor can trade but not withdraw
- ✅ Whitelist enforced
- ✅ Caps and cooldown enforced
- ✅ Post-swap balance delta verification
- ✅ Replay protection via monotonic order IDs

**DCA Extension** (Tasks 11-12):

- ✅ DCA position tracking
- ✅ Escrow limits enforced at open
- ✅ Refund tracking at close
- ✅ Position count limits

---

## 📚 Resources

- [Jupiter Swap Docs](https://dev.jup.ag/docs/swap/build-swap-transaction)
- [Jupiter DCA CPI Example](https://github.com/jup-ag/dca-cpi-example)
- [Anchor Space Reference](https://www.anchor-lang.com/docs/references/space)
- [QuickNode: Fork Programs to Localnet](https://www.quicknode.com/guides/solana-development/accounts-and-data/fork-programs-to-localnet)
