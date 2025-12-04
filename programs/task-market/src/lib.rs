use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

declare_id!("DOS4mkt1111111111111111111111111111111111111");

/// $DRONEOS Task Market Program
/// 
/// On-chain labor marketplace for robots:
/// - Task creation and management
/// - Bidding system
/// - Assignment and execution tracking
/// - Completion verification

#[program]
pub mod task_market {
    use super::*;

    /// Initialize the task market
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let market = &mut ctx.accounts.market;
        market.authority = ctx.accounts.authority.key();
        market.total_tasks = 0;
        market.total_completed = 0;
        market.total_volume = 0;
        market.fee_basis_points = 50; // 0.5% platform fee
        market.bump = ctx.bumps.market;
        
        Ok(())
    }

    /// Create a new task
    pub fn create_task(
        ctx: Context<CreateTask>,
        title: String,
        description: String,
        robot_class: u8,
        capabilities: Vec<u8>,
        min_reputation: u16,
        reward: u64,
        rate_per_second: u64,
        estimated_duration: u32,
        priority: u8,
        expires_in: i64,
    ) -> Result<()> {
        require!(title.len() <= 64, ErrorCode::TitleTooLong);
        require!(description.len() <= 256, ErrorCode::DescriptionTooLong);
        require!(capabilities.len() <= 5, ErrorCode::TooManyCapabilities);
        require!(reward > 0, ErrorCode::InvalidReward);
        require!(priority >= 1 && priority <= 5, ErrorCode::InvalidPriority);
        require!(expires_in > 0 && expires_in <= 7 * 86400, ErrorCode::InvalidExpiration);

        let task = &mut ctx.accounts.task;
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        task.creator = ctx.accounts.creator.key();
        task.title = title.clone();
        task.description = description;
        task.robot_class = robot_class;
        task.required_capabilities = capabilities;
        task.min_reputation = min_reputation;
        task.reward = reward;
        task.rate_per_second = rate_per_second;
        task.estimated_duration = estimated_duration;
        task.priority = priority;
        task.status = TaskStatus::Open;
        task.created_at = clock.unix_timestamp;
        task.expires_at = clock.unix_timestamp + expires_in;
        task.assigned_robot = None;
        task.assigned_at = None;
        task.started_at = None;
        task.completed_at = None;
        task.stream_id = None;
        task.progress = 0;
        task.bids_count = 0;
        task.bump = ctx.bumps.task;

        market.total_tasks += 1;

        emit!(TaskCreated {
            task: task.key(),
            creator: task.creator,
            title,
            reward,
            expires_at: task.expires_at,
        });

        Ok(())
    }

    /// Submit a bid on a task
    pub fn submit_bid(
        ctx: Context<SubmitBid>,
        proposed_rate: u64,
        estimated_duration: u32,
        message: String,
    ) -> Result<()> {
        require!(message.len() <= 128, ErrorCode::MessageTooLong);

        let task = &mut ctx.accounts.task;
        let bid = &mut ctx.accounts.bid;
        let clock = Clock::get()?;

        // Verify task is open
        require!(task.status == TaskStatus::Open, ErrorCode::TaskNotOpen);
        require!(clock.unix_timestamp < task.expires_at, ErrorCode::TaskExpired);

        // TODO: Verify robot meets requirements via CPI to identity-registry
        // For now, just check robot is provided
        
        bid.task = task.key();
        bid.robot = ctx.accounts.robot.key();
        bid.operator = ctx.accounts.operator.key();
        bid.proposed_rate = proposed_rate;
        bid.estimated_duration = estimated_duration;
        bid.message = message;
        bid.status = BidStatus::Pending;
        bid.submitted_at = clock.unix_timestamp;
        bid.bump = ctx.bumps.bid;

        task.bids_count += 1;

        emit!(BidSubmitted {
            task: task.key(),
            bid: bid.key(),
            robot: bid.robot,
            proposed_rate,
            estimated_duration,
        });

        Ok(())
    }

    /// Accept a bid and assign the task
    pub fn accept_bid(ctx: Context<AcceptBid>) -> Result<()> {
        let task = &mut ctx.accounts.task;
        let bid = &mut ctx.accounts.bid;
        let clock = Clock::get()?;

        require!(task.status == TaskStatus::Open, ErrorCode::TaskNotOpen);
        require!(bid.status == BidStatus::Pending, ErrorCode::BidNotPending);
        require!(task.creator == ctx.accounts.creator.key(), ErrorCode::Unauthorized);

        // Update bid status
        bid.status = BidStatus::Accepted;

        // Assign task
        task.status = TaskStatus::Assigned;
        task.assigned_robot = Some(bid.robot);
        task.assigned_at = Some(clock.unix_timestamp);
        task.rate_per_second = bid.proposed_rate;

        // TODO: Create payment stream via CPI to payment-streams
        // task.stream_id = Some(stream_pubkey);

        emit!(TaskAssigned {
            task: task.key(),
            robot: bid.robot,
            rate: bid.proposed_rate,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Reject a bid
    pub fn reject_bid(ctx: Context<RejectBid>) -> Result<()> {
        let task = &ctx.accounts.task;
        let bid = &mut ctx.accounts.bid;

        require!(task.creator == ctx.accounts.creator.key(), ErrorCode::Unauthorized);
        require!(bid.status == BidStatus::Pending, ErrorCode::BidNotPending);

        bid.status = BidStatus::Rejected;

        emit!(BidRejected {
            task: task.key(),
            bid: bid.key(),
        });

        Ok(())
    }

    /// Withdraw a bid (by robot operator)
    pub fn withdraw_bid(ctx: Context<WithdrawBid>) -> Result<()> {
        let bid = &mut ctx.accounts.bid;

        require!(bid.operator == ctx.accounts.operator.key(), ErrorCode::Unauthorized);
        require!(bid.status == BidStatus::Pending, ErrorCode::BidNotPending);

        bid.status = BidStatus::Withdrawn;

        emit!(BidWithdrawn {
            bid: bid.key(),
        });

        Ok(())
    }

    /// Start task execution (by assigned robot)
    pub fn start_task(ctx: Context<ExecuteTask>) -> Result<()> {
        let task = &mut ctx.accounts.task;
        let clock = Clock::get()?;

        require!(task.status == TaskStatus::Assigned, ErrorCode::TaskNotAssigned);
        require!(
            task.assigned_robot == Some(ctx.accounts.robot.key()),
            ErrorCode::NotAssignedRobot
        );

        task.status = TaskStatus::InProgress;
        task.started_at = Some(clock.unix_timestamp);

        // TODO: Start payment stream via CPI

        emit!(TaskStarted {
            task: task.key(),
            robot: ctx.accounts.robot.key(),
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Update task progress
    pub fn update_progress(ctx: Context<ExecuteTask>, progress: u8) -> Result<()> {
        let task = &mut ctx.accounts.task;

        require!(task.status == TaskStatus::InProgress, ErrorCode::TaskNotInProgress);
        require!(
            task.assigned_robot == Some(ctx.accounts.robot.key()),
            ErrorCode::NotAssignedRobot
        );
        require!(progress <= 100, ErrorCode::InvalidProgress);

        task.progress = progress;

        emit!(TaskProgressUpdated {
            task: task.key(),
            progress,
        });

        Ok(())
    }

    /// Complete the task (by robot)
    pub fn complete_task(ctx: Context<ExecuteTask>) -> Result<()> {
        let task = &mut ctx.accounts.task;
        let clock = Clock::get()?;

        require!(task.status == TaskStatus::InProgress, ErrorCode::TaskNotInProgress);
        require!(
            task.assigned_robot == Some(ctx.accounts.robot.key()),
            ErrorCode::NotAssignedRobot
        );

        task.status = TaskStatus::PendingVerification;
        task.progress = 100;

        // TODO: Pause payment stream pending verification

        emit!(TaskPendingVerification {
            task: task.key(),
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Verify task completion (by creator)
    pub fn verify_completion(ctx: Context<VerifyTask>, approved: bool) -> Result<()> {
        let task = &mut ctx.accounts.task;
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;

        require!(task.status == TaskStatus::PendingVerification, ErrorCode::TaskNotPendingVerification);
        require!(task.creator == ctx.accounts.creator.key(), ErrorCode::Unauthorized);

        if approved {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(clock.unix_timestamp);
            
            market.total_completed += 1;
            market.total_volume += task.reward;

            // TODO: Complete payment stream via CPI
            // TODO: Update robot reputation via CPI

            emit!(TaskCompleted {
                task: task.key(),
                robot: task.assigned_robot.unwrap(),
                total_paid: task.reward,
                timestamp: clock.unix_timestamp,
            });
        } else {
            task.status = TaskStatus::Disputed;

            emit!(TaskDisputed {
                task: task.key(),
                timestamp: clock.unix_timestamp,
            });
        }

        Ok(())
    }

    /// Cancel a task (before assignment)
    pub fn cancel_task(ctx: Context<CancelTask>) -> Result<()> {
        let task = &mut ctx.accounts.task;
        let clock = Clock::get()?;

        require!(task.creator == ctx.accounts.creator.key(), ErrorCode::Unauthorized);
        require!(
            task.status == TaskStatus::Open,
            ErrorCode::TaskCannotBeCancelled
        );

        task.status = TaskStatus::Cancelled;

        emit!(TaskCancelled {
            task: task.key(),
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Abort a task in progress (emergency)
    pub fn abort_task(ctx: Context<AbortTask>, reason: String) -> Result<()> {
        let task = &mut ctx.accounts.task;
        let clock = Clock::get()?;

        require!(reason.len() <= 128, ErrorCode::MessageTooLong);
        require!(
            task.creator == ctx.accounts.authority.key() || 
            task.assigned_robot == Some(ctx.accounts.authority.key()),
            ErrorCode::Unauthorized
        );
        require!(
            task.status == TaskStatus::Assigned || 
            task.status == TaskStatus::InProgress,
            ErrorCode::TaskCannotBeAborted
        );

        task.status = TaskStatus::Failed;

        // TODO: Terminate payment stream via CPI
        // TODO: Apply reputation penalty to robot if robot's fault

        emit!(TaskAborted {
            task: task.key(),
            reason,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }
}

// ============================================================================
// ACCOUNTS
// ============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market"],
        bump
    )]
    pub market: Account<'info, Market>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(title: String)]
pub struct CreateTask<'info> {
    #[account(mut, seeds = [b"market"], bump = market.bump)]
    pub market: Account<'info, Market>,
    
    #[account(
        init,
        payer = creator,
        space = 8 + Task::INIT_SPACE,
        seeds = [b"task", creator.key().as_ref(), &market.total_tasks.to_le_bytes()],
        bump
    )]
    pub task: Account<'info, Task>,
    
    #[account(mut)]
    pub creator: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SubmitBid<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    #[account(
        init,
        payer = operator,
        space = 8 + Bid::INIT_SPACE,
        seeds = [b"bid", task.key().as_ref(), robot.key().as_ref()],
        bump
    )]
    pub bid: Account<'info, Bid>,
    
    /// CHECK: Robot account from identity-registry
    pub robot: AccountInfo<'info>,
    
    #[account(mut)]
    pub operator: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AcceptBid<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    #[account(
        mut,
        constraint = bid.task == task.key() @ ErrorCode::BidTaskMismatch
    )]
    pub bid: Account<'info, Bid>,
    
    #[account(constraint = creator.key() == task.creator @ ErrorCode::Unauthorized)]
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct RejectBid<'info> {
    pub task: Account<'info, Task>,
    
    #[account(
        mut,
        constraint = bid.task == task.key() @ ErrorCode::BidTaskMismatch
    )]
    pub bid: Account<'info, Bid>,
    
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct WithdrawBid<'info> {
    #[account(mut)]
    pub bid: Account<'info, Bid>,
    
    pub operator: Signer<'info>,
}

#[derive(Accounts)]
pub struct ExecuteTask<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    /// CHECK: Robot account from identity-registry
    pub robot: AccountInfo<'info>,
    
    pub operator: Signer<'info>,
}

#[derive(Accounts)]
pub struct VerifyTask<'info> {
    #[account(mut, seeds = [b"market"], bump = market.bump)]
    pub market: Account<'info, Market>,
    
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct CancelTask<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct AbortTask<'info> {
    #[account(mut)]
    pub task: Account<'info, Task>,
    
    pub authority: Signer<'info>,
}

