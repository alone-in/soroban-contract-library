#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NETWORK="${STELLAR_NETWORK:-testnet}"
SOURCE_ACCOUNT="${STELLAR_SOURCE_ACCOUNT:-${1:-}}"

usage() {
  cat <<'USAGE'
Usage:
  STELLAR_SOURCE_ACCOUNT=<identity-or-secret> ./scripts/deploy_testnet.sh
  ./scripts/deploy_testnet.sh <identity-or-secret>

Environment:
  STELLAR_NETWORK          Stellar CLI network name. Defaults to "testnet".
  STELLAR_SOURCE_ACCOUNT   Stellar CLI identity, secret key, or seed phrase used
                           as --source-account for every deploy transaction.
USAGE
}

fail() {
  printf 'error: %s\n\n' "$1" >&2
  usage >&2
  exit 1
}

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'error: missing required command: %s\n' "$1" >&2
    return 1
  fi
}

if [[ -z "$SOURCE_ACCOUNT" ]]; then
  fail "missing source account"
fi

if ! require_command stellar; then
  cat >&2 <<'EOF'

Install the Stellar CLI, then retry:
  cargo install --locked stellar-cli
EOF
  exit 1
fi

if ! require_command cargo; then
  exit 1
fi

if command -v rustup >/dev/null 2>&1; then
  if ! rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
    cat >&2 <<'EOF'
error: missing Rust target: wasm32-unknown-unknown

Install it, then retry:
  rustup target add wasm32-unknown-unknown
EOF
    exit 1
  fi
fi

cd "$ROOT_DIR"

printf 'Building release WASM artifacts...\n'
cargo build --workspace --target wasm32-unknown-unknown --release

deploy_contract() {
  local name="$1"
  local artifact="$2"
  local wasm="$ROOT_DIR/target/wasm32-unknown-unknown/release/${artifact}.wasm"

  if [[ ! -f "$wasm" ]]; then
    printf 'error: expected WASM artifact not found: %s\n' "$wasm" >&2
    exit 1
  fi

  printf 'Deploying %s to %s...\n' "$name" "$NETWORK"
  local deploy_output
  deploy_output="$(
    stellar contract deploy \
      --wasm "$wasm" \
      --source-account "$SOURCE_ACCOUNT" \
      --network "$NETWORK"
  )"

  local contract_id
  contract_id="$(printf '%s\n' "$deploy_output" | awk 'NF { line = $0 } END { print line }')"
  if [[ -z "$contract_id" ]]; then
    printf 'error: stellar contract deploy returned no contract id for %s\n' "$name" >&2
    exit 1
  fi

  printf '%-12s %s\n' "$name:" "$contract_id"
}

printf '\nDeployed contract IDs:\n'
deploy_contract "escrow" "escrow"
deploy_contract "vesting" "vesting"
deploy_contract "multisig" "multisig"
deploy_contract "dao_voting" "dao_voting"
