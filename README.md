# $DRONEOS Protocol v1.0 — Core Foundation

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
| v2.0 | Swarm Intelligence | Multi-robot coordination, collective bidding |
| v3.0 | Autonomous Markets | Dynamic pricing, dispute resolution |
| v4.0 | Fleet Sovereignty | DAO governance, cross-chain, insurance |

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
