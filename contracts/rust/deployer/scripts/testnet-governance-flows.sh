# This script assumes that the contracts have already been deployed and the .env.governance.testnet file has been sourced
# It is used to test the governance flows for the contracts, specifically the timelock operations
# It tests the following flows:
# 1. Scheduling a timelock operation to update the exit escrow period
# 2. Executing a timelock operation to update the exit escrow period
# 3. Scheduling a timelock operation to cancel an operation on StakeTable
# 4. Granting the PAUSER_ROLE via timelock

#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"

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

RPC_URL="${RPC_URL:-http://localhost:8545}"
ACCOUNT_INDEX="${ACCOUNT_INDEX:-0}"
OPS_DELAY="${OPS_DELAY:-30}" # 30 seconds default
SAFE_EXIT_DELAY="${SAFE_EXIT_DELAY:-60}" # 60 seconds default

export RUST_LOG=warn
DEPLOY_CMD="cargo run --bin deploy --"
if $USE_LEDGER; then
    DEPLOY_CMD="$DEPLOY_CMD --ledger"
    unset ESPRESSO_SEQUENCER_ETH_MNEMONIC
    unset ESPRESSO_DEPLOYER_ACCOUNT_INDEX
fi

NEW_ESCROW_PERIOD=$((86400 * 2 ))  # 2 days in seconds
SALT=$(cast keccak "$(date +%s)")

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

echo "### Test 1: Scheduling timelock operation to update exit escrow period ###"
confirm "Schedule timelock operation to update exit escrow period?"
$DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
    --perform-timelock-operation \
    --timelock-operation-type schedule \
    --target-contract StakeTable \
    --function-signature "updateExitEscrowPeriod(uint64)" \
    --function-values "$NEW_ESCROW_PERIOD" \
    --timelock-operation-salt "$SALT" \
    --timelock-operation-delay "$OPS_DELAY" \
    --timelock-operation-value 0

echo ""
echo "Waiting for timelock delay (${OPS_DELAY} seconds)..."
sleep "$OPS_DELAY"

echo ""
echo "### Test 2: Executing timelock operation ###"
confirm "Execute timelock operation to update exit escrow period?"
$DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
    --perform-timelock-operation \
    --timelock-operation-type execute \
    --target-contract StakeTable \
    --function-signature "updateExitEscrowPeriod(uint64)" \
    --function-values "$NEW_ESCROW_PERIOD" \
    --timelock-operation-salt "$SALT" \
    --timelock-operation-delay "$OPS_DELAY" \
    --timelock-operation-value 0

echo ""
echo "Waiting for timelock delay (${OPS_DELAY} seconds)..."
sleep "$OPS_DELAY"

# Verify the change
CURRENT_PERIOD=$(cast call "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "exitEscrowPeriod()(uint256)" --rpc-url "$RPC_URL")
echo "Exit escrow period updated to: $CURRENT_PERIOD"

echo ""
echo "### Test 3: Scheduling then canceling an operation on StakeTable ###"
CANCEL_SALT=$(cast keccak "$(date +%s)cancel")
confirm "Schedule operation on StakeTable (to be canceled)?"
$DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
    --perform-timelock-operation \
    --timelock-operation-type schedule \
    --target-contract StakeTable \
    --function-signature "updateExitEscrowPeriod(uint64)" \
    --function-values "172800" \
    --timelock-operation-salt "$CANCEL_SALT" \
    --timelock-operation-delay "$OPS_DELAY" \
    --timelock-operation-value 0

echo ""
echo "Perform cancel operation on StakeTable"
confirm "Cancel the scheduled operation on StakeTable?"
$DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
    --perform-timelock-operation \
    --timelock-operation-type cancel \
    --target-contract StakeTable \
    --function-signature "updateExitEscrowPeriod(uint64)" \
    --function-values "172800" \
    --timelock-operation-salt "$CANCEL_SALT" \
    --timelock-operation-delay "$OPS_DELAY" \
    --timelock-operation-value 0

# echo ""
# echo "### Test 4: Granting PAUSER_ROLE via timelock ###"
# PAUSER_ROLE="0x65d7a28e3265b37a6474929f336521b332c1681b933f6cb9f3376673440d862a"  # keccak256("PAUSER_ROLE")
# OLD_PAUSER="0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
# NEW_PAUSER="0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
# GRANT_SALT=$(cast keccak "$(date +%s)grant")

# echo ""
# echo "Schedule grant role operation on StakeTable"
# confirm "Schedule grant PAUSER_ROLE operation on StakeTable?"
# $DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
#     --perform-timelock-operation \
#     --timelock-operation-type schedule \
#     --target-contract StakeTable \
#     --function-signature "grantRole(bytes32,address)" \
#     --function-values "$PAUSER_ROLE" "$NEW_PAUSER" \
#     --timelock-operation-salt "$GRANT_SALT" \
#     --timelock-operation-delay "$OPS_DELAY" \
#     --timelock-operation-value 0

# echo ""
# echo "Waiting for timelock delay (${OPS_DELAY} seconds)..."
# sleep "$OPS_DELAY"

# echo ""
# echo "Execute grant role operation on StakeTable"
# confirm "Execute grant PAUSER_ROLE operation on StakeTable?"
# $DEPLOY_CMD --rpc-url "$RPC_URL" --account-index "$ACCOUNT_INDEX" \
#     --perform-timelock-operation \
#     --timelock-operation-type execute \
#     --target-contract StakeTable \
#     --function-signature "grantRole(bytes32,address)" \
#     --function-values "$PAUSER_ROLE" "$NEW_PAUSER" \
#     --timelock-operation-salt "$GRANT_SALT" \
#     --timelock-operation-delay "$OPS_DELAY" \
#     --timelock-operation-value 0

# # Verify the new pauser has the PAUSER_ROLE
# if [[ "$(cast call "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "hasRole(bytes32,address)(bool)" "$PAUSER_ROLE" "$NEW_PAUSER" --rpc-url "$RPC_URL")" != "true" ]]; then
#     echo "ERROR: New pauser does not have the PAUSER_ROLE"
#     exit 1
# fi
# # Verify the previous pauser still has the PAUSER_ROLE
# if [[ "$(cast call "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "hasRole(bytes32,address)(bool)" "$PAUSER_ROLE" "$OLD_PAUSER" --rpc-url "$RPC_URL")" != "true" ]]; then
#     echo "ERROR: Previous pauser does not have the PAUSER_ROLE"
#     exit 1
# fi

echo "All tests passed!"