use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, MintTo};

declare_id!("DOS4tkn1111111111111111111111111111111111111");

/// $DRONEOS Token Program
/// 
/// $DRONEOS Token operations:
/// - Token initialization
/// - Staking with lock periods
/// - Reward distribution
/// - Operator stake management

// Constants
const DECIMALS: u8 = 6;
const TOTAL_SUPPLY: u64 = 1_000_000_000 * 1_000_000; // 1B tokens
const BASE_APY_BPS: u64 = 1200; // 12% base APY
const MIN_STAKE: u64 = 100 * 1_000_000; // 100 DRONEOS minimum
const SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60;

#[program]
pub mod droneos_token {
    use super::*;

    /// Initialize the $DRONEOS token
    pub fn initialize(ctx: Context<InitializeToken>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.mint = ctx.accounts.mint.key();
        config.total_staked = 0;
        config.total_rewards_distributed = 0;
        config.stake_count = 0;
        config.bump = ctx.bumps.config;
        config.mint_bump = ctx.bumps.mint;
        
        Ok(())
    }

    /// Mint initial supply (one-time)
    pub fn mint_initial_supply(ctx: Context<MintInitialSupply>) -> Result<()> {
        let config = &ctx.accounts.config;
        
        // Can only mint once
        require!(
            ctx.accounts.treasury.amount == 0,
            ErrorCode::AlreadyMinted
        );

        let seeds = &[
            b"mint",
            &[config.mint_bump],
        ];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
                authority: ctx.accounts.mint.to_account_info(),
            },
            signer,
        );
        
        token::mint_to(cpi_ctx, TOTAL_SUPPLY)?;

        emit!(InitialSupplyMinted {
            amount: TOTAL_SUPPLY,
            treasury: ctx.accounts.treasury.key(),
        });

        Ok(())
    }

    /// Stake tokens
    pub fn stake(
        ctx: Context<Stake>,
        amount: u64,
        lock_days: u16,
    ) -> Result<()> {
        require!(amount >= MIN_STAKE, ErrorCode::BelowMinimumStake);
        
        let valid_locks = [0, 30, 90, 180, 365];
        require!(
            valid_locks.contains(&lock_days),
            ErrorCode::InvalidLockPeriod
        );

        let stake_account = &mut ctx.accounts.stake_account;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Calculate multiplier based on lock period
        let multiplier = match lock_days {
            0 => 10000,   // 1.0x
            30 => 11000,  // 1.1x
            90 => 12500,  // 1.25x
            180 => 15000, // 1.5x
            365 => 20000, // 2.0x
            _ => 10000,
        };

        // Transfer tokens to stake vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token.to_account_info(),
                to: ctx.accounts.stake_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update stake account
        stake_account.owner = ctx.accounts.user.key();
        stake_account.amount = amount;
        stake_account.staked_at = clock.unix_timestamp;
        stake_account.lock_duration = lock_days as i64 * 86400;
        stake_account.lock_until = clock.unix_timestamp + (lock_days as i64 * 86400);
        stake_account.multiplier = multiplier;
        stake_account.accumulated_rewards = 0;
        stake_account.last_claim_at = clock.unix_timestamp;
        stake_account.bump = ctx.bumps.stake_account;

        config.total_staked += amount;
        config.stake_count += 1;

        emit!(TokensStaked {
            user: ctx.accounts.user.key(),
            amount,
            lock_days,
            multiplier,
        });

        Ok(())
    }

    /// Claim staking rewards
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        let rewards = calculate_rewards(stake_account, clock.unix_timestamp)?;
        require!(rewards > 0, ErrorCode::NoRewardsToClaim);

        // Transfer rewards from treasury
        let seeds = &[b"config", &[config.bump]];
        let signer = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.rewards_vault.to_account_info(),
                to: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_ctx, rewards)?;

        stake_account.last_claim_at = clock.unix_timestamp;
        stake_account.accumulated_rewards += rewards;
        config.total_rewards_distributed += rewards;

        emit!(RewardsClaimed {
            user: ctx.accounts.user.key(),
            amount: rewards,
        });

        Ok(())
    }

    /// Unstake tokens
    pub fn unstake(ctx: Context<Unstake>, amount: Option<u64>) -> Result<()> {
        let stake_account = &mut ctx.accounts.stake_account;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Check lock period
        require!(
            clock.unix_timestamp >= stake_account.lock_until,
            ErrorCode::StakeLocked
        );

        let unstake_amount = amount.unwrap_or(stake_account.amount);
        require!(unstake_amount <= stake_account.amount, ErrorCode::InsufficientStake);

        // Claim any pending rewards first
        let rewards = calculate_rewards(stake_account, clock.unix_timestamp)?;

        // Transfer staked tokens back
        let seeds = &[b"config", &[config.bump]];
        let signer = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.stake_vault.to_account_info(),
                to: ctx.accounts.user_token.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_ctx, unstake_amount)?;

        // Transfer rewards if any
        if rewards > 0 {
            let reward_transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.rewards_vault.to_account_info(),
                    to: ctx.accounts.user_token.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                signer,
            );
            token::transfer(reward_transfer_ctx, rewards)?;
            config.total_rewards_distributed += rewards;
        }

        stake_account.amount -= unstake_amount;
        stake_account.last_claim_at = clock.unix_timestamp;
        config.total_staked -= unstake_amount;

        if stake_account.amount == 0 {
            config.stake_count -= 1;
        }

        emit!(TokensUnstaked {
            user: ctx.accounts.user.key(),
            amount: unstake_amount,
            rewards_claimed: rewards,
        });

        Ok(())
    }

    /// Create operator stake (for robot operators)
    pub fn create_operator_stake(
        ctx: Context<CreateOperatorStake>,
        amount: u64,
    ) -> Result<()> {
        require!(amount >= MIN_STAKE * 10, ErrorCode::BelowMinimumOperatorStake);

        let operator_stake = &mut ctx.accounts.operator_stake;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Transfer tokens to operator vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.operator_token.to_account_info(),
                to: ctx.accounts.operator_vault.to_account_info(),
                authority: ctx.accounts.operator.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        operator_stake.operator = ctx.accounts.operator.key();
        operator_stake.total_staked = amount;
        operator_stake.slashable_amount = amount;
        operator_stake.created_at = clock.unix_timestamp;
        operator_stake.last_slash_at = None;
        operator_stake.reputation = 5000; // Start at 50%
        operator_stake.bump = ctx.bumps.operator_stake;

        config.total_staked += amount;

        emit!(OperatorStakeCreated {
            operator: ctx.accounts.operator.key(),
            amount,
        });

        Ok(())
    }

    /// Slash operator stake (called by task_market on failures)
    pub fn slash_operator(
        ctx: Context<SlashOperator>,
        amount: u64,
        reason: String,
    ) -> Result<()> {
        require!(reason.len() <= 128, ErrorCode::ReasonTooLong);
        
        let operator_stake = &mut ctx.accounts.operator_stake;
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        // Maximum slash is 10% of slashable amount
        let max_slash = operator_stake.slashable_amount / 10;
        let actual_slash = amount.min(max_slash).min(operator_stake.slashable_amount);

        require!(actual_slash > 0, ErrorCode::NothingToSlash);

        // Transfer slashed tokens to treasury
        let seeds = &[b"config", &[config.bump]];
        let signer = &[&seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.operator_vault.to_account_info(),
                to: ctx.accounts.treasury.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer,
        );
        token::transfer(transfer_ctx, actual_slash)?;

        operator_stake.total_staked -= actual_slash;
        operator_stake.slashable_amount -= actual_slash;
        operator_stake.last_slash_at = Some(clock.unix_timestamp);
        
        // Reduce reputation
        let rep_penalty = (actual_slash * 1000 / operator_stake.total_staked.max(1)) as u16;
        operator_stake.reputation = operator_stake.reputation.saturating_sub(rep_penalty);

        config.total_staked -= actual_slash;

        emit!(OperatorSlashed {
            operator: operator_stake.operator,
            amount: actual_slash,
            reason,
            new_reputation: operator_stake.reputation,
        });

        Ok(())
    }

    /// Get pending rewards (view function)
    pub fn get_pending_rewards(ctx: Context<ViewStake>) -> Result<u64> {
        let clock = Clock::get()?;
        calculate_rewards(&ctx.accounts.stake_account, clock.unix_timestamp)
    }
}

