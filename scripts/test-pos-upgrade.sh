#!/usr/bin/env bash
# This script forks mainnet and does the main mainnet PoS contract upgrade and runs verification.

# To use:
#  1. create a `mainnet-inputs.env` file with the necessary variables.
#  2. Then run the script.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT_DIR"

ENV_FILE="${1:-mainnet-inputs.env}"

if [[ ! -f "$ENV_FILE" ]]; then
    echo "Error: env file not found: $ENV_FILE"
    echo "Usage: $0 [env-file]"
    exit 1
fi

ANVIL_PORT=8545
ANVIL_RPC="http://localhost:$ANVIL_PORT"
MAINNET_RPC="https://ethereum-rpc.publicnode.com"

cleanup() {
    echo "Cleaning up..."
    if [[ -n "${ANVIL_PID:-}" ]]; then
        kill "$ANVIL_PID" 2>/dev/null || true
    fi
}
trap cleanup EXIT

echo "=== Step 1: Start Anvil (forking mainnet) ==="
if lsof -i :$ANVIL_PORT >/dev/null 2>&1; then
    echo "Port $ANVIL_PORT already in use, killing existing process..."
    lsof -ti :$ANVIL_PORT | xargs kill 2>/dev/null || true
    sleep 1
fi

anvil --port $ANVIL_PORT --fork-url "$MAINNET_RPC" &
ANVIL_PID=$!
echo "Anvil started with PID $ANVIL_PID (forking mainnet)"
sleep 3

echo "=== Step 2: Prepare env file ==="
mkdir -p tmp/pos-test

# Create env file with RPC override for local anvil
grep -v '^ESPRESSO_SEQUENCER_L1_PROVIDER=' "$ENV_FILE" > tmp/pos-test/mainnet-fork.env
echo "ESPRESSO_SEQUENCER_L1_PROVIDER=$ANVIL_RPC" >> tmp/pos-test/mainnet-fork.env

# Load env vars to get proposer addresses for CLI flags
set +u
set -a
source <(grep -v "MNEMONIC" mainnet-inputs.env)
set +a
set -u

echo "Using env file: $ENV_FILE"
echo "RPC overridden to: $ANVIL_RPC"

echo ""
echo "=== Step 3: Perform main upgrade ==="
docker run --rm --network host \
    -v "$ROOT_DIR/tmp/pos-test:/out" \
    --env-file tmp/pos-test/mainnet-fork.env \
    ghcr.io/espressosystems/espresso-sequencer/deploy:main deploy \
    --deploy-ops-timelock \
    --ops-timelock-proposers "$MULTISIG_PROPOSER_1" \
    --ops-timelock-proposers "$MULTISIG_PROPOSER_2" \
    --deploy-safe-exit-timelock \
    --safe-exit-timelock-proposers "$MULTISIG_PROPOSER_1" \
    --safe-exit-timelock-proposers "$MULTISIG_PROPOSER_2" \
    --deploy-reward-claim-v1 \
    --deploy-esp-token \
    --upgrade-esp-token-v2 \
    --deploy-stake-table \
    --upgrade-stake-table-v2 \
    --use-timelock-owner \
    --out /out/mainnet-upgrade-outputs.env

echo ""
echo "Upgrade outputs:"
cat tmp/pos-test/mainnet-upgrade-outputs.env

echo ""
echo "=== Step 4: Post-deployment verification ==="

# Load env files (excluding mnemonic) for verification
set +u
set -a
source <(grep -v "MNEMONIC" tmp/pos-test/mainnet-fork.env)
source tmp/pos-test/mainnet-upgrade-outputs.env
set +a
set -u

# Run verification script
"$SCRIPT_DIR/verify-pos-deployment.sh" --rpc-url "$ANVIL_RPC"

echo ""
echo "=== Upgrade completed successfully! ==="
