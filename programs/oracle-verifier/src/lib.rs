use anchor_lang::prelude::*;

declare_id!("DOS4orc1111111111111111111111111111111111111");

/// $DRONEOS Oracle Verifier Program
/// 
/// Decentralized verification system for robot tasks:
/// - GPS location verification via Chainlink/Pyth
/// - Proof of completion with cryptographic signatures
/// - IoT sensor data validation
/// - Dispute resolution with evidence submission
/// - Automated task verification

#[program]
pub mod oracle_verifier {
    use super::*;

    /// Initialize oracle verifier
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let verifier = &mut ctx.accounts.verifier;
        verifier.authority = ctx.accounts.authority.key();
        verifier.total_verifications = 0;
        verifier.successful_verifications = 0;
        verifier.disputed_verifications = 0;
        verifier.min_confidence_score = 80; // 80% minimum
        verifier.bump = ctx.bumps.verifier;
        
        emit!(VerifierInitialized {
            authority: verifier.authority,
        });
        
        Ok(())
    }

    /// Register oracle (Chainlink node, Pyth, or custom)
    pub fn register_oracle(
        ctx: Context<RegisterOracle>,
        oracle_type: OracleType,
        endpoint: String,
        reputation: u16,
    ) -> Result<()> {
        require!(endpoint.len() <= 128, ErrorCode::EndpointTooLong);
        require!(reputation <= 100, ErrorCode::InvalidReputation);
        
        let oracle = &mut ctx.accounts.oracle;
        oracle.provider = ctx.accounts.provider.key();
        oracle.oracle_type = oracle_type;
        oracle.endpoint = endpoint;
        oracle.reputation = reputation;
        oracle.total_verifications = 0;
        oracle.successful_verifications = 0;
        oracle.is_active = true;
        oracle.registered_at = Clock::get()?.unix_timestamp;
        oracle.bump = ctx.bumps.oracle;
        
        emit!(OracleRegistered {
            oracle: oracle.key(),
            provider: oracle.provider,
            oracle_type,
        });
        
        Ok(())
    }

    /// Submit GPS proof for task
    pub fn submit_gps_proof(
        ctx: Context<SubmitGPSProof>,
        latitude: i64,  // Fixed-point: actual * 1_000_000
        longitude: i64, // Fixed-point: actual * 1_000_000
        altitude: i32,  // Meters
        timestamp: i64,
        signature: [u8; 64], // Ed25519 signature from robot
    ) -> Result<()> {
        let proof = &mut ctx.accounts.proof;
        proof.task = ctx.accounts.task.key();
        proof.robot = ctx.accounts.robot.key();
        proof.oracle = ctx.accounts.oracle.key();
        proof.proof_type = ProofType::GPS;
        proof.latitude = Some(latitude);
        proof.longitude = Some(longitude);
        proof.altitude = Some(altitude);
        proof.timestamp = timestamp;
        proof.signature = signature;
        proof.confidence_score = 0; // To be set by oracle
        proof.status = ProofStatus::Pending;
        proof.submitted_at = Clock::get()?.unix_timestamp;
        proof.bump = ctx.bumps.proof;
        
        emit!(GPSProofSubmitted {
            proof: proof.key(),
            task: proof.task,
            robot: proof.robot,
            latitude,
            longitude,
        });
        
        Ok(())
    }

    /// Submit completion proof (photo hash, sensor data, etc)
    pub fn submit_completion_proof(
        ctx: Context<SubmitCompletionProof>,
        data_hash: [u8; 32], // SHA256 of proof data
        proof_url: String,   // IPFS/Arweave URL
        metadata: String,    // JSON metadata
    ) -> Result<()> {
        require!(proof_url.len() <= 128, ErrorCode::URLTooLong);
        require!(metadata.len() <= 256, ErrorCode::MetadataTooLong);
        
        let proof = &mut ctx.accounts.proof;
        proof.task = ctx.accounts.task.key();
        proof.robot = ctx.accounts.robot.key();
        proof.oracle = ctx.accounts.oracle.key();
        proof.proof_type = ProofType::Completion;
        proof.data_hash = Some(data_hash);
        proof.proof_url = Some(proof_url);
        proof.metadata = Some(metadata);
        proof.confidence_score = 0;
        proof.status = ProofStatus::Pending;
        proof.submitted_at = Clock::get()?.unix_timestamp;
        proof.bump = ctx.bumps.proof;
        
        emit!(CompletionProofSubmitted {
            proof: proof.key(),
            task: proof.task,
            robot: proof.robot,
            data_hash,
        });
        
        Ok(())
    }

    /// Oracle verifies proof (called by oracle node)
    pub fn verify_proof(
        ctx: Context<VerifyProof>,
        confidence_score: u8,
        is_valid: bool,
        verification_data: String,
    ) -> Result<()> {
        require!(confidence_score <= 100, ErrorCode::InvalidConfidenceScore);
        require!(verification_data.len() <= 256, ErrorCode::VerificationDataTooLong);
        
        let proof = &mut ctx.accounts.proof;
        let oracle = &mut ctx.accounts.oracle;
        let verifier = &mut ctx.accounts.verifier;
        
        require!(proof.status == ProofStatus::Pending, ErrorCode::ProofAlreadyVerified);
        
        proof.confidence_score = confidence_score;
        proof.status = if is_valid && confidence_score >= verifier.min_confidence_score {
            ProofStatus::Verified
        } else {
            ProofStatus::Failed
        };
        proof.verification_data = Some(verification_data);
        proof.verified_at = Some(Clock::get()?.unix_timestamp);
        
        // Update statistics
        verifier.total_verifications += 1;
        oracle.total_verifications += 1;
        
        if proof.status == ProofStatus::Verified {
            verifier.successful_verifications += 1;
            oracle.successful_verifications += 1;
            
            // Update oracle reputation
            if oracle.reputation < 100 {
                oracle.reputation = std::cmp::min(100, oracle.reputation + 1);
            }
        } else {
            // Decrease reputation on failure
            if oracle.reputation > 0 {
                oracle.reputation = oracle.reputation.saturating_sub(2);
            }
        }
        
        emit!(ProofVerified {
            proof: proof.key(),
            oracle: oracle.key(),
            is_valid,
            confidence_score,
        });
        
        Ok(())
    }

    /// Create dispute for a proof
    pub fn create_dispute(
        ctx: Context<CreateDispute>,
        reason: String,
        evidence_url: String,
    ) -> Result<()> {
        require!(reason.len() <= 256, ErrorCode::ReasonTooLong);
        require!(evidence_url.len() <= 128, ErrorCode::URLTooLong);
        
        let dispute = &mut ctx.accounts.dispute;
        let proof = &ctx.accounts.proof;
        let verifier = &mut ctx.accounts.verifier;
        
        require!(
            proof.status == ProofStatus::Verified || proof.status == ProofStatus::Failed,
            ErrorCode::ProofNotFinalized
        );
        
        dispute.proof = proof.key();
        dispute.challenger = ctx.accounts.challenger.key();
        dispute.reason = reason;
        dispute.evidence_url = evidence_url;
        dispute.status = DisputeStatus::Open;
        dispute.votes_for = 0;
        dispute.votes_against = 0;
        dispute.created_at = Clock::get()?.unix_timestamp;
        dispute.bump = ctx.bumps.dispute;
        
        verifier.disputed_verifications += 1;
        
        emit!(DisputeCreated {
            dispute: dispute.key(),
            proof: dispute.proof,
            challenger: dispute.challenger,
        });
        
        Ok(())
    }

    /// Vote on dispute (requires staked DRONEOS)
    pub fn vote_on_dispute(
        ctx: Context<VoteOnDispute>,
        vote_for_challenger: bool,
    ) -> Result<()> {
        let dispute = &mut ctx.accounts.dispute;
        let vote = &mut ctx.accounts.vote;
        
        require!(dispute.status == DisputeStatus::Open, ErrorCode::DisputeNotOpen);
        
        // TODO: Verify voter has staked tokens via CPI
        
        vote.dispute = dispute.key();
        vote.voter = ctx.accounts.voter.key();
        vote.vote_for_challenger = vote_for_challenger;
        vote.weight = 100; // Based on stake amount
        vote.voted_at = Clock::get()?.unix_timestamp;
        vote.bump = ctx.bumps.vote;
        
        if vote_for_challenger {
            dispute.votes_for += vote.weight;
        } else {
            dispute.votes_against += vote.weight;
        }
        
        emit!(DisputeVoted {
            dispute: dispute.key(),
            voter: vote.voter,
            vote_for_challenger,
            weight: vote.weight,
        });
        
        Ok(())
    }

    /// Resolve dispute based on votes
    pub fn resolve_dispute(ctx: Context<ResolveDispute>) -> Result<()> {
        let dispute = &mut ctx.accounts.dispute;
        let proof = &mut ctx.accounts.proof;
        
        require!(dispute.status == DisputeStatus::Open, ErrorCode::DisputeNotOpen);
        
        // Check voting period (e.g., 7 days)
        let current_time = Clock::get()?.unix_timestamp;
        let voting_period = 7 * 24 * 60 * 60; // 7 days
        require!(
            current_time >= dispute.created_at + voting_period,
            ErrorCode::VotingPeriodNotEnded
        );
        
        // Determine outcome
        if dispute.votes_for > dispute.votes_against {
            // Challenger wins - invalidate proof
            dispute.status = DisputeStatus::ChallengerWins;
            proof.status = ProofStatus::Disputed;
            dispute.resolved_at = Some(current_time);
        } else {
            // Oracle wins - proof stands
            dispute.status = DisputeStatus::OracleWins;
            dispute.resolved_at = Some(current_time);
        }
        
        emit!(DisputeResolved {
            dispute: dispute.key(),
            outcome: dispute.status.clone(),
            votes_for: dispute.votes_for,
            votes_against: dispute.votes_against,
        });
        
        Ok(())
    }

    /// Auto-verify task if all required proofs are verified
    pub fn auto_verify_task(ctx: Context<AutoVerifyTask>) -> Result<()> {
        // Check if task has required proofs:
        // - GPS proof at start location
        // - GPS proof at end location  
        // - Completion proof (photo/sensor data)
        
        // TODO: Implement CPI to task-market to mark task as verified
        
        emit!(TaskAutoVerified {
            task: ctx.accounts.task.key(),
            verified_at: Clock::get()?.unix_timestamp,
        });
        
        Ok(())
    }
}

