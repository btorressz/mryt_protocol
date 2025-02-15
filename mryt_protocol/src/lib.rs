use anchor_lang::prelude::*;
use anchor_lang::solana_program::clock::Clock;
use anchor_spl::token::{self, Burn, Mint, MintTo, Token, TokenAccount, Transfer};

declare_id!("BExFhCky6QHhYsr47ji5d3PRRmCeLXR1ujhPJYa2ujsM");

/// Constants for time-lock and withdrawal cap.
const MIN_LOCK_PERIOD: i64 = 7 * 24 * 60 * 60; // 7 days in seconds
const MAX_WITHDRAWAL_PERCENT: u8 = 20; // Maximum 20% of a user's staked amount per withdrawal

#[program]
pub mod mryt_protocol {
    use super::*;

    /// Initializes the protocol configuration and creates the MRYT mint.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.total_staked = 0;
        config.total_yield = 0;
        config.total_mryt_supply = 0;
        Ok(())
    }

    /// Deposits LP tokens into the vault to mint MRYT at a 1:1 ratio.
    /// Also updates the user's staked position (which is time-locked).
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        // Transfer LP tokens from the user to the vault.
        token::transfer(ctx.accounts.into_transfer_to_vault_context(), amount)?;

        {
            // Update global staked amount.
            let config = &mut ctx.accounts.config;
            config.total_staked = config
                .total_staked
                .checked_add(amount)
                .ok_or(ErrorCode::MathOverflow)?;
        }

        // Mint equivalent MRYT tokens to the user.
        token::mint_to(ctx.accounts.into_mint_to_context(), amount)?;

        {
            // Update the MRYT total supply.
            let config = &mut ctx.accounts.config;
            config.total_mryt_supply = config
                .total_mryt_supply
                .checked_add(amount)
                .ok_or(ErrorCode::MathOverflow)?;
        }

        {
            // Update (or initialize) the user's staked position.
            let staked_position = &mut ctx.accounts.staked_position;
            staked_position.user = ctx.accounts.authority.key();
            staked_position.amount = staked_position
                .amount
                .checked_add(amount)
                .ok_or(ErrorCode::MathOverflow)?;
            // Reset the deposit time to enforce a fresh lock period.
            staked_position.deposit_time = Clock::get()?.unix_timestamp;
        }

        Ok(())
    }

    /// Withdraws LP tokens by burning MRYT tokens.
    /// Enforces:
    /// - A 7-day lock period on staked funds.
    /// - A cap limiting withdrawal to 20% of the user's staked position.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let staked_position = &ctx.accounts.staked_position;
        let current_time = Clock::get()?.unix_timestamp;
        require!(
            current_time - staked_position.deposit_time >= MIN_LOCK_PERIOD,
            ErrorCode::EarlyWithdrawal
        );

        // Enforce a withdrawal cap: max 20% of the user's staked tokens.
        let max_allowed = staked_position
            .amount
            .checked_mul(MAX_WITHDRAWAL_PERCENT as u64)
            .ok_or(ErrorCode::MathOverflow)?
            .checked_div(100)
            .ok_or(ErrorCode::MathOverflow)?;
        require!(amount <= max_allowed, ErrorCode::WithdrawalTooHigh);

        // Burn MRYT tokens from the user.
        token::burn(ctx.accounts.into_burn_context(), amount)?;

        {
            let config = &mut ctx.accounts.config;
            config.total_mryt_supply = config
                .total_mryt_supply
                .checked_sub(amount)
                .ok_or(ErrorCode::MathOverflow)?;
        }

        // Transfer LP tokens from the vault back to the user.
        token::transfer(ctx.accounts.into_transfer_to_user_context(), amount)?;

        {
            let config = &mut ctx.accounts.config;
            config.total_staked = config
                .total_staked
                .checked_sub(amount)
                .ok_or(ErrorCode::MathOverflow)?;
        }

        {
            let staked_position = &mut ctx.accounts.staked_position;
            staked_position.amount = staked_position
                .amount
                .checked_sub(amount)
                .ok_or(ErrorCode::MathOverflow)?;
        }

        Ok(())
    }

    /// Simulates yield accrual by adding yield to the protocol’s global state.
    /// (In a real implementation, this yield would come from actual DeFi strategies.)
    pub fn accrue_yield(ctx: Context<AccrueYield>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        // For simulation, accrue 1% of the current staked amount.
        let yield_amount = config.total_staked / 100;
        config.total_yield = config
            .total_yield
            .checked_add(yield_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Auto-compounds yield by reinvesting 50% of the accrued yield back into the staked pool.
    pub fn auto_compound_yield(ctx: Context<AutoCompoundYield>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        // Reinvest 50% of the accrued yield.
        let reinvest_amount = config.total_yield / 2;
        msg!(
            "Auto-compounding {} yield tokens into staked funds",
            reinvest_amount
        );

        config.total_yield = config
            .total_yield
            .checked_sub(reinvest_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        config.total_staked = config
            .total_staked
            .checked_add(reinvest_amount)
            .ok_or(ErrorCode::MathOverflow)?;
        Ok(())
    }

    /// Calculates and logs the current APY (annual percentage yield) based on accrued yield.
    pub fn calculate_apy(ctx: Context<CalculateApy>) -> Result<()> {
        let config = &ctx.accounts.config;
        let total_staked = config.total_staked as f64;
        if total_staked == 0.0 {
            msg!("No funds staked. APY is 0%");
            return Ok(());
        }
        let total_yield = config.total_yield as f64;
        let apy = (total_yield / total_staked) * 100.0; // Simplified APY calculation
        msg!("Current APY: {:.2}%", apy);
        Ok(())
    }
}

/// Global protocol configuration.
#[account]
pub struct Config {
    pub authority: Pubkey,
    pub total_staked: u64,
    pub total_yield: u64,
    pub total_mryt_supply: u64,
}
impl Config {
    // 32 (Pubkey) + 3 * 8 = 32 + 24 = 56 bytes, plus 8 bytes for the discriminator.
    const LEN: usize = 8 + 32 + 8 + 8 + 8;
}

/// Stores each user's staked position, including deposit time for the vesting period.
#[account]
pub struct StakedPosition {
    pub user: Pubkey,
    pub deposit_time: i64, // Unix timestamp
    pub amount: u64,
}
impl StakedPosition {
    // 32 (Pubkey) + 8 + 8 = 48 bytes, plus 8 bytes for the discriminator.
    const LEN: usize = 8 + 32 + 8 + 8;
}

/// Enum for supported collateral assets.
/// (Currently, we only support LP tokens. Expand as needed.)
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum CollateralAsset {
    LP_Token,
}

/// Accounts for initialization.
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = authority, space = Config::LEN)]
    pub config: Account<'info, Config>,
    /// CHECK: Authority creating the config.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// MRYT token mint.
    #[account(
        init,
        mint::decimals = 6,
        mint::authority = config,
        payer = authority
    )]
    pub mryt_mint: Account<'info, Mint>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    pub token_program: Program<'info, Token>,
}

