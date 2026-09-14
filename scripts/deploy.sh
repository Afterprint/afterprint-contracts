#!/usr/bin/env bash
# ─── Afterprint — Soroban contract deployment ───────────────────────────────
# Usage: STELLAR_ADMIN=<G...> STELLAR_NETWORK=testnet ./scripts/deploy.sh <source-identity>
# <source-identity> is a `stellar keys` alias, secret key, or seed phrase — never pass
# a raw secret key on the command line in a shared shell; prefer an alias.
# Requires: stellar CLI (https://developers.stellar.org/docs/tools/developer-tools/cli/install)
set -euo pipefail

NETWORK="${STELLAR_NETWORK:-testnet}"
SOURCE="${1:?Usage: STELLAR_ADMIN=<G...> ./scripts/deploy.sh <source-identity>}"
ADMIN="${STELLAR_ADMIN:?Set STELLAR_ADMIN to the controller/admin public key (G...)}"

echo "==> Building all contracts (wasm32v1-none)…"
cd "$(dirname "$0")/.."
cargo build --target wasm32v1-none --release

echo ""
echo "==> Deploying contracts to $NETWORK…"

# case_registry's constructor arg is named "admin"; the other four use "controller".
declare -A CONTRACT_ARG=(
  [case_registry]=admin
  [evidence_anchor_registry]=controller
  [custody_registry]=controller
  [attestation_registry]=controller
  [access_grant_registry]=controller
)

rm -f .contracts.env
for CONTRACT in "${!CONTRACT_ARG[@]}"; do
  WASM="target/wasm32v1-none/release/${CONTRACT}.wasm"
  if [[ ! -f "$WASM" ]]; then
    echo "  [SKIP] $CONTRACT — wasm not found at $WASM"
    continue
  fi
  ARG_NAME="${CONTRACT_ARG[$CONTRACT]}"
  echo "  Deploying $CONTRACT (--$ARG_NAME $ADMIN)…"
  CONTRACT_ID=$(stellar contract deploy \
    --wasm "$WASM" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    --alias "$CONTRACT" \
    -- "--$ARG_NAME" "$ADMIN" \
    2>&1 | tail -1)
  echo "  + $CONTRACT => $CONTRACT_ID"
  UPPER=$(echo "$CONTRACT" | tr '[:lower:]' '[:upper:]')
  echo "STELLAR_${UPPER}_ID=$CONTRACT_ID" >> .contracts.env
done

echo ""
echo "==> Contract IDs saved to .contracts.env"
echo "    Copy these into your API's environment (Render dashboard, or afterprint-api/.env locally):"
cat .contracts.env
