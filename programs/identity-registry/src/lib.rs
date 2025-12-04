use anchor_lang::prelude::*;

declare_id!("DOS4id11111111111111111111111111111111111111");

/// $DRONEOS Identity Registry Program
/// 
/// Manages robot identities using 403 proofs:
/// - Robot registration with device attestation
/// - Capability certification and verification
/// - Reputation tracking
/// - Status management

#[program]
pub mod identity_registry {
    use super::*;

    /// Initialize the registry
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let registry = &mut ctx.accounts.registry;
        registry.authority = ctx.accounts.authority.key();
        registry.total_robots = 0;
        registry.total_operators = 0;
        registry.bump = ctx.bumps.registry;
        
        emit!(RegistryInitialized {
            authority: registry.authority,
        });
        
        Ok(())
    }

    /// Register a new robot
    pub fn register_robot(
        ctx: Context<RegisterRobot>,
        device_id: [u8; 32],
        manufacturer_id: String,
        model_id: String,
        firmware_hash: [u8; 32],
        robot_class: RobotClass,
    ) -> Result<()> {
        require!(manufacturer_id.len() <= 32, ErrorCode::StringTooLong);
        require!(model_id.len() <= 32, ErrorCode::StringTooLong);

        let robot = &mut ctx.accounts.robot;
        let registry = &mut ctx.accounts.registry;
        let clock = Clock::get()?;

        robot.device_id = device_id;
        robot.manufacturer_id = manufacturer_id;
        robot.model_id = model_id;
        robot.firmware_hash = firmware_hash;
        robot.robot_class = robot_class;
        robot.operator = ctx.accounts.operator.key();
        robot.registered_at = clock.unix_timestamp;
        robot.last_active_at = clock.unix_timestamp;
        robot.reputation_score = 5000; // Start at 50%
        robot.total_tasks_completed = 0;
        robot.total_earnings = 0;
        robot.status = RobotStatus::Idle;
        robot.capabilities = Vec::new();
        robot.bump = ctx.bumps.robot;

        registry.total_robots += 1;

        emit!(RobotRegistered {
            robot: robot.key(),
            device_id,
            operator: robot.operator,
            robot_class,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Add capability to robot
    pub fn add_capability(
        ctx: Context<UpdateRobot>,
        capability: Capability,
        certification_level: u8,
        valid_days: u32,
    ) -> Result<()> {
        require!(certification_level >= 1 && certification_level <= 5, ErrorCode::InvalidCertificationLevel);
        
        let robot = &mut ctx.accounts.robot;
        let clock = Clock::get()?;
        
        // Check if capability already exists
        let existing = robot.capabilities.iter_mut().find(|c| c.capability == capability);
        
        let valid_until = clock.unix_timestamp + (valid_days as i64 * 86400);
        
        if let Some(cap) = existing {
            cap.certification_level = certification_level;
            cap.valid_until = valid_until;
            cap.issuer = ctx.accounts.authority.key();
        } else {
            require!(robot.capabilities.len() < 10, ErrorCode::TooManyCapabilities);
            robot.capabilities.push(CapabilityProof {
                capability,
                certification_level,
                valid_until,
                issuer: ctx.accounts.authority.key(),
            });
        }

        emit!(CapabilityAdded {
            robot: robot.key(),
            capability,
            level: certification_level,
            valid_until,
        });

        Ok(())
    }

    /// Update robot status
    pub fn update_status(
        ctx: Context<UpdateRobotByOperator>,
        new_status: RobotStatus,
    ) -> Result<()> {
        let robot = &mut ctx.accounts.robot;
        let clock = Clock::get()?;
        
        // Validate status transition
        require!(
            is_valid_status_transition(robot.status, new_status),
            ErrorCode::InvalidStatusTransition
        );
        
        let old_status = robot.status;
        robot.status = new_status;
        robot.last_active_at = clock.unix_timestamp;

        emit!(RobotStatusChanged {
            robot: robot.key(),
            old_status,
            new_status,
            timestamp: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Update reputation after task completion
    pub fn update_reputation(
        ctx: Context<UpdateRobotByProgram>,
        delta: i32,
        task_completed: bool,
        earnings: u64,
    ) -> Result<()> {
        let robot = &mut ctx.accounts.robot;
        let clock = Clock::get()?;
        
        // Apply reputation change (clamped to 0-10000)
        let new_rep = (robot.reputation_score as i32 + delta).max(0).min(10000);
        robot.reputation_score = new_rep as u16;
        
        if task_completed {
            robot.total_tasks_completed += 1;
            robot.total_earnings += earnings;
        }
        
        robot.last_active_at = clock.unix_timestamp;

        emit!(ReputationUpdated {
            robot: robot.key(),
            old_score: robot.reputation_score as i32 - delta,
            new_score: robot.reputation_score,
            delta,
        });

        Ok(())
    }

    /// Verify robot identity (returns capability proof)
    pub fn verify_robot(
        ctx: Context<VerifyRobot>,
        required_capability: Capability,
    ) -> Result<()> {
        let robot = &ctx.accounts.robot;
        let clock = Clock::get()?;
        
        // Check robot is active
        require!(
            robot.status == RobotStatus::Available || robot.status == RobotStatus::Busy,
            ErrorCode::RobotNotActive
        );
        
        // Find and verify capability
        let cap = robot.capabilities.iter()
            .find(|c| c.capability == required_capability)
            .ok_or(ErrorCode::CapabilityNotFound)?;
        
        require!(cap.valid_until > clock.unix_timestamp, ErrorCode::CapabilityExpired);

        emit!(RobotVerified {
            robot: robot.key(),
            capability: required_capability,
            verified_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Deactivate robot (by operator)
    pub fn deactivate_robot(ctx: Context<UpdateRobotByOperator>) -> Result<()> {
        let robot = &mut ctx.accounts.robot;
        
        require!(
            robot.status != RobotStatus::Busy,
            ErrorCode::RobotBusy
        );
        
        robot.status = RobotStatus::Offline;

        emit!(RobotDeactivated {
            robot: robot.key(),
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
        space = 8 + Registry::INIT_SPACE,
        seeds = [b"registry"],
        bump
    )]
    pub registry: Account<'info, Registry>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(device_id: [u8; 32])]
pub struct RegisterRobot<'info> {
    #[account(
        mut,
        seeds = [b"registry"],
        bump = registry.bump
    )]
    pub registry: Account<'info, Registry>,
    
    #[account(
        init,
        payer = operator,
        space = 8 + Robot::INIT_SPACE,
        seeds = [b"robot", device_id.as_ref()],
        bump
    )]
    pub robot: Account<'info, Robot>,
    
    #[account(mut)]
    pub operator: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRobot<'info> {
    #[account(mut)]
    pub robot: Account<'info, Robot>,
    
    #[account(
        constraint = authority.key() == robot.operator @ ErrorCode::Unauthorized
    )]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateRobotByOperator<'info> {
    #[account(
        mut,
        constraint = robot.operator == operator.key() @ ErrorCode::Unauthorized
    )]
    pub robot: Account<'info, Robot>,
    
    pub operator: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateRobotByProgram<'info> {
    #[account(mut)]
    pub robot: Account<'info, Robot>,
    
    /// CHECK: Verified by caller program via CPI
    pub caller_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct VerifyRobot<'info> {
    pub robot: Account<'info, Robot>,
}

// ============================================================================
// STATE
// ============================================================================

#[account]
#[derive(InitSpace)]
pub struct Registry {
    pub authority: Pubkey,
    pub total_robots: u64,
    pub total_operators: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Robot {
    pub device_id: [u8; 32],
    #[max_len(32)]
    pub manufacturer_id: String,
    #[max_len(32)]
    pub model_id: String,
    pub firmware_hash: [u8; 32],
    pub robot_class: RobotClass,
    pub operator: Pubkey,
    pub registered_at: i64,
    pub last_active_at: i64,
    pub reputation_score: u16,      // 0-10000 (0-100.00%)
    pub total_tasks_completed: u32,
    pub total_earnings: u64,
    pub status: RobotStatus,
    #[max_len(10)]
    pub capabilities: Vec<CapabilityProof>,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, InitSpace)]
pub struct CapabilityProof {
    pub capability: Capability,
    pub certification_level: u8,  // 1-5
    pub valid_until: i64,
    pub issuer: Pubkey,
}

// ============================================================================
// ENUMS
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum RobotClass {
    Drone,
    Ground,
    Marine,
    Industrial,
    Humanoid,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum RobotStatus {
    Idle,
    Available,
    Busy,
    Maintenance,
    Offline,
    Suspended,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum Capability {
    Delivery,
    Surveillance,
    Inspection,
    Transport,
    Manipulation,
    Cleaning,
    Security,
    Agriculture,
    Construction,
    Warehouse,
}

// ============================================================================
// HELPERS
// ============================================================================

fn is_valid_status_transition(from: RobotStatus, to: RobotStatus) -> bool {
    match from {
        RobotStatus::Idle => matches!(to, RobotStatus::Available | RobotStatus::Maintenance | RobotStatus::Offline),
        RobotStatus::Available => matches!(to, RobotStatus::Busy | RobotStatus::Idle | RobotStatus::Maintenance | RobotStatus::Offline),
        RobotStatus::Busy => matches!(to, RobotStatus::Available | RobotStatus::Idle | RobotStatus::Maintenance),
        RobotStatus::Maintenance => matches!(to, RobotStatus::Idle | RobotStatus::Available | RobotStatus::Offline),
        RobotStatus::Offline => matches!(to, RobotStatus::Idle | RobotStatus::Available | RobotStatus::Maintenance),
        RobotStatus::Suspended => matches!(to, RobotStatus::Idle),
    }
}

// ============================================================================
// EVENTS
// ============================================================================

#[event]
pub struct RegistryInitialized {
    pub authority: Pubkey,
}

#[event]
pub struct RobotRegistered {
    pub robot: Pubkey,
    pub device_id: [u8; 32],
    pub operator: Pubkey,
    pub robot_class: RobotClass,
    pub timestamp: i64,
}

#[event]
pub struct CapabilityAdded {
    pub robot: Pubkey,
    pub capability: Capability,
    pub level: u8,
    pub valid_until: i64,
}

#[event]
pub struct RobotStatusChanged {
    pub robot: Pubkey,
    pub old_status: RobotStatus,
    pub new_status: RobotStatus,
    pub timestamp: i64,
}

#[event]
pub struct ReputationUpdated {
    pub robot: Pubkey,
    pub old_score: i32,
    pub new_score: u16,
    pub delta: i32,
}

#[event]
pub struct RobotVerified {
    pub robot: Pubkey,
    pub capability: Capability,
    pub verified_at: i64,
}

#[event]
pub struct RobotDeactivated {
    pub robot: Pubkey,
}

// ============================================================================
// ERRORS
// ============================================================================

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized access")]
    Unauthorized,
    
    #[msg("String exceeds maximum length")]
    StringTooLong,
    
    #[msg("Invalid certification level (must be 1-5)")]
    InvalidCertificationLevel,
    
    #[msg("Too many capabilities (max 10)")]
    TooManyCapabilities,
    
    #[msg("Invalid status transition")]
    InvalidStatusTransition,
    
    #[msg("Robot is not active")]
    RobotNotActive,
    
    #[msg("Robot is busy and cannot be modified")]
    RobotBusy,
    
    #[msg("Capability not found")]
    CapabilityNotFound,
    
    #[msg("Capability has expired")]
    CapabilityExpired,
}
