use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("DOS4pay1111111111111111111111111111111111111");

/// $DRONEOS Payment Streams Program
/// 
/// X402 Protocol Implementation:
/// - Real-time streaming payments
/// - Automatic escrow management
/// - Pay-per-second billing
/// - Auto-termination on insufficient funds

#[program]
pub mod payment_streams {
    use super::*;

    /// Initialize the payment streams program
    pub fn initialize(ctx: Context<InitializeProgram>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.fee_basis_points = 10; // 0.1% fee
        config.min_stream_duration = 60; // 1 minute
        config.max_stream_duration = 30 * 86400; // 30 days
        config.total_streams = 0;
        config.total_volume = 0;
        config.bump = ctx.bumps.config;
        
        Ok(())
    }

    /// Create a new payment stream
    pub fn create_stream(
        ctx: Context<CreateStream>,
        rate_per_second: u64,
        max_duration: i64,
        grace_period: i64,
        auto_terminate: bool,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        let stream = &mut ctx.accounts.stream;
        let clock = Clock::get()?;

        // Validate parameters
        require!(rate_per_second > 0, ErrorCode::InvalidRate);
        require!(
            max_duration >= config.min_stream_duration as i64 && 
            max_duration <= config.max_stream_duration as i64,
            ErrorCode::InvalidDuration
        );
        require!(grace_period >= 0 && grace_period <= 300, ErrorCode::InvalidGracePeriod);

        // Calculate required escrow
        let required_escrow = rate_per_second
            .checked_mul(max_duration as u64)
            .ok_or(ErrorCode::Overflow)?;
        
        require!(
            ctx.accounts.payer_token.amount >= required_escrow,
            ErrorCode::InsufficientFunds
        );

        // Transfer to escrow
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer_token.to_account_info(),
                to: ctx.accounts.escrow.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, required_escrow)?;

        // Initialize stream
        stream.payer = ctx.accounts.payer.key();
        stream.payee = ctx.accounts.payee.key();
        stream.rate_per_second = rate_per_second;
        stream.max_duration = max_duration;
        stream.grace_period = grace_period;
        stream.auto_terminate = auto_terminate;
        stream.status = StreamStatus::Pending;
        stream.created_at = clock.unix_timestamp;
        stream.started_at = 0;
        stream.last_tick_at = 0;
        stream.total_paid = 0;
        stream.total_ticks = 0;
        stream.escrow_balance = required_escrow;
        stream.task_id = None;
        stream.escrow_bump = ctx.bumps.escrow;
        stream.bump = ctx.bumps.stream;

