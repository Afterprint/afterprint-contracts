#!/usr/bin/env bash
# ─── Afterprint — Soroban contract deployment ───────────────────────────────
# Usage: STELLAR_SECRET_KEY=<key> STELLAR_NETWORK=testnet ./scripts/deploy.sh
# Requires: stellar CLI (https://developers.stellar.org/docs/tools/developer-tools/cli/install)
set -euo pipefail

NETWORK="${STELLAR_NETWORK:-testnet}"
SOURCE_KEY="${STELLAR_SECRET_KEY:?Set STELLAR_SECRET_KEY to your Stellar secret key}"

echo "==> Building all contracts (wasm32-unknown-unknown)…"
cd "$(dirname "$0")/.."
cargo build --target wasm32-unknown-unknown --release

echo ""
echo "==> Deploying contracts to $NETWORK…"

CONTRACTS=( case_registry evidence_anchor_registry custody_registry attestation_registry access_grant_registry )

for CONTRACT in "${CONTRACTS[@]}"; do
  WASM="target/wasm32-unknown-unknown/release/${CONTRACT}.wasm"
  if [[ ! -f "$WASM" ]]; then
    echo "  [SKIP] $CONTRACT — wasm not found at $WASM"
    continue
  fi
  echo "  Uploading $CONTRACT…"
  CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM" \
    --source "$SOURCE_KEY" \
    --network "$NETWORK" \
    2>&1 | tail -1)
  echo "  ✓ $CONTRACT => $CONTRACT_ID"
  echo "${CONTRACT^^}_CONTRACT_ID=$CONTRACT_ID" >> .contracts.env
done

echo ""
echo "==> Contract IDs saved to .contracts.env"
echo "    Copy these into your API .env file:"
cat .contracts.env
