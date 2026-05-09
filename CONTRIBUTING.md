# Contributing to soroban-contract-library

Thank you for your interest in contributing! This project participates in the **[Stellar Wave Program](https://www.drips.network/wave/stellar)** — a bounty program that rewards open-source contributors with points redeemable for USDC.

---

## Dev Setup

### 1. Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### 2. Add the WASM target

```bash
rustup target add wasm32-unknown-unknown
```

### 3. Clone the repo

```bash
git clone https://github.com/YOUR_USERNAME/soroban-contract-library.git
cd soroban-contract-library
```

### 4. Build

```bash
# Native (fast iteration)
cargo build --workspace

# WASM release artifacts
cargo build --workspace --target wasm32-unknown-unknown --release
```

### 5. Run tests

```bash
cargo test --workspace
```

---

## How the Stellar Wave Program Works

The [Drips Wave Program](https://www.drips.network/wave/stellar) rewards contributors for completing issues tagged with a complexity level. Points are awarded per merged PR that closes a tagged issue.

| Complexity | Points |
|---|---|
| Trivial | 100 pts |
| Medium | 150 pts |
| High | 200 pts |

Points accumulate and can be redeemed for USDC via the Drips platform. To participate:

1. Browse open issues in this repo tagged `wave:trivial`, `wave:medium`, or `wave:high`.
2. Comment on the issue to claim it.
3. Fork the repo, implement the change, and open a PR referencing the issue (`Closes #N`).
4. Once merged, points are credited to your Drips account.

---

## PR Guidelines

- One PR per issue. Keep changes focused.
- All new code must include tests. CI must pass.
- Reference the issue in your PR description: `Closes #N`.
- Keep commit messages clear: `feat(escrow): add partial milestone release`.
- Do not bump `soroban-sdk` version without discussion.

---

## Code Style

```bash
# Format
cargo fmt --all

# Lint (warnings are errors in CI)
cargo clippy --workspace -- -D warnings
```

All contracts must:
- Use `#![no_std]`
- Use `env.storage().persistent()` for per-entity state
- Use `env.storage().instance()` for global config/counters
- Emit events via `env.events().publish()` for every state change
- Include a `#[cfg(test)]` module with at least a happy-path and one error-path test

---

## Issue Labels

| Label | Meaning |
|---|---|
| `wave:trivial` | Small, well-scoped task (100 pts) |
| `wave:medium` | Moderate complexity (150 pts) |
| `wave:high` | Significant feature or refactor (200 pts) |
| `good first issue` | Suitable for first-time contributors |
| `bug` | Something is broken |
| `enhancement` | New feature or improvement |

---

## Questions?

Open a GitHub Discussion or reach out on the [Stellar Developer Discord](https://discord.gg/stellardev).