        emit!(StreamCreated {
            stream: stream.key(),
            payer: stream.payer,
            payee: stream.payee,
            rate_per_second,
            escrow_amount: required_escrow,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Start the payment stream
    pub fn start_stream(ctx: Context<StartStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        let clock = Clock::get()?;

        require!(stream.status == StreamStatus::Pending, ErrorCode::StreamNotPending);

        stream.status = StreamStatus::Active;
        stream.started_at = clock.unix_timestamp;
        stream.last_tick_at = clock.unix_timestamp;

        emit!(StreamStarted {
            stream: stream.key(),
            started_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Process a payment tick - transfers accumulated payment to payee
    pub fn tick(ctx: Context<Tick>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        let clock = Clock::get()?;

        require!(stream.status == StreamStatus::Active, ErrorCode::StreamNotActive);

        // Calculate time elapsed and amount due
        let elapsed = clock.unix_timestamp - stream.last_tick_at;
        require!(elapsed > 0, ErrorCode::NoTimeElapsed);

        let amount_due = stream.rate_per_second
            .checked_mul(elapsed as u64)
            .ok_or(ErrorCode::Overflow)?;

        // Check if escrow has enough
        if amount_due > stream.escrow_balance {
            if stream.auto_terminate {
                // Pay remaining balance and terminate
                let remaining = stream.escrow_balance;
                if remaining > 0 {
                    transfer_from_escrow(
                        &ctx.accounts.escrow,
                        &ctx.accounts.payee_token,
                        &stream,
                        remaining,
                        &ctx.accounts.token_program,
                    )?;
                }
                
                stream.total_paid += remaining;
                stream.escrow_balance = 0;
                stream.status = StreamStatus::Completed;
                
                emit!(StreamTerminated {
                    stream: stream.key(),
                    reason: "Escrow depleted".to_string(),
                    total_paid: stream.total_paid,
                    timestamp: clock.unix_timestamp,
                });
                
                return Ok(());
            } else {
                return Err(ErrorCode::InsufficientEscrow.into());
            }
        }

        // Transfer payment
        transfer_from_escrow(
            &ctx.accounts.escrow,
            &ctx.accounts.payee_token,
            &stream,
            amount_due,
            &ctx.accounts.token_program,
        )?;

        // Update stream state
        stream.last_tick_at = clock.unix_timestamp;
        stream.total_paid += amount_due;
        stream.total_ticks += 1;
        stream.escrow_balance -= amount_due;

        emit!(StreamTick {
            stream: stream.key(),
            tick_number: stream.total_ticks,
            amount: amount_due,
            total_paid: stream.total_paid,
            escrow_remaining: stream.escrow_balance,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Pause the stream
    pub fn pause_stream(ctx: Context<ControlStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        let clock = Clock::get()?;

        require!(stream.status == StreamStatus::Active, ErrorCode::StreamNotActive);

        stream.status = StreamStatus::Paused;

        emit!(StreamPaused {
            stream: stream.key(),
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Resume a paused stream
    pub fn resume_stream(ctx: Context<ControlStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        let clock = Clock::get()?;

        require!(stream.status == StreamStatus::Paused, ErrorCode::StreamNotPaused);

        stream.status = StreamStatus::Active;
        stream.last_tick_at = clock.unix_timestamp; // Reset tick timer

        emit!(StreamResumed {
            stream: stream.key(),
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Terminate the stream and refund remaining escrow
    pub fn terminate_stream(ctx: Context<TerminateStream>, reason: String) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        let clock = Clock::get()?;

        require!(
            stream.status == StreamStatus::Active || 
            stream.status == StreamStatus::Paused ||
            stream.status == StreamStatus::Pending,
            ErrorCode::StreamAlreadyTerminated
        );

        // Process final tick if active
        if stream.status == StreamStatus::Active && stream.last_tick_at > 0 {
            let elapsed = clock.unix_timestamp - stream.last_tick_at;
            let final_payment = stream.rate_per_second
                .checked_mul(elapsed as u64)
                .ok_or(ErrorCode::Overflow)?
                .min(stream.escrow_balance);

            if final_payment > 0 {
                transfer_from_escrow(
                    &ctx.accounts.escrow,
                    &ctx.accounts.payee_token,
                    &stream,
                    final_payment,
                    &ctx.accounts.token_program,
                )?;
                stream.total_paid += final_payment;
                stream.escrow_balance -= final_payment;
            }
        }

        // Refund remaining escrow to payer
        let refund = stream.escrow_balance;
        if refund > 0 {
            transfer_from_escrow(
                &ctx.accounts.escrow,
                &ctx.accounts.payer_token,
                &stream,
                refund,
                &ctx.accounts.token_program,
            )?;
            stream.escrow_balance = 0;
        }

        stream.status = StreamStatus::Completed;

        emit!(StreamTerminated {
            stream: stream.key(),
            reason,
            total_paid: stream.total_paid,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Top up escrow balance
    pub fn top_up_escrow(ctx: Context<TopUpEscrow>, amount: u64) -> Result<()> {
        let stream = &mut ctx.accounts.stream;

        require!(
            stream.status != StreamStatus::Completed && 
            stream.status != StreamStatus::Cancelled,
            ErrorCode::StreamAlreadyTerminated
        );

        // Transfer to escrow
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.payer_token.to_account_info(),
                to: ctx.accounts.escrow.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        stream.escrow_balance += amount;

        emit!(EscrowToppedUp {
            stream: stream.key(),
            amount,
            new_balance: stream.escrow_balance,
        });

        Ok(())
    }

    /// Cancel a pending stream (before start)
    pub fn cancel_stream(ctx: Context<CancelStream>) -> Result<()> {
        let stream = &mut ctx.accounts.stream;

        require!(stream.status == StreamStatus::Pending, ErrorCode::StreamNotPending);

        // Refund full escrow
        let refund = stream.escrow_balance;
        if refund > 0 {
            transfer_from_escrow(
                &ctx.accounts.escrow,
                &ctx.accounts.payer_token,
                &stream,
                refund,
                &ctx.accounts.token_program,
            )?;
            stream.escrow_balance = 0;
        }

        stream.status = StreamStatus::Cancelled;

        emit!(StreamCancelled {
            stream: stream.key(),
            refunded: refund,
        });

        Ok(())
    }

    /// Link stream to a task (called by task_market program)
    pub fn link_to_task(ctx: Context<LinkToTask>, task_id: Pubkey) -> Result<()> {
        let stream = &mut ctx.accounts.stream;
        
        require!(stream.task_id.is_none(), ErrorCode::StreamAlreadyLinked);
        
        stream.task_id = Some(task_id);

        Ok(())
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn transfer_from_escrow<'info>(
    escrow: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    stream: &Account<'info, PaymentStream>,
    amount: u64,
    token_program: &Program<'info, Token>,
) -> Result<()> {
    let seeds = &[
        b"escrow",
        stream.to_account_info().key.as_ref(),
        &[stream.escrow_bump],
    ];
    let signer = &[&seeds[..]];

    let transfer_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        Transfer {
            from: escrow.to_account_info(),
            to: to.to_account_info(),
            authority: escrow.to_account_info(),
        },
        signer,
    );
    token::transfer(transfer_ctx, amount)?;

    Ok(())
}

// ============================================================================
// ACCOUNTS
// ============================================================================

#[derive(Accounts)]
pub struct InitializeProgram<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + ProgramConfig::INIT_SPACE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, ProgramConfig>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateStream<'info> {
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, ProgramConfig>,
    
    #[account(
        init,
        payer = payer,
        space = 8 + PaymentStream::INIT_SPACE,
        seeds = [b"stream", payer.key().as_ref(), payee.key().as_ref(), &Clock::get()?.unix_timestamp.to_le_bytes()],
        bump
    )]
    pub stream: Account<'info, PaymentStream>,
    
    #[account(
        init,
        payer = payer,
        seeds = [b"escrow", stream.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow,
    )]
    pub escrow: Account<'info, TokenAccount>,
    
    pub mint: Account<'info, anchor_spl::token::Mint>,
    
    #[account(
        mut,
        constraint = payer_token.owner == payer.key(),
        constraint = payer_token.mint == mint.key()
    )]
    pub payer_token: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    /// CHECK: Just storing the payee address
    pub payee: AccountInfo<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct StartStream<'info> {
    #[account(
        mut,
        constraint = stream.payer == payer.key() @ ErrorCode::Unauthorized
    )]
    pub stream: Account<'info, PaymentStream>,
    
    pub payer: Signer<'info>,
}

#[derive(Accounts)]
pub struct Tick<'info> {
    #[account(mut)]
    pub stream: Account<'info, PaymentStream>,
    
    #[account(
        mut,
        seeds = [b"escrow", stream.key().as_ref()],
        bump = stream.escrow_bump
    )]
    pub escrow: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        constraint = payee_token.owner == stream.payee
    )]
    pub payee_token: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ControlStream<'info> {
    #[account(
        mut,
        constraint = stream.payer == authority.key() @ ErrorCode::Unauthorized
    )]
    pub stream: Account<'info, PaymentStream>,
    
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct TerminateStream<'info> {
    #[account(
        mut,
        constraint = stream.payer == authority.key() || stream.payee == authority.key() @ ErrorCode::Unauthorized
    )]
    pub stream: Account<'info, PaymentStream>,
    
    #[account(
        mut,
        seeds = [b"escrow", stream.key().as_ref()],
        bump = stream.escrow_bump
    )]
    pub escrow: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = payer_token.owner == stream.payer)]
    pub payer_token: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = payee_token.owner == stream.payee)]
    pub payee_token: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct TopUpEscrow<'info> {
    #[account(mut)]
    pub stream: Account<'info, PaymentStream>,
    
    #[account(
        mut,
        seeds = [b"escrow", stream.key().as_ref()],
        bump = stream.escrow_bump
    )]
    pub escrow: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = payer_token.owner == payer.key())]
    pub payer_token: Account<'info, TokenAccount>,
    
    #[account(constraint = payer.key() == stream.payer @ ErrorCode::Unauthorized)]
    pub payer: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelStream<'info> {
    #[account(
        mut,
        constraint = stream.payer == payer.key() @ ErrorCode::Unauthorized
    )]
    pub stream: Account<'info, PaymentStream>,
    
    #[account(
        mut,
        seeds = [b"escrow", stream.key().as_ref()],
        bump = stream.escrow_bump
    )]
    pub escrow: Account<'info, TokenAccount>,
    
    #[account(mut, constraint = payer_token.owner == payer.key())]
    pub payer_token: Account<'info, TokenAccount>,
    
    pub payer: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct LinkToTask<'info> {
    #[account(mut)]
    pub stream: Account<'info, PaymentStream>,
    
    /// CHECK: Verified by CPI caller
    pub task_market_program: AccountInfo<'info>,
}

