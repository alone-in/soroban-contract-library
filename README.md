# soroban-contract-library

[![Stellar](https://img.shields.io/badge/Stellar-Soroban-blue?logo=stellar)](https://stellar.org)
[![Soroban SDK](https://img.shields.io/badge/soroban--sdk-21.0.0-blueviolet)](https://docs.rs/soroban-sdk)
[![Wave Program](https://img.shields.io/badge/Drips-Wave%20Program-orange)](https://www.drips.network/wave/stellar)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![CI](https://github.com/YOUR_USERNAME/soroban-contract-library/actions/workflows/ci.yml/badge.svg)](https://github.com/YOUR_USERNAME/soroban-contract-library/actions/workflows/ci.yml)

A production-ready library of auditable, reusable Soroban smart contract templates for the Stellar ecosystem. Built with `soroban-sdk v21`, `#![no_std]`, and full test coverage.

---

## Contracts

| Contract | Path | Status | Description |
|---|---|---|---|
| Escrow | `contracts/escrow` | ✅ Ready | Milestone-based trustless escrow with arbiter dispute resolution |
| Vesting | `contracts/vesting` | ✅ Ready | Linear and cliff token vesting schedules with revocation |
| Multisig | `contracts/multisig` | ✅ Ready | M-of-N multisig wallet with auto-execution and expiry |
| DAO Voting | `contracts/dao_voting` | ✅ Ready | On-chain governance with quorum and approval basis points |

---

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown
```

### Clone & Build

```bash
git clone https://github.com/YOUR_USERNAME/soroban-contract-library.git
cd soroban-contract-library

# Build all contracts (native)
cargo build --workspace

# Build WASM artifacts
cargo build --workspace --target wasm32-unknown-unknown --release

# Run all tests
cargo test --workspace
```

---

## Project Structure

```
soroban-contract-library/
├── Cargo.toml                    # Workspace root
├── README.md
├── CONTRIBUTING.md
├── LICENSE
├── .github/
│   └── workflows/
│       └── ci.yml                # CI: test + lint
├── contracts/
│   ├── escrow/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── vesting/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── multisig/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   └── dao_voting/
│       ├── Cargo.toml
│       └── src/lib.rs
└── docs/
    └── WAVE_ISSUES.md            # Pre-written GitHub issues for Wave Program
```

---

## Contract Summaries

### Escrow
Trustless milestone-based escrow. A depositor locks funds split across milestones. An arbiter or depositor can release individual milestones to the beneficiary. Either party can raise a dispute, which the arbiter resolves. Depositor can reclaim unreleased funds after expiry.

**Key functions:** `create`, `release_milestone`, `raise_dispute`, `resolve_dispute`, `reclaim_expired`, `get_escrow`

### Vesting
Token vesting with `Linear` (pro-rata over time) and `Cliff` (all-or-nothing at cliff ledger) modes. Supports revocable schedules where the funder can claw back unvested tokens.

**Key functions:** `create_schedule`, `vested_amount`, `claim`, `revoke`, `get_schedule`

### Multisig
M-of-N multisig wallet. Owners propose token transfers; the proposer auto-approves. Once the approval threshold is reached the transfer executes automatically. Proposals expire at a configurable ledger.

**Key functions:** `initialize`, `propose_transfer`, `approve`, `cancel`, `get_proposal`, `get_config`

### DAO Voting
On-chain governance with configurable quorum (basis points of total supply) and approval ratio (basis points of votes cast). Voters supply their own `voting_power` — making this a clean integration point for token-weighted voting. Passed proposals can execute an optional on-chain token transfer.

**Key functions:** `initialize`, `submit_proposal`, `vote`, `finalize`, `execute`, `cancel`, `get_proposal`, `get_config`

---

## Contributing

Contributions are welcome! This project participates in the **[Stellar Wave Program](https://www.drips.network/wave/stellar)** — open issues are tagged with complexity and point values. See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions and PR guidelines.

---

## License

[MIT](LICENSE) © 2026