// ============================================================================
// HELPERS
// ============================================================================

fn calculate_rewards(stake: &StakeAccount, current_time: i64) -> Result<u64> {
    let elapsed = (current_time - stake.last_claim_at) as u64;
    
    // Base reward calculation
    let base_reward = stake.amount
        .checked_mul(BASE_APY_BPS)
        .ok_or(ErrorCode::Overflow)?
        .checked_mul(elapsed)
        .ok_or(ErrorCode::Overflow)?
        / (10000 * SECONDS_PER_YEAR);
    
    // Apply multiplier
    let multiplied_reward = base_reward
        .checked_mul(stake.multiplier as u64)
        .ok_or(ErrorCode::Overflow)?
        / 10000;
    
    Ok(multiplied_reward)
}

// ============================================================================
// ACCOUNTS
// ============================================================================

#[derive(Accounts)]
pub struct InitializeToken<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + TokenConfig::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, TokenConfig>,
    
    #[account(
        init,
        payer = authority,
        seeds = [b"mint"],
        bump,
        mint::decimals = DECIMALS,
        mint::authority = mint,
    )]
    pub mint: Account<'info, Mint>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct MintInitialSupply<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, TokenConfig>,
    
    #[account(
        mut,
        seeds = [b"mint"],
        bump = config.mint_bump
    )]
    pub mint: Account<'info, Mint>,
    
    #[account(mut)]
    pub treasury: Account<'info, TokenAccount>,
    
    #[account(constraint = authority.key() == config.authority @ ErrorCode::Unauthorized)]
    pub authority: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, TokenConfig>,
    
    #[account(
        init,
        payer = user,
        space = 8 + StakeAccount::INIT_SPACE,
        seeds = [b"stake", user.key().as_ref()],
        bump
    )]
    pub stake_account: Account<'info, StakeAccount>,
    
    #[account(mut)]
    pub stake_vault: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = user_token.owner == user.key())]
    pub user_token: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, TokenConfig>,
    
    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub stake_account: Account<'info, StakeAccount>,
    
    #[account(mut)]
    pub rewards_vault: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = user_token.owner == user.key())]
    pub user_token: Account<'info, TokenAccount>,
    
    pub user: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, TokenConfig>,
    
    #[account(
        mut,
        seeds = [b"stake", user.key().as_ref()],
        bump = stake_account.bump,
        constraint = stake_account.owner == user.key() @ ErrorCode::Unauthorized
    )]
    pub stake_account: Account<'info, StakeAccount>,
    
    #[account(mut)]
    pub stake_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub rewards_vault: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = user_token.owner == user.key())]
    pub user_token: Account<'info, TokenAccount>,
    
    pub user: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CreateOperatorStake<'info> {
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, TokenConfig>,
    
    #[account(
        init,
        payer = operator,
        space = 8 + OperatorStake::INIT_SPACE,
        seeds = [b"operator", operator.key().as_ref()],
        bump
    )]
    pub operator_stake: Account<'info, OperatorStake>,
    
    #[account(mut)]
    pub operator_vault: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = operator_token.owner == operator.key())]
    pub operator_token: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub operator: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SlashOperator<'info> {
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, TokenConfig>,
    
    #[account(mut)]
    pub operator_stake: Account<'info, OperatorStake>,
    
    #[account(mut)]
    pub operator_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub treasury: Account<'info, TokenAccount>,
    
    /// CHECK: Verified by CPI from task_market
    pub authority: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ViewStake<'info> {
    pub stake_account: Account<'info, StakeAccount>,
}

