use anchor_lang::prelude::*;
use anchor_spl::token::{Token, TokenAccount};

declare_id!("DOS4swm1111111111111111111111111111111111111");

/// $DRONEOS Swarm Coordinator Program
/// 
/// Multi-robot task coordination:
/// - Group task creation (multiple robots required)
/// - Collective bidding with reward splitting
/// - Swarm synchronization and coordination
/// - Performance-based reward distribution

#[program]
pub mod swarm_coordinator {
    use super::*;

    /// Initialize swarm coordinator
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let coordinator = &mut ctx.accounts.coordinator;
        coordinator.authority = ctx.accounts.authority.key();
        coordinator.total_swarms = 0;
        coordinator.total_group_tasks = 0;
        coordinator.bump = ctx.bumps.coordinator;
        
        emit!(CoordinatorInitialized {
            authority: coordinator.authority,
        });
        
        Ok(())
    }

    /// Create a swarm (group of robots)
    pub fn create_swarm(
        ctx: Context<CreateSwarm>,
        name: String,
        max_robots: u8,
        min_reputation: u16,
    ) -> Result<()> {
        require!(max_robots >= 2 && max_robots <= 20, ErrorCode::InvalidSwarmSize);
        require!(name.len() <= 32, ErrorCode::NameTooLong);
        
        let swarm = &mut ctx.accounts.swarm;
        swarm.leader = ctx.accounts.leader.key();
        swarm.name = name;
        swarm.max_robots = max_robots;
        swarm.current_robots = 0;
        swarm.min_reputation = min_reputation;
        swarm.status = SwarmStatus::Recruiting;
        swarm.total_tasks_completed = 0;
        swarm.total_earned = 0;
        swarm.created_at = Clock::get()?.unix_timestamp;
        swarm.bump = ctx.bumps.swarm;
        
        let coordinator = &mut ctx.accounts.coordinator;
        coordinator.total_swarms += 1;
        
        emit!(SwarmCreated {
            swarm: swarm.key(),
            leader: swarm.leader,
            max_robots,
        });
        
        Ok(())
    }

    /// Join a swarm
    pub fn join_swarm(ctx: Context<JoinSwarm>) -> Result<()> {
        let swarm = &mut ctx.accounts.swarm;
        
        require!(swarm.status == SwarmStatus::Recruiting, ErrorCode::SwarmNotRecruiting);
        require!(swarm.current_robots < swarm.max_robots, ErrorCode::SwarmFull);
        
        // Check robot reputation
        // TODO: Verify via identity registry CPI
        
        let membership = &mut ctx.accounts.membership;
        membership.swarm = swarm.key();
        membership.robot = ctx.accounts.robot.key();
        membership.operator = ctx.accounts.operator.key();
        membership.joined_at = Clock::get()?.unix_timestamp;
        membership.tasks_completed = 0;
        membership.contribution_score = 100; // Base score
        membership.bump = ctx.bumps.membership;
        
        swarm.current_robots += 1;
        
        // Auto-activate if full
        if swarm.current_robots == swarm.max_robots {
            swarm.status = SwarmStatus::Active;
        }
        
        emit!(RobotJoinedSwarm {
            swarm: swarm.key(),
            robot: membership.robot,
            operator: membership.operator,
        });
        
        Ok(())
    }

    /// Create group task (requires multiple robots)
    pub fn create_group_task(
        ctx: Context<CreateGroupTask>,
        title: String,
        description: String,
        required_robots: u8,
        total_reward: u64,
        duration_seconds: i64,
    ) -> Result<()> {
        require!(required_robots >= 2 && required_robots <= 20, ErrorCode::InvalidRobotCount);
        require!(title.len() <= 64, ErrorCode::TitleTooLong);
        require!(description.len() <= 256, ErrorCode::DescriptionTooLong);
        require!(total_reward > 0, ErrorCode::InvalidReward);
        
        let task = &mut ctx.accounts.group_task;
        task.creator = ctx.accounts.creator.key();
        task.title = title;
        task.description = description;
        task.required_robots = required_robots;
        task.current_robots = 0;
        task.total_reward = total_reward;
        task.reward_per_robot = total_reward / required_robots as u64;
        task.duration_seconds = duration_seconds;
        task.status = GroupTaskStatus::Open;
        task.created_at = Clock::get()?.unix_timestamp;
        task.bump = ctx.bumps.group_task;
        
        let coordinator = &mut ctx.accounts.coordinator;
        coordinator.total_group_tasks += 1;
        
        emit!(GroupTaskCreated {
            task: task.key(),
            creator: task.creator,
            required_robots,
            total_reward,
        });
        
        Ok(())
    }

    /// Swarm bids on group task (collective bid)
    pub fn swarm_bid(
        ctx: Context<SwarmBid>,
        proposed_rate: u64,
        estimated_duration: i64,
    ) -> Result<()> {
        let swarm = &ctx.accounts.swarm;
        let task = &ctx.accounts.group_task;
        
        require!(swarm.status == SwarmStatus::Active, ErrorCode::SwarmNotActive);
        require!(task.status == GroupTaskStatus::Open, ErrorCode::TaskNotOpen);
        require!(swarm.current_robots >= task.required_robots, ErrorCode::InsufficientRobots);
        
        let bid = &mut ctx.accounts.bid;
        bid.task = task.key();
        bid.swarm = swarm.key();
        bid.proposed_rate = proposed_rate;
        bid.estimated_duration = estimated_duration;
        bid.total_cost = proposed_rate * estimated_duration as u64;
        bid.status = BidStatus::Pending;
        bid.submitted_at = Clock::get()?.unix_timestamp;
        bid.bump = ctx.bumps.bid;
        
        emit!(SwarmBidSubmitted {
            bid: bid.key(),
            swarm: swarm.key(),
            task: task.key(),
            total_cost: bid.total_cost,
        });
        
        Ok(())
    }

    /// Accept swarm bid and assign task
    pub fn accept_swarm_bid(ctx: Context<AcceptSwarmBid>) -> Result<()> {
        let task = &mut ctx.accounts.group_task;
        let bid = &mut ctx.accounts.bid;
        let swarm = &ctx.accounts.swarm;
        
        require!(task.status == GroupTaskStatus::Open, ErrorCode::TaskNotOpen);
        require!(bid.status == BidStatus::Pending, ErrorCode::BidNotPending);
        
        bid.status = BidStatus::Accepted;
        task.status = GroupTaskStatus::InProgress;
        task.assigned_swarm = Some(swarm.key());
        task.started_at = Some(Clock::get()?.unix_timestamp);
        
        // TODO: Initialize payment streams for all swarm members via CPI
        
        emit!(SwarmBidAccepted {
            task: task.key(),
            swarm: swarm.key(),
            bid: bid.key(),
        });
        
        Ok(())
    }

    /// Complete group task
    pub fn complete_group_task(ctx: Context<CompleteGroupTask>) -> Result<()> {
        let task = &mut ctx.accounts.group_task;
        let swarm = &mut ctx.accounts.swarm;
        
        require!(task.status == GroupTaskStatus::InProgress, ErrorCode::TaskNotInProgress);
        
        task.status = GroupTaskStatus::Completed;
        task.completed_at = Some(Clock::get()?.unix_timestamp);
        
        swarm.total_tasks_completed += 1;
        swarm.total_earned += task.total_reward;
        
        emit!(GroupTaskCompleted {
            task: task.key(),
            swarm: swarm.key(),
            total_reward: task.total_reward,
        });
        
        Ok(())
    }

    /// Distribute rewards to swarm members based on contribution
    pub fn distribute_rewards(ctx: Context<DistributeRewards>) -> Result<()> {
        let task = &ctx.accounts.group_task;
        let membership = &mut ctx.accounts.membership;
        
        require!(task.status == GroupTaskStatus::Completed, ErrorCode::TaskNotCompleted);
        
        // Calculate reward based on contribution score
        let base_reward = task.reward_per_robot;
        let contribution_multiplier = membership.contribution_score as u64;
        let final_reward = (base_reward * contribution_multiplier) / 100;
        
        // TODO: Transfer tokens via CPI
        
        membership.tasks_completed += 1;
        
        emit!(RewardDistributed {
            task: task.key(),
            robot: membership.robot,
            amount: final_reward,
        });
        
        Ok(())
    }
}

