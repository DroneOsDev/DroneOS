import { Connection, PublicKey, Keypair, Transaction } from '@solana/web3.js';
import { Program, AnchorProvider, Idl } from '@coral-xyz/anchor';

// Program IDs
export const PROGRAM_IDS = {
  IDENTITY_REGISTRY: new PublicKey('DOS4id11111111111111111111111111111111111111'),
  PAYMENT_STREAMS: new PublicKey('DOS4pay1111111111111111111111111111111111111'),
  TASK_MARKET: new PublicKey('DOS4mkt1111111111111111111111111111111111111'),
  DRONEOS_TOKEN: new PublicKey('DOS4tkn1111111111111111111111111111111111111'),
};

// Re-export modules
export * from './identity';
export * from './payments';
export * from './market';
export * from './token';
export * from './types';

/**
 * $DRONEOS Protocol SDK
 * 
 * Main entry point for interacting with DroneOS protocol
 */
export class DroneOS {
  public readonly connection: Connection;
  public readonly identity: IdentityClient;
  public readonly payments: PaymentsClient;
  public readonly market: MarketClient;
  public readonly token: TokenClient;

  constructor(connection: Connection, wallet?: Keypair) {
    this.connection = connection;
    this.identity = new IdentityClient(connection, wallet);
    this.payments = new PaymentsClient(connection, wallet);
    this.market = new MarketClient(connection, wallet);
    this.token = new TokenClient(connection, wallet);
  }

  /**
   * Set wallet for signing transactions
   */
  setWallet(wallet: Keypair): void {
    this.identity.setWallet(wallet);
    this.payments.setWallet(wallet);
    this.market.setWallet(wallet);
    this.token.setWallet(wallet);
  }
}

// Import client implementations
import { IdentityClient } from './identity';
import { PaymentsClient } from './payments';
import { MarketClient } from './market';
import { TokenClient } from './token';
