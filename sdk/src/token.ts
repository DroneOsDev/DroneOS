import { Connection, PublicKey, Keypair, Transaction, SystemProgram } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PROGRAM_IDS } from './index';
import {
  StakeAccount,
  OperatorStakeAccount,
  StakeParams,
  TransactionResult,
  PDAResult,
} from './types';

// Constants
const DECIMALS = 6;
const MIN_STAKE = 100 * 1_000_000; // 100 DRON
const BASE_APY_BPS = 1200; // 12%
const SECONDS_PER_YEAR = 365 * 24 * 60 * 60;

/**
 * Token Client
 * 
 * $DR0N token operations, staking, and rewards
 */
export class TokenClient {
  private connection: Connection;
  private wallet: Keypair | null;
  private programId: PublicKey;

  constructor(connection: Connection, wallet?: Keypair) {
    this.connection = connection;
    this.wallet = wallet || null;
    this.programId = PROGRAM_IDS.DRON_TOKEN;
  }

  setWallet(wallet: Keypair): void {
    this.wallet = wallet;
  }

  // ============================================================================
  // PDA DERIVATIONS
  // ============================================================================

  getConfigPDA(): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('config')],
      this.programId
    );
    return { publicKey, bump };
  }

  getMintPDA(): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('mint')],
      this.programId
    );
    return { publicKey, bump };
  }

  getStakePDA(user: PublicKey): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('stake'), user.toBuffer()],
      this.programId
    );
    return { publicKey, bump };
  }

  getOperatorStakePDA(operator: PublicKey): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('operator'), operator.toBuffer()],
      this.programId
    );
    return { publicKey, bump };
  }

  // ============================================================================
  // STAKING OPERATIONS
  // ============================================================================

  /**
   * Stake tokens
   */
  async stake(
    params: StakeParams,
    stakeVault: PublicKey,
    userTokenAccount: PublicKey,
    user: Keypair
  ): Promise<TransactionResult> {
    if (params.amount < BigInt(MIN_STAKE)) {
      return { 
        signature: '', 
        success: false, 
        error: `Minimum stake is ${MIN_STAKE / 1_000_000} DRON` 
      };
    }

    const configPDA = this.getConfigPDA();
    const stakePDA = this.getStakePDA(user.publicKey);

    const data = Buffer.alloc(8 + 8 + 2);
    data.writeBigUInt64LE(BigInt('0x1111111111111111'), 0);
    data.writeBigUInt64LE(params.amount, 8);
    data.writeUInt16LE(params.lockDays, 16);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: configPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: stakePDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: stakeVault, isSigner: false, isWritable: true },
        { pubkey: userTokenAccount, isSigner: false, isWritable: true },
        { pubkey: user.publicKey, isSigner: true, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [user]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Claim staking rewards
   */
  async claimRewards(
    rewardsVault: PublicKey,
    userTokenAccount: PublicKey,
    user: Keypair
  ): Promise<TransactionResult> {
    const configPDA = this.getConfigPDA();
    const stakePDA = this.getStakePDA(user.publicKey);

    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0x2222222222222222'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: configPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: stakePDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: rewardsVault, isSigner: false, isWritable: true },
        { pubkey: userTokenAccount, isSigner: false, isWritable: true },
        { pubkey: user.publicKey, isSigner: true, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [user]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Unstake tokens
   */
  async unstake(
    amount: bigint | null,
    stakeVault: PublicKey,
    rewardsVault: PublicKey,
    userTokenAccount: PublicKey,
    user: Keypair
  ): Promise<TransactionResult> {
    const configPDA = this.getConfigPDA();
    const stakePDA = this.getStakePDA(user.publicKey);

    const data = Buffer.alloc(8 + 1 + (amount ? 8 : 0));
    data.writeBigUInt64LE(BigInt('0x3333333333333333'), 0);
    
    if (amount) {
      data.writeUInt8(1, 8); // Some
      data.writeBigUInt64LE(amount, 9);
    } else {
      data.writeUInt8(0, 8); // None
    }

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: configPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: stakePDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: stakeVault, isSigner: false, isWritable: true },
        { pubkey: rewardsVault, isSigner: false, isWritable: true },
        { pubkey: userTokenAccount, isSigner: false, isWritable: true },
        { pubkey: user.publicKey, isSigner: true, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [user]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Create operator stake
   */
  async createOperatorStake(
    amount: bigint,
    operatorVault: PublicKey,
    operatorTokenAccount: PublicKey,
    operator: Keypair
  ): Promise<TransactionResult> {
    const configPDA = this.getConfigPDA();
    const operatorStakePDA = this.getOperatorStakePDA(operator.publicKey);

    const data = Buffer.alloc(8 + 8);
    data.writeBigUInt64LE(BigInt('0x4444444444444444'), 0);
    data.writeBigUInt64LE(amount, 8);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: configPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: operatorStakePDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: operatorVault, isSigner: false, isWritable: true },
        { pubkey: operatorTokenAccount, isSigner: false, isWritable: true },
        { pubkey: operator.publicKey, isSigner: true, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [operator]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  // ============================================================================
  // QUERIES
  // ============================================================================

  /**
   * Get stake account
   */
  async getStake(user: PublicKey): Promise<StakeAccount | null> {
    const stakePDA = this.getStakePDA(user);
    const accountInfo = await this.connection.getAccountInfo(stakePDA.publicKey);
    if (!accountInfo) return null;
    return this.decodeStakeAccount(accountInfo.data);
  }

  /**
   * Get operator stake account
   */
  async getOperatorStake(operator: PublicKey): Promise<OperatorStakeAccount | null> {
    const stakePDA = this.getOperatorStakePDA(operator);
    const accountInfo = await this.connection.getAccountInfo(stakePDA.publicKey);
    if (!accountInfo) return null;
    return this.decodeOperatorStakeAccount(accountInfo.data);
  }

  /**
   * Calculate pending rewards
   */
  async getPendingRewards(user: PublicKey): Promise<bigint> {
    const stake = await this.getStake(user);
    if (!stake) return BigInt(0);

    const now = Math.floor(Date.now() / 1000);
    return this.calculateRewards(stake, now);
  }

  /**
   * Calculate rewards for a stake position
   */
  calculateRewards(stake: StakeAccount, currentTime: number): bigint {
    const elapsed = BigInt(currentTime - stake.lastClaimAt);
    
    // Base reward
    const baseReward = (stake.amount * BigInt(BASE_APY_BPS) * elapsed) / 
      (BigInt(10000) * BigInt(SECONDS_PER_YEAR));
    
    // Apply multiplier
    const multipliedReward = (baseReward * BigInt(stake.multiplier)) / BigInt(10000);
    
    return multipliedReward;
  }

  /**
   * Get APY for lock duration
   */
  getAPY(lockDays: number): number {
    const multipliers: Record<number, number> = {
      0: 10000,
      30: 11000,
      90: 12500,
      180: 15000,
      365: 20000,
    };
    
    const multiplier = multipliers[lockDays] || 10000;
    return (BASE_APY_BPS * multiplier) / 1000000; // Returns percentage
  }

  /**
   * Check if stake is locked
   */
  async isLocked(user: PublicKey): Promise<boolean> {
    const stake = await this.getStake(user);
    if (!stake) return false;
    
    const now = Math.floor(Date.now() / 1000);
    return now < stake.lockUntil;
  }

  // ============================================================================
  // UTILITIES
  // ============================================================================

  /**
   * Format token amount for display
   */
  formatAmount(amount: bigint): string {
    const num = Number(amount) / 1_000_000;
    return num.toLocaleString('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 6,
    }) + ' DRON';
  }

  /**
   * Parse token amount from string
   */
  parseAmount(amount: string): bigint {
    const num = parseFloat(amount.replace(/[^\d.]/g, ''));
    return BigInt(Math.floor(num * 1_000_000));
  }

  // ============================================================================
  // DECODING
  // ============================================================================

  private decodeStakeAccount(data: Buffer): StakeAccount {
    let offset = 8;

    const owner = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const amount = data.readBigUInt64LE(offset);
    offset += 8;

    const stakedAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const lockDuration = Number(data.readBigInt64LE(offset));
    offset += 8;

    const lockUntil = Number(data.readBigInt64LE(offset));
    offset += 8;

    const multiplier = data.readUInt16LE(offset);
    offset += 2;

    const accumulatedRewards = data.readBigUInt64LE(offset);
    offset += 8;

    const lastClaimAt = Number(data.readBigInt64LE(offset));

    return {
      owner,
      amount,
      stakedAt,
      lockDuration,
      lockUntil,
      multiplier,
      accumulatedRewards,
      lastClaimAt,
    };
  }

  private decodeOperatorStakeAccount(data: Buffer): OperatorStakeAccount {
    let offset = 8;

    const operator = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const totalStaked = data.readBigUInt64LE(offset);
    offset += 8;

    const slashableAmount = data.readBigUInt64LE(offset);
    offset += 8;

    const createdAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const hasLastSlash = data.readUInt8(offset) === 1;
    offset += 1;
    const lastSlashAt = hasLastSlash ? Number(data.readBigInt64LE(offset)) : null;
    if (hasLastSlash) offset += 8;

    const reputation = data.readUInt16LE(offset);

    return {
      operator,
      totalStaked,
      slashableAmount,
      createdAt,
      lastSlashAt,
      reputation,
    };
  }
}