// Account Structures

#[account]
pub struct Coordinator {
    pub authority: Pubkey,
    pub total_swarms: u64,
    pub total_group_tasks: u64,
    pub bump: u8,
}

#[account]
pub struct Swarm {
    pub leader: Pubkey,
    pub name: String,
    pub max_robots: u8,
    pub current_robots: u8,
    pub min_reputation: u16,
    pub status: SwarmStatus,
    pub total_tasks_completed: u64,
    pub total_earned: u64,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
pub struct SwarmMembership {
    pub swarm: Pubkey,
    pub robot: Pubkey,
    pub operator: Pubkey,
    pub joined_at: i64,
    pub tasks_completed: u32,
    pub contribution_score: u16, // 0-200, base 100
    pub bump: u8,
}

#[account]
pub struct GroupTask {
    pub creator: Pubkey,
    pub title: String,
    pub description: String,
    pub required_robots: u8,
    pub current_robots: u8,
    pub total_reward: u64,
    pub reward_per_robot: u64,
    pub duration_seconds: i64,
    pub status: GroupTaskStatus,
    pub assigned_swarm: Option<Pubkey>,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub bump: u8,
}

#[account]
pub struct SwarmBid {
    pub task: Pubkey,
    pub swarm: Pubkey,
    pub proposed_rate: u64,
    pub estimated_duration: i64,
    pub total_cost: u64,
    pub status: BidStatus,
    pub submitted_at: i64,
    pub bump: u8,
}

// Enums

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum SwarmStatus {
    Recruiting,
    Active,
    Disbanded,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum GroupTaskStatus {
    Open,
    InProgress,
    Completed,
    Cancelled,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum BidStatus {
    Pending,
    Accepted,
    Rejected,
}

// Context Structs (simplified)

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 1,
        seeds = [b"coordinator"],
        bump
    )]
    pub coordinator: Account<'info, Coordinator>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateSwarm<'info> {
    #[account(mut)]
    pub coordinator: Account<'info, Coordinator>,
    #[account(
        init,
        payer = leader,
        space = 8 + 32 + 36 + 1 + 1 + 2 + 1 + 8 + 8 + 8 + 1,
        seeds = [b"swarm", leader.key().as_ref()],
        bump
    )]
    pub swarm: Account<'info, Swarm>,
    #[account(mut)]
    pub leader: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinSwarm<'info> {
    #[account(mut)]
    pub swarm: Account<'info, Swarm>,
    #[account(
        init,
        payer = operator,
        space = 8 + 32 + 32 + 32 + 8 + 4 + 2 + 1,
        seeds = [b"membership", swarm.key().as_ref(), robot.key().as_ref()],
        bump
    )]
    pub membership: Account<'info, SwarmMembership>,
    /// CHECK: Robot account from identity registry
    pub robot: AccountInfo<'info>,
    #[account(mut)]
    pub operator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateGroupTask<'info> {
    #[account(mut)]
    pub coordinator: Account<'info, Coordinator>,
    #[account(
        init,
        payer = creator,
        space = 8 + 32 + 68 + 260 + 1 + 1 + 8 + 8 + 8 + 1 + 33 + 8 + 9 + 9 + 1,
        seeds = [b"group-task", creator.key().as_ref(), &coordinator.total_group_tasks.to_le_bytes()],
        bump
    )]
    pub group_task: Account<'info, GroupTask>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SwarmBid<'info> {
    pub swarm: Account<'info, Swarm>,
    pub group_task: Account<'info, GroupTask>,
    #[account(
        init,
        payer = leader,
        space = 8 + 32 + 32 + 8 + 8 + 8 + 1 + 8 + 1,
        seeds = [b"swarm-bid", group_task.key().as_ref(), swarm.key().as_ref()],
        bump
    )]
    pub bid: Account<'info, SwarmBid>,
    #[account(mut)]
    pub leader: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AcceptSwarmBid<'info> {
    #[account(mut)]
    pub group_task: Account<'info, GroupTask>,
    #[account(mut)]
    pub bid: Account<'info, SwarmBid>,
    pub swarm: Account<'info, Swarm>,
    pub creator: Signer<'info>,
}