/// Accounts for deposit.
#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
    /// CHECK: User depositing funds.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The SPL token account holding the user's LP tokens.
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    /// The vault account that will hold the LP tokens.
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    /// MRYT token mint.
    #[account(mut)]
    pub mryt_mint: Account<'info, Mint>,
    /// The user's token account to receive minted MRYT tokens.
    #[account(mut)]
    pub user_mryt: Account<'info, TokenAccount>,
    /// The user's staked position account (PDA based on user's key).
    #[account(
        init_if_needed,
        payer = authority,
        space = StakedPosition::LEN,
        seeds = [b"staked", authority.key().as_ref()],
        bump
    )]
    pub staked_position: Account<'info, StakedPosition>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

/// Accounts for withdrawal.
#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
    /// CHECK: User withdrawing funds.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// The user's MRYT token account.
    #[account(mut)]
    pub user_mryt: Account<'info, TokenAccount>,
    /// MRYT token mint.
    #[account(mut)]
    pub mryt_mint: Account<'info, Mint>,
    /// The vault holding LP tokens.
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,
    /// The user's token account to receive LP tokens.
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    /// The user's staked position account.
    #[account(mut, seeds = [b"staked", authority.key().as_ref()], bump)]
    pub staked_position: Account<'info, StakedPosition>,
    pub token_program: Program<'info, Token>,
}

/// Accounts for simulating yield accrual.
#[derive(Accounts)]
pub struct AccrueYield<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
}

/// Accounts for auto-compounding yield.
#[derive(Accounts)]
pub struct AutoCompoundYield<'info> {
    #[account(mut)]
    pub config: Account<'info, Config>,
}

/// Accounts for calculating APY.
#[derive(Accounts)]
pub struct CalculateApy<'info> {
    pub config: Account<'info, Config>,
}

/// Helper implementations for CPI contexts.
impl<'info> Deposit<'info> {
    fn into_transfer_to_vault_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.user_token_account.to_account_info().clone(),
            to: self.vault_token_account.to_account_info().clone(),
            authority: self.authority.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn into_mint_to_context(&self) -> CpiContext<'_, '_, '_, 'info, MintTo<'info>> {
        // Using the config as the mint authority (in production, consider using a PDA).
        let cpi_accounts = MintTo {
            mint: self.mryt_mint.to_account_info().clone(),
            to: self.user_mryt.to_account_info().clone(),
            authority: self.config.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

impl<'info> Withdraw<'info> {
    fn into_burn_context(&self) -> CpiContext<'_, '_, '_, 'info, Burn<'info>> {
        let cpi_accounts = Burn {
            mint: self.mryt_mint.to_account_info().clone(),
            from: self.user_mryt.to_account_info().clone(),
            authority: self.authority.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }

    fn into_transfer_to_user_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        // Here the config acts as the authority over the vault.
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info().clone(),
            to: self.user_token_account.to_account_info().clone(),
            authority: self.config.to_account_info().clone(),
        };
        CpiContext::new(self.token_program.to_account_info().clone(), cpi_accounts)
    }
}

/// Custom error codes.
#[error_code]
pub enum ErrorCode {
    #[msg("Math operation overflowed")]
    MathOverflow,
    #[msg("Withdrawal attempted before the minimum lock period elapsed")]
    EarlyWithdrawal,
    #[msg("Withdrawal amount exceeds the maximum allowed limit")]
    WithdrawalTooHigh,
}