// ============================================================================
// STATE
// ============================================================================

#[account]
#[derive(InitSpace)]
pub struct ProgramConfig {
    pub authority: Pubkey,
    pub fee_basis_points: u16,
    pub min_stream_duration: u32,
    pub max_stream_duration: u32,
    pub total_streams: u64,
    pub total_volume: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PaymentStream {
    pub payer: Pubkey,
    pub payee: Pubkey,
    pub rate_per_second: u64,
    pub max_duration: i64,
    pub grace_period: i64,
    pub auto_terminate: bool,
    pub status: StreamStatus,
    pub created_at: i64,
    pub started_at: i64,
    pub last_tick_at: i64,
    pub total_paid: u64,
    pub total_ticks: u32,
    pub escrow_balance: u64,
    pub task_id: Option<Pubkey>,
    pub escrow_bump: u8,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum StreamStatus {
    Pending,
    Active,
    Paused,
    Completed,
    Cancelled,
    Disputed,
}

// ============================================================================
// EVENTS
// ============================================================================

#[event]
pub struct StreamCreated {
    pub stream: Pubkey,
    pub payer: Pubkey,
    pub payee: Pubkey,
    pub rate_per_second: u64,
    pub escrow_amount: u64,
    pub timestamp: i64,
}

#[event]
pub struct StreamStarted {
    pub stream: Pubkey,
    pub started_at: i64,
}

#[event]
pub struct StreamTick {
    pub stream: Pubkey,
    pub tick_number: u32,
    pub amount: u64,
    pub total_paid: u64,
    pub escrow_remaining: u64,
    pub timestamp: i64,
}

#[event]
pub struct StreamPaused {
    pub stream: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct StreamResumed {
    pub stream: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct StreamTerminated {
    pub stream: Pubkey,
    pub reason: String,
    pub total_paid: u64,
    pub timestamp: i64,
}

#[event]
pub struct StreamCancelled {
    pub stream: Pubkey,
    pub refunded: u64,
}

#[event]
pub struct EscrowToppedUp {
    pub stream: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
}

// ============================================================================
// ERRORS
// ============================================================================

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized access")]
    Unauthorized,
    
    #[msg("Invalid payment rate")]
    InvalidRate,
    
    #[msg("Invalid stream duration")]
    InvalidDuration,
    
    #[msg("Invalid grace period (max 300 seconds)")]
    InvalidGracePeriod,
    
    #[msg("Insufficient funds for escrow")]
    InsufficientFunds,
    
    #[msg("Insufficient escrow balance")]
    InsufficientEscrow,
    
    #[msg("Stream is not pending")]
    StreamNotPending,
    
    #[msg("Stream is not active")]
    StreamNotActive,
    
    #[msg("Stream is not paused")]
    StreamNotPaused,
    
    #[msg("Stream is already terminated")]
    StreamAlreadyTerminated,
    
    #[msg("Stream is already linked to a task")]
    StreamAlreadyLinked,
    
    #[msg("No time has elapsed since last tick")]
    NoTimeElapsed,
    
    #[msg("Arithmetic overflow")]
    Overflow,
}