// ============================================================================
// STATE
// ============================================================================

#[account]
#[derive(InitSpace)]
pub struct TokenConfig {
    pub authority: Pubkey,
    pub mint: Pubkey,
    pub total_staked: u64,
    pub total_rewards_distributed: u64,
    pub stake_count: u64,
    pub bump: u8,
    pub mint_bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub owner: Pubkey,
    pub amount: u64,
    pub staked_at: i64,
    pub lock_duration: i64,
    pub lock_until: i64,
    pub multiplier: u16,
    pub accumulated_rewards: u64,
    pub last_claim_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct OperatorStake {
    pub operator: Pubkey,
    pub total_staked: u64,
    pub slashable_amount: u64,
    pub created_at: i64,
    pub last_slash_at: Option<i64>,
    pub reputation: u16,
    pub bump: u8,
}

// ============================================================================
// EVENTS
// ============================================================================

#[event]
pub struct InitialSupplyMinted {
    pub amount: u64,
    pub treasury: Pubkey,
}

#[event]
pub struct TokensStaked {
    pub user: Pubkey,
    pub amount: u64,
    pub lock_days: u16,
    pub multiplier: u16,
}

#[event]
pub struct RewardsClaimed {
    pub user: Pubkey,
    pub amount: u64,
}

#[event]
pub struct TokensUnstaked {
    pub user: Pubkey,
    pub amount: u64,
    pub rewards_claimed: u64,
}

#[event]
pub struct OperatorStakeCreated {
    pub operator: Pubkey,
    pub amount: u64,
}

#[event]
pub struct OperatorSlashed {
    pub operator: Pubkey,
    pub amount: u64,
    pub reason: String,
    pub new_reputation: u16,
}

// ============================================================================
// ERRORS
// ============================================================================

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized,
    
    #[msg("Initial supply already minted")]
    AlreadyMinted,
    
    #[msg("Below minimum stake amount (100 DRON)")]
    BelowMinimumStake,
    
    #[msg("Below minimum operator stake (1000 DRON)")]
    BelowMinimumOperatorStake,
    
    #[msg("Invalid lock period")]
    InvalidLockPeriod,
    
    #[msg("Stake is still locked")]
    StakeLocked,
    
    #[msg("Insufficient staked amount")]
    InsufficientStake,
    
    #[msg("No rewards to claim")]
    NoRewardsToClaim,
    
    #[msg("Nothing to slash")]
    NothingToSlash,
    
    #[msg("Reason too long (max 128 chars)")]
    ReasonTooLong,
    
    #[msg("Arithmetic overflow")]
    Overflow,
}