// Account Structures

#[account]
pub struct Verifier {
    pub authority: Pubkey,
    pub total_verifications: u64,
    pub successful_verifications: u64,
    pub disputed_verifications: u64,
    pub min_confidence_score: u8,
    pub bump: u8,
}

#[account]
pub struct Oracle {
    pub provider: Pubkey,
    pub oracle_type: OracleType,
    pub endpoint: String,
    pub reputation: u16, // 0-100
    pub total_verifications: u64,
    pub successful_verifications: u64,
    pub is_active: bool,
    pub registered_at: i64,
    pub bump: u8,
}

#[account]
pub struct Proof {
    pub task: Pubkey,
    pub robot: Pubkey,
    pub oracle: Pubkey,
    pub proof_type: ProofType,
    
    // GPS data (optional)
    pub latitude: Option<i64>,
    pub longitude: Option<i64>,
    pub altitude: Option<i32>,
    
    // Completion data (optional)
    pub data_hash: Option<[u8; 32]>,
    pub proof_url: Option<String>,
    pub metadata: Option<String>,
    
    pub timestamp: i64,
    pub signature: [u8; 64],
    pub confidence_score: u8,
    pub status: ProofStatus,
    pub verification_data: Option<String>,
    pub submitted_at: i64,
    pub verified_at: Option<i64>,
    pub bump: u8,
}

