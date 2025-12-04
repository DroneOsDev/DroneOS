import { Connection, PublicKey, Keypair, Transaction, SystemProgram } from '@solana/web3.js';
import { PROGRAM_IDS } from './index';
import {
  RobotAccount,
  RobotClass,
  RobotStatus,
  Capability,
  RegisterRobotParams,
  AddCapabilityParams,
  TransactionResult,
  PDAResult,
} from './types';

/**
 * Identity Registry Client
 * 
 * Manages robot registration, capabilities, and status
 */
export class IdentityClient {
  private connection: Connection;
  private wallet: Keypair | null;
  private programId: PublicKey;

  constructor(connection: Connection, wallet?: Keypair) {
    this.connection = connection;
    this.wallet = wallet || null;
    this.programId = PROGRAM_IDS.IDENTITY_REGISTRY;
  }

  setWallet(wallet: Keypair): void {
    this.wallet = wallet;
  }

  // ============================================================================
  // PDA DERIVATIONS
  // ============================================================================

  /**
   * Derive registry PDA
   */
  getRegistryPDA(): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('registry')],
      this.programId
    );
    return { publicKey, bump };
  }

  /**
   * Derive robot PDA from device ID
   */
  getRobotPDA(deviceId: Uint8Array): PDAResult {
    const [publicKey, bump] = PublicKey.findProgramAddressSync(
      [Buffer.from('robot'), deviceId],
      this.programId
    );
    return { publicKey, bump };
  }

  // ============================================================================
  // ROBOT REGISTRATION
  // ============================================================================

  /**
   * Register a new robot
   */
  async registerRobot(
    params: RegisterRobotParams,
    operator: Keypair
  ): Promise<TransactionResult> {
    const robotPDA = this.getRobotPDA(params.deviceId);
    const registryPDA = this.getRegistryPDA();

    // Build instruction data
    const data = this.encodeRegisterRobot(params);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: registryPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: robotPDA.publicKey, isSigner: false, isWritable: true },
        { pubkey: operator.publicKey, isSigner: true, isWritable: true },
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
      return { 
        signature: '', 
        success: false, 
        error: (error as Error).message 
      };
    }
  }

  /**
   * Add capability to robot
   */
  async addCapability(
    robotPubkey: PublicKey,
    params: AddCapabilityParams,
    authority: Keypair
  ): Promise<TransactionResult> {
    const data = this.encodeAddCapability(params);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: robotPubkey, isSigner: false, isWritable: true },
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
      return { 
        signature: '', 
        success: false, 
        error: (error as Error).message 
      };
    }
  }

  /**
   * Update robot status
   */
  async updateStatus(
    robotPubkey: PublicKey,
    newStatus: RobotStatus,
    operator: Keypair
  ): Promise<TransactionResult> {
    const data = Buffer.alloc(9);
    data.writeUInt8(2, 0); // Instruction index for update_status
    data.writeUInt8(newStatus, 8);

    const instruction = {
      programId: this.programId,
      keys: [
        { pubkey: robotPubkey, isSigner: false, isWritable: true },
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
      return { 
        signature: '', 
        success: false, 
        error: (error as Error).message 
      };
    }
  }

  // ============================================================================
  // QUERIES
  // ============================================================================

  /**
   * Get robot account data
   */
  async getRobot(robotPubkey: PublicKey): Promise<RobotAccount | null> {
    const accountInfo = await this.connection.getAccountInfo(robotPubkey);
    if (!accountInfo) return null;
    
    return this.decodeRobotAccount(accountInfo.data);
  }

  /**
   * Get robot by device ID
   */
  async getRobotByDeviceId(deviceId: Uint8Array): Promise<RobotAccount | null> {
    const pda = this.getRobotPDA(deviceId);
    return this.getRobot(pda.publicKey);
  }

  /**
   * Check if robot has capability
   */
  async hasCapability(
    robotPubkey: PublicKey,
    capability: Capability
  ): Promise<boolean> {
    const robot = await this.getRobot(robotPubkey);
    if (!robot) return false;

    const now = Math.floor(Date.now() / 1000);
    return robot.capabilities.some(
      (cap) => cap.capability === capability && cap.validUntil > now
    );
  }

  /**
   * Check robot status
   */
  async isAvailable(robotPubkey: PublicKey): Promise<boolean> {
    const robot = await this.getRobot(robotPubkey);
    return robot?.status === RobotStatus.Available;
  }

  // ============================================================================
  // ENCODING/DECODING
  // ============================================================================

  private encodeRegisterRobot(params: RegisterRobotParams): Buffer {
    const manufacturerBytes = Buffer.from(params.manufacturerId);
    const modelBytes = Buffer.from(params.modelId);
    
    const buffer = Buffer.alloc(
      8 + // discriminator
      32 + // device_id
      4 + manufacturerBytes.length + // manufacturer string
      4 + modelBytes.length + // model string
      32 + // firmware_hash
      1 // robot_class
    );

    let offset = 0;
    
    // Discriminator for register_robot
    buffer.writeBigUInt64LE(BigInt('0x1234567890abcdef'), offset);
    offset += 8;
    
    // device_id
    Buffer.from(params.deviceId).copy(buffer, offset);
    offset += 32;
    
    // manufacturer_id (string with length prefix)
    buffer.writeUInt32LE(manufacturerBytes.length, offset);
    offset += 4;
    manufacturerBytes.copy(buffer, offset);
    offset += manufacturerBytes.length;
    
    // model_id
    buffer.writeUInt32LE(modelBytes.length, offset);
    offset += 4;
    modelBytes.copy(buffer, offset);
    offset += modelBytes.length;
    
    // firmware_hash
    Buffer.from(params.firmwareHash).copy(buffer, offset);
    offset += 32;
    
    // robot_class
    buffer.writeUInt8(params.robotClass, offset);

    return buffer;
  }

  private encodeAddCapability(params: AddCapabilityParams): Buffer {
    const buffer = Buffer.alloc(8 + 1 + 1 + 4);
    
    buffer.writeBigUInt64LE(BigInt('0x2345678901bcdef0'), 0); // discriminator
    buffer.writeUInt8(params.capability, 8);
    buffer.writeUInt8(params.certificationLevel, 9);
    buffer.writeUInt32LE(params.validDays, 10);

    return buffer;
  }

  private decodeRobotAccount(data: Buffer): RobotAccount {
    // Skip discriminator (8 bytes)
    let offset = 8;

    const deviceId = data.slice(offset, offset + 32);
    offset += 32;

    const manufacturerLen = data.readUInt32LE(offset);
    offset += 4;
    const manufacturerId = data.slice(offset, offset + manufacturerLen).toString();
    offset += manufacturerLen;

    const modelLen = data.readUInt32LE(offset);
    offset += 4;
    const modelId = data.slice(offset, offset + modelLen).toString();
    offset += modelLen;

    const firmwareHash = data.slice(offset, offset + 32);
    offset += 32;

    const robotClass = data.readUInt8(offset) as RobotClass;
    offset += 1;

    const operator = new PublicKey(data.slice(offset, offset + 32));
    offset += 32;

    const registeredAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const lastActiveAt = Number(data.readBigInt64LE(offset));
    offset += 8;

    const reputationScore = data.readUInt16LE(offset);
    offset += 2;

    const totalTasksCompleted = data.readUInt32LE(offset);
    offset += 4;

    const totalEarnings = data.readBigUInt64LE(offset);
    offset += 8;

    const status = data.readUInt8(offset) as RobotStatus;
    offset += 1;

    // Parse capabilities vector
    const capabilitiesLen = data.readUInt32LE(offset);
    offset += 4;

    const capabilities = [];
    for (let i = 0; i < capabilitiesLen; i++) {
      const capability = data.readUInt8(offset) as Capability;
      offset += 1;
      const certificationLevel = data.readUInt8(offset);
      offset += 1;
      const validUntil = Number(data.readBigInt64LE(offset));
      offset += 8;
      const issuer = new PublicKey(data.slice(offset, offset + 32));
      offset += 32;

      capabilities.push({ capability, certificationLevel, validUntil, issuer });
    }

    return {
      deviceId: new Uint8Array(deviceId),
      manufacturerId,
      modelId,
      firmwareHash: new Uint8Array(firmwareHash),
      robotClass,
      operator,
      registeredAt,
      lastActiveAt,
      reputationScore,
      totalTasksCompleted,
      totalEarnings,
      status,
      capabilities,
    };
  }
}
