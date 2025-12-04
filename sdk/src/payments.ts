import { Connection, PublicKey, Keypair, Transaction, SystemProgram } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { PROGRAM_IDS } from './index';
import {
  PaymentStreamAccount,
  StreamStatus,
  CreateStreamParams,
  TransactionResult,
  PDAResult,
  StreamTickEvent,
} from './types';

/**
 * Payment Streams Client
 * 
 * X402 real-time micropayment implementation
 */
export class PaymentsClient {
  private connection: Connection;
  private wallet: Keypair | null;
  private programId: PublicKey;
  private tickCallbacks: Map<string, (event: StreamTickEvent) => void>;

  constructor(connection: Connection, wallet?: Keypair) {
    this.connection = connection;
    this.wallet = wallet || null;
    this.programId = PROGRAM_IDS.PAYMENT_STREAMS;
    this.tickCallbacks = new Map();
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

  getStreamPDA(payer: PublicKey, payee: PublicKey, timestamp: number): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from('stream'),
        payer.toBuffer(),
        payee.toBuffer(),
        Buffer.from(new BigInt64Array([BigInt(timestamp)]).buffer),
      ],
      this.programId
    );
    return { publicKey, bump };
  }

  getEscrowPDA(stream: PublicKey): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('escrow'), stream.toBuffer()],
      this.programId
    );
    return { publicKey, bump };
  }

  // ============================================================================
  // STREAM OPERATIONS
  // ============================================================================

  /**
   * Create a new payment stream
   */
  async createStream(
    params: CreateStreamParams,
    mint: PublicKey,
    payerTokenAccount: PublicKey,
    payer: Keypair
  ): Promise<{ result: TransactionResult; streamPubkey: PublicKey }> {
    const timestamp = Math.floor(Date.now() / 1000);
    const streamPDA = this.getStreamPDA(payer.publicKey, params.payee, timestamp);
    const escrowPDA = this.getEscrowPDA(streamPDA.publicKey);
    const configPDA = this.getConfigPDA();

    // Encode instruction
    const data = Buffer.alloc(8 + 8 + 8 + 8 + 1);
    let offset = 0;
    
    data.writeBigUInt64LE(BigInt('0x1111111111111111'), offset); // discriminator
    offset += 8;
    data.writeBigUInt64LE(params.ratePerSecond, offset);
    offset += 8;
    data.writeBigInt64LE(BigInt(params.maxDuration), offset);
    offset += 8;
    data.writeBigInt64LE(BigInt(params.gracePeriod || 60), offset);
    offset += 8;
    data.writeUInt8(params.autoTerminate !== false ? 1 : 0, offset);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: configPDA.publicKey, isSigner: false, isWritable: false },
        { pubkey: streamPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: escrowPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: payerTokenAccount, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: true },
        { pubkey: params.payee, isSigner: false, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
        { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [payer]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      
      return { 
        result: { signature, success: true },
        streamPubkey: streamPDA.publicKey,
      };
    } catch (error) {
      return { 
        result: { signature: '', success: false, error: (error as Error).message },
        streamPubkey: streamPDA.publicKey,
      };
    }
  }

  /**
   * Start a pending stream
   */
  async startStream(
    streamPubkey: PublicKey,
    payer: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0x2222222222222222'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: streamPubkey, isSigner: false, isWritable: true },
        { pubkey: payer.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [payer]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Execute a tick (transfer accumulated payment)
   */
  async tick(
    streamPubkey: PublicKey,
    payeeTokenAccount: PublicKey,
    caller: Keypair
  ): Promise<TransactionResult> {
    const escrowPDA = this.getEscrowPDA(streamPubkey);

    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0x3333333333333333'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: streamPubkey, isSigner: false, isWritable: true },
        { pubkey: escrowPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: payeeTokenAccount, isSigner: false, isWritable: true },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [caller]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Pause stream
   */
  async pauseStream(
    streamPubkey: PublicKey,
    authority: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0x4444444444444444'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: streamPubkey, isSigner: false, isWritable: true },
        { pubkey: authority.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [authority]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Resume stream
   */
  async resumeStream(
    streamPubkey: PublicKey,
    authority: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(8);
    data.writeBigUInt64LE(BigInt('0x5555555555555555'), 0);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: streamPubkey, isSigner: false, isWritable: true },
        { pubkey: authority.publicKey, isSigner: true, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [authority]);
      await this.connection.confirmTransaction(signature, 'confirmed');
      return { signature, success: true };
    } catch (error) {
      return { signature: '', success: false, error: (error as Error).message };
    }
  }

  /**
   * Terminate stream
   */
  async terminateStream(
    streamPubkey: PublicKey,
    payerTokenAccount: PublicKey,
    payeeTokenAccount: PublicKey,
    reason: string,
    authority: Keypair
  ): Promise<TransactionResult> {
    const escrowPDA = this.getEscrowPDA(streamPubkey);
    const reasonBytes = Buffer.from(reason);

    const data = Buffer.alloc(8 + 4 + reasonBytes.length);
    data.writeBigUInt64LE(BigInt('0x6666666666666666'), 0);
    data.writeUInt32LE(reasonBytes.length, 8);
    reasonBytes.copy(data, 12);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: streamPubkey, isSigner: false, isWritable: true },
        { pubkey: escrowPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: payerTokenAccount, isSigner: false, isWritable: true },
        { pubkey: payeeTokenAccount, isSigner: false, isWritable: true },
        { pubkey: authority.publicKey, isSigner: true, isWritable: false },
        { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      ],
      data,
    };

    const transaction = new Transaction().add(instruction);

    try {
      const signature = await this.connection.sendTransaction(transaction, [authority]);
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
   * Get stream account
   */
  async getStream(streamPubkey: PublicKey): Promise<PaymentStreamAccount | null> {
    const accountInfo = await this.connection.getAccountInfo(streamPubkey);
    if (!accountInfo) return null;
    return this.decodeStreamAccount(accountInfo.data);
  }

  /**
   * Calculate current amount owed
   */
  async getCurrentDebt(streamPubkey: PublicKey): Promise<bigint> {
    const stream = await this.getStream(streamPubkey);
    if (!stream || stream.status !== StreamStatus.Active) return BigInt(0);

    const now = Math.floor(Date.now() / 1000);
    const elapsed = now - stream.lastTickAt;
    return stream.ratePerSecond * BigInt(elapsed);
  }

  /**
   * Get remaining stream time based on escrow
   */
  async getRemainingTime(streamPubkey: PublicKey): Promise<number> {
    const stream = await this.getStream(streamPubkey);
    if (!stream || stream.ratePerSecond === BigInt(0)) return 0;
    return Number(stream.escrowBalance / stream.ratePerSecond);
  }

  // ============================================================================
  // AUTO-TICK
  // ============================================================================

  /**
   * Start automatic ticking for a stream
   */
  startAutoTick(
    streamPubkey: PublicKey,
    payeeTokenAccount: PublicKey,
    caller: Keypair,
    intervalMs: number = 1000,
    onTick?: (event: StreamTickEvent) => void
  ): () => void {
    const key = streamPubkey.toBase58();
    
    if (onTick) {
      this.tickCallbacks.set(key, onTick);
    }

    const interval = setInterval(async () => {
      const result = await this.tick(streamPubkey, payeeTokenAccount, caller);
      
      if (result.success && onTick) {
        const stream = await this.getStream(streamPubkey);
        if (stream) {
          onTick({
            stream: streamPubkey,
            tickNumber: stream.totalTicks,
            amount: stream.ratePerSecond,
            totalPaid: stream.totalPaid,
            escrowRemaining: stream.escrowBalance,
            timestamp: Math.floor(Date.now() / 1000),
          });
        }
      }
    }, intervalMs);

    // Return stop function
    return () => {
      clearInterval(interval);
      this.tickCallbacks.delete(key);
    };
  }

  // ============================================================================
  // DECODING
  // ============================================================================

  private decodeStreamAccount(data: Buffer): PaymentStreamAccount {
    let offset = 8; // Skip discriminator

    const payer = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const payee = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const ratePerSecond = data.readBigUInt64LE(offset);
    offset += 8;

    const maxDuration = Number(data.readBigInt64LE(offset));
    offset += 8;

    const gracePeriod = Number(data.readBigInt64LE(offset));
    offset += 8;

    const autoTerminate = data.readUInt8(offset) === 1;
    offset += 1;

    const status = data.readUInt8(offset) as StreamStatus;
    offset += 1;

    const createdAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const startedAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const lastTickAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const totalPaid = data.readBigUInt64LE(offset);
    offset += 8;

    const totalTicks = data.readUInt32LE(offset);
    offset += 4;

    const escrowBalance = data.readBigUInt64LE(offset);
    offset += 8;

    // Option<Pubkey> for taskId
    const hasTaskId = data.readUInt8(offset) === 1;
    offset += 1;
    const taskId = hasTaskId ? new PublicKey(data.slice(offset, offset + 32)) : null;

    return {
      payer,
      payee,
      ratePerSecond,
      maxDuration,
      gracePeriod,
      autoTerminate,
      status,
      createdAt,
      startedAt,
      lastTickAt,
      totalPaid,
      totalTicks,
      escrowBalance,
      taskId,
    };
  }
}