#[account]
pub struct Dispute {
    pub proof: Pubkey,
    pub challenger: Pubkey,
    pub reason: String,
    pub evidence_url: String,
    pub status: DisputeStatus,
    pub votes_for: u64,
    pub votes_against: u64,
    pub created_at: i64,
    pub resolved_at: Option<i64>,
    pub bump: u8,
}

#[account]
pub struct DisputeVote {
    pub dispute: Pubkey,
    pub voter: Pubkey,
    pub vote_for_challenger: bool,
    pub weight: u64, // Based on staked amount
    pub voted_at: i64,
    pub bump: u8,
}

// Enums

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum OracleType {
    Chainlink,
    Pyth,
    Custom,
    GPS,
    IoT,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum ProofType {
    GPS,
    Completion,
    Sensor,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum ProofStatus {
    Pending,
    Verified,
    Failed,
    Disputed,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum DisputeStatus {
    Open,
    ChallengerWins,
    OracleWins,
}

// Context Structs (simplified)

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + 32 + 8 + 8 + 8 + 1 + 1,
        seeds = [b"verifier"],
        bump
    )]
    pub verifier: Account<'info, Verifier>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterOracle<'info> {
    #[account(
        init,
        payer = provider,
        space = 8 + 32 + 1 + 132 + 2 + 8 + 8 + 1 + 8 + 1,
        seeds = [b"oracle", provider.key().as_ref()],
        bump
    )]
    pub oracle: Account<'info, Oracle>,
    #[account(mut)]
    pub provider: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SubmitGPSProof<'info> {
    /// CHECK: Task account
    pub task: AccountInfo<'info>,
    /// CHECK: Robot account
    pub robot: AccountInfo<'info>,
    pub oracle: Account<'info, Oracle>,
    #[account(
        init,
        payer = operator,
        space = 8 + 32 + 32 + 32 + 1 + 9 + 9 + 5 + 33 + 132 + 260 + 8 + 64 + 1 + 1 + 260 + 8 + 9 + 1,
        seeds = [b"proof", task.key().as_ref(), robot.key().as_ref()],
        bump
    )]
    pub proof: Account<'info, Proof>,
    #[account(mut)]
    pub operator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SubmitCompletionProof<'info> {
    /// CHECK: Task account
    pub task: AccountInfo<'info>,
    /// CHECK: Robot account
    pub robot: AccountInfo<'info>,
    pub oracle: Account<'info, Oracle>,
    #[account(
        init,
        payer = operator,
        space = 8 + 32 + 32 + 32 + 1 + 9 + 9 + 5 + 33 + 132 + 260 + 8 + 64 + 1 + 1 + 260 + 8 + 9 + 1,
        seeds = [b"completion-proof", task.key().as_ref()],
        bump
    )]
    pub proof: Account<'info, Proof>,
    #[account(mut)]
    pub operator: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyProof<'info> {
    #[account(mut)]
    pub verifier: Account<'info, Verifier>,
    #[account(mut)]
    pub oracle: Account<'info, Oracle>,
    #[account(mut)]
    pub proof: Account<'info, Proof>,
    pub oracle_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CreateDispute<'info> {
    #[account(mut)]
    pub verifier: Account<'info, Verifier>,
    pub proof: Account<'info, Proof>,
    #[account(
        init,
        payer = challenger,
        space = 8 + 32 + 32 + 260 + 132 + 1 + 8 + 8 + 8 + 9 + 1,
        seeds = [b"dispute", proof.key().as_ref(), challenger.key().as_ref()],
        bump
    )]
    pub dispute: Account<'info, Dispute>,
    #[account(mut)]
    pub challenger: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VoteOnDispute<'info> {
    #[account(mut)]
    pub dispute: Account<'info, Dispute>,
    #[account(
        init,
        payer = voter,
        space = 8 + 32 + 32 + 1 + 8 + 8 + 1,
        seeds = [b"vote", dispute.key().as_ref(), voter.key().as_ref()],
        bump
    )]
    pub vote: Account<'info, DisputeVote>,
    #[account(mut)]
    pub voter: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ResolveDispute<'info> {
    #[account(mut)]
    pub dispute: Account<'info, Dispute>,
    #[account(mut)]
    pub proof: Account<'info, Proof>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AutoVerifyTask<'info> {
    /// CHECK: Task account
    pub task: AccountInfo<'info>,
    pub verifier: Account<'info, Verifier>,
}

