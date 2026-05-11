# Testnet Deployment

This directory contains helper scripts for deploying the workspace contracts to
Stellar testnet.

## Prerequisites

- Rust with the `wasm32-unknown-unknown` target installed.
- Stellar CLI installed and available as `stellar`.
- A funded testnet identity or account that the Stellar CLI can use as
  `--source-account`.

Install the Rust target:

```bash
rustup target add wasm32-unknown-unknown
```

Install the Stellar CLI:

```bash
cargo install --locked stellar-cli
```

Create and fund a testnet identity:

```bash
stellar keys generate alice --network testnet --fund
stellar keys address alice
```

## Deploy All Contracts

From the repository root:

```bash
./scripts/deploy_testnet.sh alice
```

Or pass the source account through the environment:

```bash
STELLAR_SOURCE_ACCOUNT=alice ./scripts/deploy_testnet.sh
```

The script builds all workspace contracts with:

```bash
cargo build --workspace --target wasm32-unknown-unknown --release
```

It then deploys:

- `escrow`
- `vesting`
- `multisig`
- `dao_voting`

Each deployed contract ID is printed to stdout.

## Configuration

The script defaults to the Stellar CLI `testnet` network. Override it with:

```bash
STELLAR_NETWORK=testnet ./scripts/deploy_testnet.sh alice
```

`STELLAR_SOURCE_ACCOUNT` can be a Stellar CLI identity, secret key, or seed
phrase accepted by `stellar contract deploy --source-account`.