#[derive(Accounts)]
pub struct CompleteGroupTask<'info> {
    #[account(mut)]
    pub group_task: Account<'info, GroupTask>,
    #[account(mut)]
    pub swarm: Account<'info, Swarm>,
    pub leader: Signer<'info>,
}

#[derive(Accounts)]
pub struct DistributeRewards<'info> {
    pub group_task: Account<'info, GroupTask>,
    #[account(mut)]
    pub membership: Account<'info, SwarmMembership>,
    pub operator: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// Events

#[event]
pub struct CoordinatorInitialized {
    pub authority: Pubkey,
}

#[event]
pub struct SwarmCreated {
    pub swarm: Pubkey,
    pub leader: Pubkey,
    pub max_robots: u8,
}

#[event]
pub struct RobotJoinedSwarm {
    pub swarm: Pubkey,
    pub robot: Pubkey,
    pub operator: Pubkey,
}

#[event]
pub struct GroupTaskCreated {
    pub task: Pubkey,
    pub creator: Pubkey,
    pub required_robots: u8,
    pub total_reward: u64,
}

#[event]
pub struct SwarmBidSubmitted {
    pub bid: Pubkey,
    pub swarm: Pubkey,
    pub task: Pubkey,
    pub total_cost: u64,
}

#[event]
pub struct SwarmBidAccepted {
    pub task: Pubkey,
    pub swarm: Pubkey,
    pub bid: Pubkey,
}

#[event]
pub struct GroupTaskCompleted {
    pub task: Pubkey,
    pub swarm: Pubkey,
    pub total_reward: u64,
}

#[event]
pub struct RewardDistributed {
    pub task: Pubkey,
    pub robot: Pubkey,
    pub amount: u64,
}

// Errors

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid swarm size (must be 2-20 robots)")]
    InvalidSwarmSize,
    #[msg("Name too long (max 32 characters)")]
    NameTooLong,
    #[msg("Swarm is not recruiting")]
    SwarmNotRecruiting,
    #[msg("Swarm is full")]
    SwarmFull,
    #[msg("Swarm is not active")]
    SwarmNotActive,
    #[msg("Invalid robot count")]
    InvalidRobotCount,
    #[msg("Title too long")]
    TitleTooLong,
    #[msg("Description too long")]
    DescriptionTooLong,
    #[msg("Invalid reward amount")]
    InvalidReward,
    #[msg("Task is not open")]
    TaskNotOpen,
    #[msg("Insufficient robots in swarm")]
    InsufficientRobots,
    #[msg("Bid is not pending")]
    BidNotPending,
    #[msg("Task is not in progress")]
    TaskNotInProgress,
    #[msg("Task is not completed")]
    TaskNotCompleted,
}
