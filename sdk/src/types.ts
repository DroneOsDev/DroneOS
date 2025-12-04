import { PublicKey } from '@solana/web3.js';

// ============================================================================
// ROBOT IDENTITY TYPES
// ============================================================================

export enum RobotClass {
  Drone = 0,
  Ground = 1,
  Marine = 2,
  Industrial = 3,
  Humanoid = 4,
}

export enum RobotStatus {
  Idle = 0,
  Available = 1,
  Busy = 2,
  Maintenance = 3,
  Offline = 4,
  Suspended = 5,
}

export enum Capability {
  Delivery = 0,
  Surveillance = 1,
  Inspection = 2,
  Transport = 3,
  Manipulation = 4,
  Cleaning = 5,
  Security = 6,
  Agriculture = 7,
  Construction = 8,
  Warehouse = 9,
}

export interface RobotAccount {
  deviceId: Uint8Array;
  manufacturerId: string;
  modelId: string;
  firmwareHash: Uint8Array;
  robotClass: RobotClass;
  operator: PublicKey;
  registeredAt: number;
  lastActiveAt: number;
  reputationScore: number;
  totalTasksCompleted: number;
  totalEarnings: bigint;
  status: RobotStatus;
  capabilities: CapabilityProof[];
}

export interface CapabilityProof {
  capability: Capability;
  certificationLevel: number;
  validUntil: number;
  issuer: PublicKey;
}

export interface RegisterRobotParams {
  deviceId: Uint8Array;
  manufacturerId: string;
  modelId: string;
  firmwareHash: Uint8Array;
  robotClass: RobotClass;
}

export interface AddCapabilityParams {
  capability: Capability;
  certificationLevel: number;
  validDays: number;
}

// ============================================================================
// PAYMENT STREAM TYPES
// ============================================================================

export enum StreamStatus {
  Pending = 0,
  Active = 1,
  Paused = 2,
  Completed = 3,
  Cancelled = 4,
  Disputed = 5,
}

export interface PaymentStreamAccount {
  payer: PublicKey;
  payee: PublicKey;
  ratePerSecond: bigint;
  maxDuration: number;
  gracePeriod: number;
  autoTerminate: boolean;
  status: StreamStatus;
  createdAt: number;
  startedAt: number;
  lastTickAt: number;
  totalPaid: bigint;
  totalTicks: number;
  escrowBalance: bigint;
  taskId: PublicKey | null;
}

export interface CreateStreamParams {
  payee: PublicKey;
  ratePerSecond: bigint;
  maxDuration: number;
  gracePeriod?: number;
  autoTerminate?: boolean;
}

// ============================================================================
// TASK MARKET TYPES
// ============================================================================

export enum TaskStatus {
  Open = 0,
  Assigned = 1,
  InProgress = 2,
  PendingVerification = 3,
  Completed = 4,
  Failed = 5,
  Cancelled = 6,
  Disputed = 7,
}

export enum BidStatus {
  Pending = 0,
  Accepted = 1,
  Rejected = 2,
  Withdrawn = 3,
  Expired = 4,
}

export interface TaskAccount {
  creator: PublicKey;
  title: string;
  description: string;
  robotClass: RobotClass;
  requiredCapabilities: Capability[];
  minReputation: number;
  reward: bigint;
  ratePerSecond: bigint;
  estimatedDuration: number;
  priority: number;
  status: TaskStatus;
  createdAt: number;
  expiresAt: number;
  assignedRobot: PublicKey | null;
  assignedAt: number | null;
  startedAt: number | null;
  completedAt: number | null;
  streamId: PublicKey | null;
  progress: number;
  bidsCount: number;
}

export interface BidAccount {
  task: PublicKey;
  robot: PublicKey;
  operator: PublicKey;
  proposedRate: bigint;
  estimatedDuration: number;
  message: string;
  status: BidStatus;
  submittedAt: number;
}

export interface CreateTaskParams {
  title: string;
  description: string;
  robotClass: RobotClass;
  capabilities: Capability[];
  minReputation?: number;
  reward: bigint;
  ratePerSecond?: bigint;
  estimatedDuration: number;
  priority?: number;
  expiresIn: number;
}

export interface SubmitBidParams {
  proposedRate: bigint;
  estimatedDuration: number;
  message?: string;
}

// ============================================================================
// TOKEN TYPES
// ============================================================================

export interface StakeAccount {
  owner: PublicKey;
  amount: bigint;
  stakedAt: number;
  lockDuration: number;
  lockUntil: number;
  multiplier: number;
  accumulatedRewards: bigint;
  lastClaimAt: number;
}

export interface OperatorStakeAccount {
  operator: PublicKey;
  totalStaked: bigint;
  slashableAmount: bigint;
  createdAt: number;
  lastSlashAt: number | null;
  reputation: number;
}

export interface StakeParams {
  amount: bigint;
  lockDays: number;
}

// ============================================================================
// COMMON TYPES
// ============================================================================

export interface TransactionResult {
  signature: string;
  success: boolean;
  error?: string;
}

export interface PDAResult {
  publicKey: PublicKey;
  bump: number;
}

export type EventCallback<T> = (event: T) => void;

// ============================================================================
// EVENT TYPES
// ============================================================================

export interface RobotRegisteredEvent {
  robot: PublicKey;
  deviceId: Uint8Array;
  operator: PublicKey;
  robotClass: RobotClass;
  timestamp: number;
}

export interface StreamCreatedEvent {
  stream: PublicKey;
  payer: PublicKey;
  payee: PublicKey;
  ratePerSecond: bigint;
  escrowAmount: bigint;
  timestamp: number;
}

export interface StreamTickEvent {
  stream: PublicKey;
  tickNumber: number;
  amount: bigint;
  totalPaid: bigint;
  escrowRemaining: bigint;
  timestamp: number;
}

export interface TaskCreatedEvent {
  task: PublicKey;
  creator: PublicKey;
  title: string;
  reward: bigint;
  expiresAt: number;
}

export interface TaskCompletedEvent {
  task: PublicKey;
  robot: PublicKey;
  totalPaid: bigint;
  timestamp: number;
}
