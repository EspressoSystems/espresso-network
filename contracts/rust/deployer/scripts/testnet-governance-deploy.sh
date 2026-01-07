# This script deploys the contracts to testnet so that governance flows can be tested
#!/usr/bin/env bash
set -euo pipefail

# Find repo root and source .env file
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
echo "REPO_ROOT: $REPO_ROOT"
if [[ -f "$REPO_ROOT/.env" ]]; then
    set -a
    source "$REPO_ROOT/.env"
    set +a

     # Unset any variables containing "PROXY_ADDRESS" to force fresh deployment
    for var in $(env | grep -i "PROXY_ADDRESS" | cut -d= -f1); do
        unset "$var" 2>/dev/null || true
    done

    unset ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS
fi

RPC_URL="${RPC_URL:-http://localhost:8545}"
OUTPUT_FILE=".env.governance.testnet"
ACCOUNT_INDEX="${ESPRESSO_DEPLOYER_ACCOUNT_INDEX:-0}"
OPS_DELAY="30" # 30 seconds
SAFE_EXIT_DELAY="60" # 60 seconds
ESPRESSO_OPS_TIMELOCK_PROPOSERS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
ESPRESSO_OPS_TIMELOCK_EXECUTORS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"

DEPLOY_CMD="cargo run --bin deploy --release --"

# Deploy timelocks
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
echo "Fee contract owner: $(cast call "$ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS" "owner()(address)" --rpc-url "$RPC_URL")"
$DEPLOY_CMD "${BASE_ARGS[@]}" \
    --transfer-ownership-from-eoa \
    --target-contract FeeContract \
    --transfer-ownership-new-owner "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"


"${REPO_ROOT}/scripts/verify-pos-deployment.sh" --rpc-url "$RPC_URL"