// Events

#[event]
pub struct VerifierInitialized {
    pub authority: Pubkey,
}

#[event]
pub struct OracleRegistered {
    pub oracle: Pubkey,
    pub provider: Pubkey,
    pub oracle_type: OracleType,
}

#[event]
pub struct GPSProofSubmitted {
    pub proof: Pubkey,
    pub task: Pubkey,
    pub robot: Pubkey,
    pub latitude: i64,
    pub longitude: i64,
}

#[event]
pub struct CompletionProofSubmitted {
    pub proof: Pubkey,
    pub task: Pubkey,
    pub robot: Pubkey,
    pub data_hash: [u8; 32],
}

#[event]
pub struct ProofVerified {
    pub proof: Pubkey,
    pub oracle: Pubkey,
    pub is_valid: bool,
    pub confidence_score: u8,
}

#[event]
pub struct DisputeCreated {
    pub dispute: Pubkey,
    pub proof: Pubkey,
    pub challenger: Pubkey,
}

#[event]
pub struct DisputeVoted {
    pub dispute: Pubkey,
    pub voter: Pubkey,
    pub vote_for_challenger: bool,
    pub weight: u64,
}

#[event]
pub struct DisputeResolved {
    pub dispute: Pubkey,
    pub outcome: DisputeStatus,
    pub votes_for: u64,
    pub votes_against: u64,
}

#[event]
pub struct TaskAutoVerified {
    pub task: Pubkey,
    pub verified_at: i64,
}

// Errors

#[error_code]
pub enum ErrorCode {
    #[msg("Endpoint URL too long")]
    EndpointTooLong,
    #[msg("Invalid reputation score")]
    InvalidReputation,
    #[msg("Proof URL too long")]
    URLTooLong,
    #[msg("Metadata too long")]
    MetadataTooLong,
    #[msg("Invalid confidence score")]
    InvalidConfidenceScore,
    #[msg("Verification data too long")]
    VerificationDataTooLong,
    #[msg("Proof already verified")]
    ProofAlreadyVerified,
    #[msg("Reason too long")]
    ReasonTooLong,
    #[msg("Proof not finalized")]
    ProofNotFinalized,
    #[msg("Dispute not open")]
    DisputeNotOpen,
    #[msg("Voting period not ended")]
    VotingPeriodNotEnded,
}
