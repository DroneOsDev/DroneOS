import { Connection, PublicKey, Keypair, Transaction, SystemProgram } from '@solana/web3.js';
import { PROGRAM_IDS } from './index';
import {
  TaskAccount,
  BidAccount,
  TaskStatus,
  BidStatus,
  RobotClass,
  Capability,
  CreateTaskParams,
  SubmitBidParams,
  TransactionResult,
  PDAResult,
} from './types';

/**
 * Task Market Client
 * 
 * On-chain labor marketplace for robots
 */
export class MarketClient {
  private connection: Connection;
  private wallet: Keypair | null;
  private programId: PublicKey;

  constructor(connection: Connection, wallet?: Keypair) {
    this.connection = connection;
    this.wallet = wallet || null;
    this.programId = PROGRAM_IDS.TASK_MARKET;
  }

  setWallet(wallet: Keypair): void {
    this.wallet = wallet;
  }

  // ============================================================================
  // PDA DERIVATIONS
  // ============================================================================

  getMarketPDA(): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('market')],
      this.programId
    );
    return { publicKey, bump };
  }

  getTaskPDA(creator: PublicKey, taskIndex: number): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('task'),
        creator.toBuffer(),
        Buffer.from(new BigUint64Array([BigInt(taskIndex)]).buffer),
      ],
      this.programId
    );
    return { publicKey, bump };
  }

  getBidPDA(task: PublicKey, robot: PublicKey): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('bid'), task.toBuffer(), robot.toBuffer()],
      this.programId
    );
    return { publicKey, bump };
  }

  // ============================================================================
  // TASK OPERATIONS
  // ============================================================================

  /**
   * Create a new task
   */
  async createTask(
    params: CreateTaskParams,
    creator: Keypair
  ): Promise<{ result: TransactionResult; taskPubkey: PublicKey }> {
    const marketPDA = this.getMarketPDA();
    
    // Get current task count for PDA derivation
    const marketAccount = await this.connection.getAccountInfo(marketPDA.publicKey);
    const taskIndex = marketAccount ? this.decodeTaskCount(marketAccount.data) : 0;
    
    const taskPDA = this.getTaskPDA(creator.publicKey, taskIndex);

    const data = this.encodeCreateTask(params);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: marketPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: taskPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: creator.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [creator]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      
      return { 
        result: { signature, success: true },
        taskPubkey: taskPDA.publicKey,
      };
    } catch (error) {
      return { 
        result: { signature: '', success: false, error: (error as Error).message },
        taskPubkey: taskPDA.publicKey,
      };
    }
  }

  /**
   * Submit a bid on a task
   */
  async submitBid(
    taskPubkey: PublicKey,
    robotPubkey: PublicKey,
    params: SubmitBidParams,
    operator: Keypair
  ): Promise<{ result: TransactionResult; bidPubkey: PublicKey }> {
    const bidPDA = this.getBidPDA(taskPubkey, robotPubkey);

    const messageBytes = Buffer.from(params.message || '');
    const data = Buffer.alloc(8 + 8 + 4 + 4 + messageBytes.length);
    
    let offset = 0;
    data.writeBigUInt64LE(BigInt('0xaaaaaaaaaaaaaaaa'), offset); // discriminator
    offset += 8;
    data.writeBigUInt64LE(params.proposedRate, offset);
    offset += 8;
    data.writeUInt32LE(params.estimatedDuration, offset);
    offset += 4;
    data.writeUInt32LE(messageBytes.length, offset);
    offset += 4;
    messageBytes.copy(data, offset);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: taskPubkey, isSigner: false, isWritable: true },
        { pubkey: bidPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: robotPubkey, isSigner: false, isWritable: false },
        { pubkey: operator.publicKey, isSigner: true, isWritable: true },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [operator]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      
      return { 
        result: { signature, success: true },
        bidPubkey: bidPDA.publicKey,
      };
    } catch (error) {
      return { 
        result: { signature: '', success: false, error: (error as Error).message },
        bidPubkey: bidPDA.publicKey,
      };
    }
  }

  /**
   * Accept a bid
   */
  async acceptBid(
    taskPubkey: PublicKey,
    bidPubkey: PublicKey,
    creator: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0xbbbbbbbbbbbbbbbb'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: taskPubkey, isSigner: false, isWritable: true },
        { pubkey: bidPubkey, isSigner: false, isWritable: true },
        { pubkey: creator.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [creator]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Start task execution
   */
  async startTask(
    taskPubkey: PublicKey,
    robotPubkey: PublicKey,
    operator: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0xcccccccccccccccc'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: taskPubkey, isSigner: false, isWritable: true },
        { pubkey: robotPubkey, isSigner: false, isWritable: false },
        { pubkey: operator.publicKey, isSigner: true, isWritable: false },
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

  /**
   * Update task progress
   */
  async updateProgress(
    taskPubkey: PublicKey,
    robotPubkey: PublicKey,
    progress: number,
    operator: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(9);
    data.writeBigUInt64LE(BigInt('0xdddddddddddddddd'), 0);
    data.writeUInt8(progress, 8);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: taskPubkey, isSigner: false, isWritable: true },
        { pubkey: robotPubkey, isSigner: false, isWritable: false },
        { pubkey: operator.publicKey, isSigner: true, isWritable: false },
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

  /**
   * Complete task
   */
  async completeTask(
    taskPubkey: PublicKey,
    robotPubkey: PublicKey,
    operator: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0xeeeeeeeeeeeeeeee'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: taskPubkey, isSigner: false, isWritable: true },
        { pubkey: robotPubkey, isSigner: false, isWritable: false },
        { pubkey: operator.publicKey, isSigner: true, isWritable: false },
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

  /**
   * Verify task completion
   */
  async verifyCompletion(
    taskPubkey: PublicKey,
    approved: boolean,
    creator: Keypair
  ): Promise<TransactionResult> {
    const marketPDA = this.getMarketPDA();
    
    const data = Buffer.alloc(9);
    data.writeBigUInt64LE(BigInt('0xffffffffffff0000'), 0);
    data.writeUInt8(approved ? 1 : 0, 8);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: marketPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: taskPubkey, isSigner: false, isWritable: true },
        { pubkey: creator.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [creator]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Cancel task
   */
  async cancelTask(
    taskPubkey: PublicKey,
    creator: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0xffffffffffff1111'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: taskPubkey, isSigner: false, isWritable: true },
        { pubkey: creator.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [creator]);
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
   * Get task account
   */
  async getTask(taskPubkey: PublicKey): Promise<TaskAccount | null> {
    const accountInfo = await this.connection.getAccountInfo(taskPubkey);
    if (!accountInfo) return null;
    return this.decodeTaskAccount(accountInfo.data);
  }

  /**
   * Get bid account
   */
  async getBid(bidPubkey: PublicKey): Promise<BidAccount | null> {
    const accountInfo = await this.connection.getAccountInfo(bidPubkey);
    if (!accountInfo) return null;
    return this.decodeBidAccount(accountInfo.data);
  }

  /**
   * Get all bids for a task
   */
  async getTaskBids(taskPubkey: PublicKey): Promise<BidAccount[]> {
    // In production, use getProgramAccounts with filters
    // This is a simplified version
    return [];
  }

  /**
   * Check if task is open for bids
   */
  async isTaskOpen(taskPubkey: PublicKey): Promise<boolean> {
    const task = await this.getTask(taskPubkey);
    if (!task) return false;
    
    const now = Math.floor(Date.now() / 1000);
    return task.status === TaskStatus.Open && task.expiresAt > now;
  }

  // ============================================================================
  // ENCODING/DECODING
  // ============================================================================

  private encodeCreateTask(params: CreateTaskParams): Buffer {
    const titleBytes = Buffer.from(params.title);
    const descBytes = Buffer.from(params.description);
    const capsBytes = Buffer.from(params.capabilities.map(c => c as number));

    const size = 8 + // discriminator
      4 + titleBytes.length +
      4 + descBytes.length +
      1 + // robot_class
      4 + capsBytes.length +
      2 + // min_reputation
      8 + // reward
      8 + // rate_per_second
      4 + // estimated_duration
      1 + // priority
      8;  // expires_in

    const data = Buffer.alloc(size);
    let offset = 0;

    data.writeBigUInt64LE(BigInt('0x9999999999999999'), offset);
    offset += 8;

    data.writeUInt32LE(titleBytes.length, offset);
    offset += 4;
    titleBytes.copy(data, offset);
    offset += titleBytes.length;

    data.writeUInt32LE(descBytes.length, offset);
    offset += 4;
    descBytes.copy(data, offset);
    offset += descBytes.length;

    data.writeUInt8(params.robotClass, offset);
    offset += 1;

    data.writeUInt32LE(capsBytes.length, offset);
    offset += 4;
    capsBytes.copy(data, offset);
    offset += capsBytes.length;

    data.writeUInt16LE(params.minReputation || 0, offset);
    offset += 2;

    data.writeBigUInt64LE(params.reward, offset);
    offset += 8;

    const rate = params.ratePerSecond || params.reward / BigInt(params.estimatedDuration);
    data.writeBigUInt64LE(rate, offset);
    offset += 8;

    data.writeUInt32LE(params.estimatedDuration, offset);
    offset += 4;

    data.writeUInt8(params.priority || 2, offset);
    offset += 1;

    data.writeBigInt64LE(BigInt(params.expiresIn), offset);

    return data;
  }

  private decodeTaskCount(data: Buffer): number {
    // Skip discriminator (8) and authority (32), read total_tasks
    return Number(data.readBigUInt64LE(40));
  }

  private decodeTaskAccount(data: Buffer): TaskAccount {
    let offset = 8; // Skip discriminator

    const creator = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const titleLen = data.readUInt32LE(offset);
    offset += 4;
    const title = data.slice(offset, offset + titleLen).toString();
    offset += titleLen;

    const descLen = data.readUInt32LE(offset);
    offset += 4;
    const description = data.slice(offset, offset + descLen).toString();
    offset += descLen;

    const robotClass = data.readUInt8(offset) as RobotClass;
    offset += 1;

    const capsLen = data.readUInt32LE(offset);
    offset += 4;
    const requiredCapabilities: Capability[] = [];
    for (let i = 0; i < capsLen; i++) {
      requiredCapabilities.push(data.readUInt8(offset + i) as Capability);
    }
    offset += capsLen;

    const minReputation = data.readUInt16LE(offset);
    offset += 2;

    const reward = data.readBigUInt64LE(offset);
    offset += 8;

    const ratePerSecond = data.readBigUInt64LE(offset);
    offset += 8;

    const estimatedDuration = data.readUInt32LE(offset);
    offset += 4;

    const priority = data.readUInt8(offset);
    offset += 1;

    const status = data.readUInt8(offset) as TaskStatus;
    offset += 1;

    const createdAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const expiresAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    // Option fields
    const hasAssignedRobot = data.readUInt8(offset) === 1;
    offset += 1;
    const assignedRobot = hasAssignedRobot ? new PublicKey(data.slice(offset, offset + 32)) : null;
    if (hasAssignedRobot) offset += 32;

    const hasAssignedAt = data.readUInt8(offset) === 1;
    offset += 1;
    const assignedAt = hasAssignedAt ? Number(data.readBigInt64LE(offset)) : null;
    if (hasAssignedAt) offset += 8;

    const hasStartedAt = data.readUInt8(offset) === 1;
    offset += 1;
    const startedAt = hasStartedAt ? Number(data.readBigInt64LE(offset)) : null;
    if (hasStartedAt) offset += 8;

    const hasCompletedAt = data.readUInt8(offset) === 1;
    offset += 1;
    const completedAt = hasCompletedAt ? Number(data.readBigInt64LE(offset)) : null;
    if (hasCompletedAt) offset += 8;

    const hasStreamId = data.readUInt8(offset) === 1;
    offset += 1;
    const streamId = hasStreamId ? new PublicKey(data.slice(offset, offset + 32)) : null;
    if (hasStreamId) offset += 32;

    const progress = data.readUInt8(offset);
    offset += 1;

    const bidsCount = data.readUInt16LE(offset);

    return {
      creator,
      title,
      description,
      robotClass,
      requiredCapabilities,
      minReputation,
      reward,
      ratePerSecond,
      estimatedDuration,
      priority,
      status,
      createdAt,
      expiresAt,
      assignedRobot,
      assignedAt,
      startedAt,
      completedAt,
      streamId,
      progress,
      bidsCount,
    };
  }

  private decodeBidAccount(data: Buffer): BidAccount {
    let offset = 8;

    const task = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const robot = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const operator = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const proposedRate = data.readBigUInt64LE(offset);
    offset += 8;

    const estimatedDuration = data.readUInt32LE(offset);
    offset += 4;

    const messageLen = data.readUInt32LE(offset);
    offset += 4;
    const message = data.slice(offset, offset + messageLen).toString();
    offset += messageLen;

    const status = data.readUInt8(offset) as BidStatus;
    offset += 1;

    const submittedAt = Number(data.readBigInt64LE(offset));

    return {
      task,
      robot,
      operator,
      proposedRate,
      estimatedDuration,
      message,
      status,
      submittedAt,
    };
  }
}
