# $DRONEOS Protocol v4.0 — Oracle Integration & Verification

**DroneOS: Autonomous Robot Leasing Protocol on Solana**

> Robots operate like a swarm economy — renting themselves out, earning revenue, and settling everything through X402 payment streams.

[![Solana](https://img.shields.io/badge/Solana-v1.18-purple.svg)](https://solana.com)
[![Anchor](https://img.shields.io/badge/Anchor-v0.30.1-blue.svg)](https://anchor-lang.com)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

---

## 🚀 Overview

DroneOS enables **fully autonomous robot leasing**:

| Feature | Description |
|---------|-------------|
| **403 Identity** | ZK-based robot authentication without central accounts |
| **X402 Payments** | Real-time streaming payments — stop paying, robot stops working |
| **Task Market** | On-chain labor marketplace with bidding |
| **$DR0N Token** | Staking, rewards, operator accountability |

---

## 📦 Architecture

```
droneos-protocol/
├── programs/                    # Solana Programs (Anchor/Rust)
│   ├── identity-registry/       # 403 Robot Identity System
│   ├── payment-streams/         # X402 Real-time Payments
│   ├── task-market/             # On-chain Labor Market
│   └── token/                   # $DRONEOS Token & Staking
├── sdk/                         # TypeScript SDK
├── tests/                       # Integration Tests
├── app/                         # Demo Frontend (optional)
└── docs/                        # Documentation
```

---

## 🔧 Quick Start

### Prerequisites

```bash
# Install Solana CLI
sh -c "$(curl -sSfL https://release.solana.com/v1.18.0/install)"

# Install Anchor
cargo install --git https://github.com/coral-xyz/anchor anchor-cli

# Install dependencies
npm install
```

### Build & Deploy

```bash
# Build programs
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet

# Run tests
anchor test
```

---

## 📖 Usage

### 1. Register a Robot

```typescript
import { Connection, Keypair } from '@solana/web3.js';
import { DroneOS } from '@droneos/sdk';

const connection = new Connection('https://api.devnet.solana.com');
const droneos = new DroneOS(connection);

// Register robot
const robot = await droneos.identity.registerRobot({
  deviceId: generateDeviceId(),
  manufacturerId: 'DJI',
  modelId: 'Mavic-3',
  firmwareHash: firmwareHash,
  robotClass: 'Drone',
  operator: operatorKeypair,
});

// Add capabilities
await droneos.identity.addCapability(robot, {
  capability: 'Delivery',
  certificationLevel: 3,
  validDays: 365,
});
```

### 2. Create a Task

```typescript
const task = await droneos.market.createTask({
  title: 'Deliver package to downtown',
  description: 'Pickup at warehouse, deliver to 123 Main St',
  robotClass: 'Drone',
  capabilities: ['Delivery'],
  reward: 50_000_000, // 50 DRONEOS
  ratePerSecond: 13889, // ~50 DRONEOS/hour
  estimatedDuration: 3600,
  priority: 2,
  expiresIn: 86400,
}, creatorKeypair);
```

### 3. Bid on Task (Robot)

```typescript
await droneos.market.submitBid(task.publicKey, {
  robot: robotKeypair.publicKey,
  proposedRate: 12000, // Competitive rate
  estimatedDuration: 3000, // 50 minutes
  message: 'Ready for immediate pickup',
}, operatorKeypair);
```

### 4. Accept Bid & Start Payment Stream

```typescript
// Creator accepts bid
await droneos.market.acceptBid(task.publicKey, bidPublicKey, creatorKeypair);

// Robot starts task - payment stream begins automatically
await droneos.market.startTask(task.publicKey, robotKeypair, operatorKeypair);

// Payments flow in real-time...
// If escrow runs out -> robot receives termination signal
```

### 5. Complete Task

```typescript
// Robot completes work
await droneos.market.completeTask(task.publicKey, robotKeypair, operatorKeypair);

// Creator verifies
await droneos.market.verifyCompletion(task.publicKey, true, creatorKeypair);

// Payment finalized, reputation updated
```

---

## 💰 Payment Streams (X402)

The core innovation — **real-time micropayments**:

```
┌─────────────────────────────────────────────────────────┐
│                    PAYMENT STREAM                        │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  Payer ──[$DRONEOS]──> Escrow ──[tick]──> Payee (Robot) │
│                        │                                 │
│                        └── Auto-terminate if empty       │
│                                                          │
│  Rate: 0.01 DRONEOS/second                               │
│  Tick: Every 1 second                                    │
│  Grace Period: 60 seconds                                │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

**Key Features:**
- Pay-per-second billing
- Automatic escrow management
- Instant termination on depletion
- No overpayment — pay only for work done

---

## 🪙 $DRONEOS Token Economics

| Parameter | Value |
|-----------|-------|
| Total Supply | 1,000,000,000 DRONEOS |
| Decimals | 6 |
| Min Stake | 100 DRONEOS |
| Base APY | 12% |

**Staking Multipliers:**

| Lock Period | Multiplier | Effective APY |
|-------------|------------|---------------|
| Flexible | 1.0x | 12% |
| 30 days | 1.1x | 13.2% |
| 90 days | 1.25x | 15% |
| 180 days | 1.5x | 18% |
| 365 days | 2.0x | 24% |

---

## 🤖 Robot Classes & Capabilities

**Classes:**
- `Drone` — Aerial vehicles
- `Ground` — Wheeled/tracked robots
- `Marine` — Water-based robots
- `Industrial` — Factory automation
- `Humanoid` — Bipedal robots

**Capabilities:**
- `Delivery` — Package transport
- `Surveillance` — Monitoring/security
- `Inspection` — Infrastructure checks
- `Transport` — Heavy cargo
- `Manipulation` — Object handling
- `Cleaning` — Sanitation
- `Security` — Protection
- `Agriculture` — Farming tasks
- `Construction` — Building work
- `Warehouse` — Logistics

---

## 🗺️ Roadmap

| Version | Name | Features |
|---------|------|----------|
| **v1.0** | Core Foundation | ✅ Identity, Payments, Market, Token |
| **v2.0** | Web Interface | ✅ Landing Page, Dashboard, Analytics |
| **v3.0** | Swarm Intelligence | ✅ Multi-Robot Coordination, Group Tasks |
| **v4.0** | Oracle Integration | ✅ GPS Verification, Proof of Completion, Disputes |

---

## 🔮 Oracle Verifier (v4.0 New!)

### Decentralized Verification System
Trust-minimized verification infrastructure for robot task completion:

**Verification Types:**
- **GPS Location Proof**: Chainlink/Pyth oracle verification of coordinates
- **Completion Proof**: Cryptographic photo/sensor data validation
- **IoT Sensor Data**: Real-time telemetry verification
- **Dispute Resolution**: Community-driven challenge system

### How It Works

**1. Register Oracle Provider**
```typescript
const oracle = await droneos.oracle.registerOracle({
  type: 'GPS',
  endpoint: 'https://chainlink-gps-adapter.com',
  reputation: 95,
  provider: providerKeypair
});
```

**2. Submit GPS Proof During Task**
```typescript
await droneos.oracle.submitGPSProof(task.publicKey, {
  latitude: 40_756_0800, // 40.7568° N (fixed point)
  longitude: -73_986_4400, // -73.9864° W
  altitude: 45, // meters
  timestamp: Date.now(),
  signature: robotSignature, // Ed25519 from robot
  robot: robotKeypair,
  oracle: oracleKeypair
});
```

**3. Submit Completion Proof**
```typescript
await droneos.oracle.submitCompletionProof(task.publicKey, {
  dataHash: sha256(photoData),
  proofUrl: 'ipfs://QmXx...', // Photo on IPFS
  metadata: JSON.stringify({
    cameraModel: 'DJI Mavic 3',
    timestamp: Date.now(),
    gpsCoords: [40.7568, -73.9864]
  }),
  robot: robotKeypair
});
```

**4. Oracle Verifies Proof**
```typescript
// Chainlink node calls this
await droneos.oracle.verifyProof(proof.publicKey, {
  confidenceScore: 95, // 95% confidence
  isValid: true,
  verificationData: JSON.stringify({
    matchScore: 0.95,
    method: 'GPS triangulation + visual recognition'
  }),
  oracleAuthority: nodeKeypair
});
```

**5. Dispute if Suspicious**
```typescript
await droneos.oracle.createDispute(proof.publicKey, {
  reason: 'Photo timestamp doesn\'t match GPS timestamp',
  evidenceUrl: 'ipfs://QmYy...',
  challenger: challengerKeypair
});

// Community votes (weighted by staked DRONEOS)
await droneos.oracle.voteOnDispute(dispute.publicKey, {
  voteForChallenger: true, // or false
  voter: voterKeypair
});
```

### Features

**Multi-Oracle Support:**
- Chainlink GPS feeds
- Pyth Network price/data feeds
- Custom oracle integrations
- Reputation-weighted consensus

**Cryptographic Proofs:**
- Ed25519 signatures from robots
- SHA256 hashes for data integrity
- IPFS/Arweave for permanent storage
- Timestamp verification

**Dispute System:**
- 7-day voting period
- Stake-weighted voting
- Automatic resolution
- Slashing for false reports

**Confidence Scoring:**
- Minimum 80% confidence required
- Multiple oracle validation
- Historical accuracy tracking
- Oracle reputation system

### Benefits

**For Task Creators:**
- ✅ Verifiable proof of completion
- ✅ Reduced fraud
- ✅ Automated verification
- ✅ Dispute resolution

**For Robot Operators:**
- ✅ Protect reputation
- ✅ Provable work completion
- ✅ Fair dispute process
- ✅ Increased trust

**For Oracles:**
- ✅ Earn verification fees
- ✅ Build reputation
- ✅ Decentralized infrastructure
- ✅ Quality scoring

### Program ID
`DOS4orc1111111111111111111111111111111111111`

---

## 🤖 Swarm Coordinator (v3.0)

### Multi-Robot Coordination
Revolutionary swarm intelligence system enabling robots to work together:

**Key Features:**
- **Swarm Formation**: Create groups of 2-20 robots with reputation requirements
- **Group Tasks**: Multi-robot tasks with coordinated execution
- **Collective Bidding**: Swarms bid as a unit for better rates
- **Performance Tracking**: Contribution-based reward distribution
- **Dynamic Coordination**: Real-time task synchronization

### How It Works

**1. Create a Swarm**
```typescript
const swarm = await droneos.swarm.createSwarm({
  name: "Delivery Fleet Alpha",
  maxRobots: 5,
  minReputation: 85,
  leader: operatorKeypair
});
```

**2. Robots Join Swarm**
```typescript
await droneos.swarm.joinSwarm(swarm.publicKey, {
  robot: robotKeypair,
  operator: operatorKeypair
});
```

**3. Create Group Task**
```typescript
const groupTask = await droneos.swarm.createGroupTask({
  title: "Warehouse inventory scan",
  requiredRobots: 4,
  totalReward: 200_000_000, // 200 DRONEOS
  duration: 7200, // 2 hours
  creator: clientKeypair
});
```

**4. Swarm Bids Collectively**
```typescript
await droneos.swarm.submitSwarmBid(groupTask.publicKey, {
  swarm: swarm.publicKey,
  proposedRate: 11000, // Better rate than individual
  estimatedDuration: 6000, // 1.67 hours
  leader: leaderKeypair
});
```

**5. Rewards Distributed by Contribution**
```typescript
// Automatic distribution based on performance:
// Robot A: 105% contribution → 52.5 DRONEOS
// Robot B: 100% contribution → 50 DRONEOS  
// Robot C: 95% contribution → 47.5 DRONEOS
// Robot D: 100% contribution → 50 DRONEOS
```

### Benefits

**For Clients:**
- ✅ More reliable task completion
- ✅ Better pricing through collective bidding
- ✅ Faster execution with parallelization
- ✅ Built-in redundancy

**For Operators:**
- ✅ Access to high-value group tasks
- ✅ Reputation pooling
- ✅ Shared risk
- ✅ Performance-based bonuses

### Program ID
`DOS4swm1111111111111111111111111111111111111`

---

## 🌐 Web Interface (v2.0)

### Landing Page
Professional cyberpunk-themed landing with:
- Particle effects and smooth animations
- Protocol features showcase
- Real-time network statistics
- Wallet connection integration

### Operator Dashboard
Complete control panel for robot operators:
- **Fleet Management**: Register and monitor robots
- **Task Marketplace**: Browse, filter, and bid on tasks
- **Live Analytics**: Earnings, reputation, payment streams
- **Activity Feed**: Real-time updates on your operations
- **Quick Actions**: One-click task acceptance and rewards claiming

### Getting Started
```bash
cd app
npm install
npm run dev  # Launch at http://localhost:3000
```

**Tech Stack:**
- Next.js 14 + TypeScript
- Solana Wallet Adapter (Phantom, Solflare)
- Framer Motion for smooth animations
- Recharts for data visualization
- Tailwind CSS with custom cyberpunk theme

---

## 📁 Program IDs

| Program | Devnet ID |
|---------|-----------|
| Identity Registry | `DOS4id11111111111111111111111111111111111111` |
| Payment Streams | `DOS4pay1111111111111111111111111111111111111` |
| Task Market | `DOS4mkt1111111111111111111111111111111111111` |
| $DRONEOS Token | `DOS4tkn1111111111111111111111111111111111111` |

*Note: Deploy your own instances and update these IDs*

---

## 🧪 Testing

```bash
# Run all tests
anchor test

# Run specific test
anchor test --skip-build tests/identity.ts

# Test on devnet
anchor test --provider.cluster devnet
```

---

## 📚 Documentation

- [Architecture Guide](docs/architecture.md)
- [SDK Reference](docs/sdk-reference.md)
- [Integration Guide](docs/integration.md)
- [API Documentation](docs/api.md)

---

## 🤝 Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing`)
5. Open Pull Request

---

## 📄 License

MIT License — see [LICENSE](LICENSE) for details.

---

**$DRONEOS Protocol** — *Autonomous robots. Autonomous economy.*

```
██████╗ ██████╗  ██████╗ ███╗   ██╗███████╗ ██████╗ ███████╗
██╔══██╗██╔══██╗██╔═══██╗████╗  ██║██╔════╝██╔═══██╗██╔════╝
██║  ██║██████╔╝██║   ██║██╔██╗ ██║█████╗  ██║   ██║███████╗
██║  ██║██╔══██╗██║   ██║██║╚██╗██║██╔══╝  ██║   ██║╚════██║
██████╔╝██║  ██║╚██████╔╝██║ ╚████║███████╗╚██████╔╝███████║
╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝╚══════╝ ╚═════╝ ╚══════╝
```