// ============================================================================
// STATE
// ============================================================================

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub authority: Pubkey,
    pub total_tasks: u64,
    pub total_completed: u64,
    pub total_volume: u64,
    pub fee_basis_points: u16,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Task {
    pub creator: Pubkey,
    #[max_len(64)]
    pub title: String,
    #[max_len(256)]
    pub description: String,
    pub robot_class: u8,
    #[max_len(5)]
    pub required_capabilities: Vec<u8>,
    pub min_reputation: u16,
    pub reward: u64,
    pub rate_per_second: u64,
    pub estimated_duration: u32,
    pub priority: u8,
    pub status: TaskStatus,
    pub created_at: i64,
    pub expires_at: i64,
    pub assigned_robot: Option<Pubkey>,
    pub assigned_at: Option<i64>,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub stream_id: Option<Pubkey>,
    pub progress: u8,
    pub bids_count: u16,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Bid {
    pub task: Pubkey,
    pub robot: Pubkey,
    pub operator: Pubkey,
    pub proposed_rate: u64,
    pub estimated_duration: u32,
    #[max_len(128)]
    pub message: String,
    pub status: BidStatus,
    pub submitted_at: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum TaskStatus {
    Open,
    Assigned,
    InProgress,
    PendingVerification,
    Completed,
    Failed,
    Cancelled,
    Disputed,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum BidStatus {
    Pending,
    Accepted,
    Rejected,
    Withdrawn,
    Expired,
}

// ============================================================================
// EVENTS
// ============================================================================

#[event]
pub struct TaskCreated {
    pub task: Pubkey,
    pub creator: Pubkey,
    pub title: String,
    pub reward: u64,
    pub expires_at: i64,
}

#[event]
pub struct BidSubmitted {
    pub task: Pubkey,
    pub bid: Pubkey,
    pub robot: Pubkey,
    pub proposed_rate: u64,
    pub estimated_duration: u32,
}

#[event]
pub struct BidRejected {
    pub task: Pubkey,
    pub bid: Pubkey,
}

#[event]
pub struct BidWithdrawn {
    pub bid: Pubkey,
}

#[event]
pub struct TaskAssigned {
    pub task: Pubkey,
    pub robot: Pubkey,
    pub rate: u64,
    pub timestamp: i64,
}

#[event]
pub struct TaskStarted {
    pub task: Pubkey,
    pub robot: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TaskProgressUpdated {
    pub task: Pubkey,
    pub progress: u8,
}

#[event]
pub struct TaskPendingVerification {
    pub task: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TaskCompleted {
    pub task: Pubkey,
    pub robot: Pubkey,
    pub total_paid: u64,
    pub timestamp: i64,
}

#[event]
pub struct TaskDisputed {
    pub task: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TaskCancelled {
    pub task: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct TaskAborted {
    pub task: Pubkey,
    pub reason: String,
    pub timestamp: i64,
}

// ============================================================================
// ERRORS
// ============================================================================

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized,
    
    #[msg("Title too long (max 64 chars)")]
    TitleTooLong,
    
    #[msg("Description too long (max 256 chars)")]
    DescriptionTooLong,
    
    #[msg("Message too long (max 128 chars)")]
    MessageTooLong,
    
    #[msg("Too many required capabilities (max 5)")]
    TooManyCapabilities,
    
    #[msg("Invalid reward amount")]
    InvalidReward,
    
    #[msg("Invalid priority (must be 1-5)")]
    InvalidPriority,
    
    #[msg("Invalid expiration time")]
    InvalidExpiration,
    
    #[msg("Invalid progress value (must be 0-100)")]
    InvalidProgress,
    
    #[msg("Task is not open for bids")]
    TaskNotOpen,
    
    #[msg("Task has expired")]
    TaskExpired,
    
    #[msg("Task is not assigned")]
    TaskNotAssigned,
    
    #[msg("Task is not in progress")]
    TaskNotInProgress,
    
    #[msg("Task is not pending verification")]
    TaskNotPendingVerification,
    
    #[msg("Task cannot be cancelled in current state")]
    TaskCannotBeCancelled,
    
    #[msg("Task cannot be aborted in current state")]
    TaskCannotBeAborted,
    
    #[msg("Bid is not pending")]
    BidNotPending,
    
    #[msg("Bid does not match task")]
    BidTaskMismatch,
    
    #[msg("Not the assigned robot")]
    NotAssignedRobot,
}
