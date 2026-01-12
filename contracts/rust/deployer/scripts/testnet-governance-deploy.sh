# This script deploys the contracts to testnet so that governance flows can be tested
#!/usr/bin/env bash
set -euo pipefail

# Find repo root and source .env file
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
echo "REPO_ROOT: $REPO_ROOT"

# Parse command line arguments
USE_LEDGER=false
ENV_FILE=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --ledger)
            USE_LEDGER=true
            shift
            ;;
        --env-file)
            ENV_FILE="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--ledger] [--env-file FILE]"
            exit 1
            ;;
    esac
done

# Source env file if provided, otherwise try default .env
if [[ -n "$ENV_FILE" ]]; then
    if [[ ! -f "$ENV_FILE" ]]; then
        echo "Error: env file not found: $ENV_FILE"
        exit 1
    fi
    set -a
    source "$ENV_FILE"
    set +a
elif [[ -f "$REPO_ROOT/.env" ]]; then
    set -a
    source "$REPO_ROOT/.env"
    set +a
fi

# Unset any variables containing "PROXY_ADDRESS" to force fresh deployment
for var in $(env | grep -i "PROXY_ADDRESS" | cut -d= -f1); do
    unset "$var" 2>/dev/null || true
done


RPC_URL="${RPC_URL:-http://localhost:8545}"
OUTPUT_FILE=".env.governance.testnet"
ACCOUNT_INDEX="${ACCOUNT_INDEX:-0}"
OPS_DELAY="${OPS_DELAY:-30}" # 30 seconds default
SAFE_EXIT_DELAY="${SAFE_EXIT_DELAY:-60}" # 60 seconds default

# Helper function to check if RPC URL is localhost
is_localhost_rpc() {
    local url="$1"
    # Check for localhost, 127.0.0.1
    [[ "$url" =~ ^https?://(localhost|127\.0\.0\.1)(:[0-9]+)?(/.*)?$ ]]
}

if is_localhost_rpc "$RPC_URL"; then
    unset ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS
fi

# Function to prompt user for confirmation on real testnets
confirm() {
    local message="${1:-Continue?}"
    if is_localhost_rpc "$RPC_URL"; then
        return 0
    fi
    read -p "$message [y/N] " -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Aborted."
        exit 1
    fi
}

# Use hardcoded anvil addresses only for localhost, otherwise use env vars
if is_localhost_rpc "$RPC_URL"; then
    ESPRESSO_OPS_TIMELOCK_PROPOSERS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
    ESPRESSO_OPS_TIMELOCK_EXECUTORS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
    ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
    ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
    ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
else
    ESPRESSO_OPS_TIMELOCK_PROPOSERS="${ESPRESSO_OPS_TIMELOCK_PROPOSERS:?ESPRESSO_OPS_TIMELOCK_PROPOSERS must be set for non-localhost deployments}"
    ESPRESSO_OPS_TIMELOCK_EXECUTORS="${ESPRESSO_OPS_TIMELOCK_EXECUTORS:?ESPRESSO_OPS_TIMELOCK_EXECUTORS must be set for non-localhost deployments}"
    ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS="${ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS:?ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS must be set for non-localhost deployments}"
    ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS="${ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS:?ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS must be set for non-localhost deployments}"
    ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS="${ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS:?ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS must be set for non-localhost deployments}"
fi

DEPLOY_CMD="cargo run --bin deploy --"
if $USE_LEDGER; then
    DEPLOY_CMD="$DEPLOY_CMD --ledger"
    unset ESPRESSO_SEQUENCER_ETH_MNEMONIC
    unset ESPRESSO_DEPLOYER_ACCOUNT_INDEX
fi

# echo "=== Deploying Governance Contracts ==="
echo "RPC URL: $RPC_URL"
if ! is_localhost_rpc "$RPC_URL"; then
    echo "WARNING: This will deploy to a non-localhost network!"
    confirm "Are you sure you want to proceed with deployment?"
fi

echo ""
echo "### Deploying Ops Timelock ###"
$DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
    --deploy-ops-timelock \
    --ops-timelock-admin "$ESPRESSO_OPS_TIMELOCK_ADMIN" \
    --ops-timelock-delay "$OPS_DELAY" \
    --ops-timelock-proposers "$ESPRESSO_OPS_TIMELOCK_PROPOSERS" \
    --ops-timelock-executors "$ESPRESSO_OPS_TIMELOCK_EXECUTORS" \
    --out "$OUTPUT_FILE"

set -a
source "${OUTPUT_FILE}"
set +a

echo ""
echo "### Deploying Safe Exit Timelock ###"
$DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
    --deploy-safe-exit-timelock \
    --safe-exit-timelock-admin "$ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN" \
    --safe-exit-timelock-delay "$SAFE_EXIT_DELAY" \
    --safe-exit-timelock-proposers "$ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS" \
    --safe-exit-timelock-executors "$ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS" \
    --out "$OUTPUT_FILE"

set -a
source "${OUTPUT_FILE}"
set +a

echo ""
echo "### Deploying Core Contracts (v1) ###"
# Deploy contracts without timelock ownership
BASE_ARGS=(
    --rpc-url "$RPC_URL"
    --account-index "$ACCOUNT_INDEX"
    --multisig-pauser-address "$ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS"
    --token-name "$ESP_TOKEN_NAME"
    --token-symbol "$ESP_TOKEN_SYMBOL"
    --initial-token-supply "$ESP_TOKEN_INITIAL_SUPPLY"
    --initial-token-grant-recipient "$ESP_TOKEN_INITIAL_GRANT_RECIPIENT_ADDRESS"
    --exit-escrow-period "$ESPRESSO_SEQUENCER_STAKE_TABLE_EXIT_ESCROW_PERIOD"
    --sequencer-url "$ESPRESSO_SEQUENCER_URL"
    --mock-espresso-live-network
)

[[ -n "${ESPRESSO_SEQUENCER_PERMISSIONED_PROVER:-}" ]] && \
    BASE_ARGS+=(--permissioned-prover "$ESPRESSO_SEQUENCER_PERMISSIONED_PROVER")

$DEPLOY_CMD "${BASE_ARGS[@]}" \
    --deploy-fee-v1 \
    --deploy-light-client-v1 \
    --deploy-esp-token-v1 \
    --deploy-stake-table-v1 \
    --use-mock \
    --upgrade-light-client-v2 \
    --out "$OUTPUT_FILE"

set -a
source "${OUTPUT_FILE}"
set +a

echo ""
echo "### Deploying Upgrades (v2/v3) ###"
UPGRADE_OUTPUT_FILE="${OUTPUT_FILE}.upgrade"
$DEPLOY_CMD "${BASE_ARGS[@]}" \
    --deploy-reward-claim-v1 \
    --upgrade-esp-token-v2 \
    --upgrade-light-client-v3 \
    --upgrade-stake-table-v2 \
    --use-timelock-owner \
    --out "${UPGRADE_OUTPUT_FILE}"

set -a
source "${UPGRADE_OUTPUT_FILE}"
set +a

# print fee contract owner
echo ""
echo "Fee contract owner: $(cast call "$ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS" "owner()(address)" --rpc-url "$RPC_URL")"
echo ""
echo "### Transferring Fee Contract Ownership to Timelock ###"
$DEPLOY_CMD "${BASE_ARGS[@]}" \
    --transfer-ownership-from-eoa \
    --target-contract FeeContract \
    --transfer-ownership-new-owner "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"

echo ""
echo "### Verifying Deployment ###"
"${REPO_ROOT}/scripts/verify-pos-deployment.sh" --rpc-url "$RPC_URL"
echo ""
echo "Deployment complete!"