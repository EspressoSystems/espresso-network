#!/usr/bin/env bash
# Verify governance deployment matches expected configuration

# Required env vars: timelock addresses, admin/proposer/executor addresses, contract proxy addresses
# Optional: RPC_URL (defaults to localhost:8545), timelock delays, multisig pauser, token supply, light client config

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

RPC_URL="${RPC_URL:-http://localhost:8545}"

while [[ $# -gt 0 ]]; do
    case $1 in
        --rpc-url)
            RPC_URL="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--rpc-url RPC_URL] [--help|-h]"
            echo ""
            echo "Verify governance deployment matches expected configuration."
            echo ""
            echo "Environment Variables:"
            echo ""
            echo "    ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"
            echo "    ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS"
            echo "    ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS"
            echo "    ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS"
            echo "    ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS"
            echo "    ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS"
            echo "    ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS"
            echo "    ESPRESSO_OPS_TIMELOCK_ADMIN"
            echo "    ESPRESSO_OPS_TIMELOCK_PROPOSERS"
            echo "    ESPRESSO_OPS_TIMELOCK_EXECUTORS"
            echo "    ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN"
            echo "    ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS"
            echo "    ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS"
            echo "    ESPRESSO_OPS_TIMELOCK_DELAY"
            echo "    ESPRESSO_SAFE_EXIT_TIMELOCK_DELAY"
            echo "    ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS"
            echo "    ESP_TOKEN_INITIAL_SUPPLY"
            echo "    ESPRESSO_LIGHT_CLIENT_BLOCKS_PER_EPOCH"
            echo "    ESPRESSO_LIGHT_CLIENT_EPOCH_START_BLOCK"
            echo "    RPC_URL"
            echo ""
            echo "Examples:"
            echo "  # Source from .env file"
            echo "  source .env && $0"
            echo ""
            echo "  # Set variables inline"
            echo "  ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS=0x... $0 --rpc-url https://eth-mainnet.g.alchemy.com/v2/KEY"
            echo ""
            echo "Note: The script will skip checks for any unset variables and show warnings."
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}"
            echo "Usage: $0 [--rpc-url RPC_URL]"
            exit 1
            ;;
    esac
done

OPS_DELAY_EXPECTED="${ESPRESSO_OPS_TIMELOCK_DELAY:-172800}"
SAFE_EXIT_DELAY_EXPECTED="${ESPRESSO_SAFE_EXIT_TIMELOCK_DELAY:-1209600}"

check_version() {
    local contract_addr="$1"
    local expected_major="$2"
    local name="$3"
    local version_output=$(cast call "$contract_addr" "getVersion()(uint8,uint8,uint8)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    local major=$(echo "$version_output" | awk '{print $1}')
    [ "$major" -eq "$expected_major" ] && echo -e "${GREEN}✓${NC} $name version: $version_output (V$expected_major)" || echo -e "${RED}✗${NC} $name version: $version_output (expected major version $expected_major)"
}

check_has_role() {
    local contract_addr="$1"
    local role="$2"
    local address="$3"
    local description="$4"
    local has_role=$(cast call "$contract_addr" "hasRole(bytes32,address)(bool)" "$role" "$address" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    [ "$has_role" = "true" ] && echo -e "${GREEN}✓${NC} $description" || echo -e "${RED}✗${NC} $description mismatch: expected $address"
}

check_owner() {
    local contract_addr="$1"
    local expected_owner="$2"
    local description="$3"
    local owner=$(cast call "$contract_addr" "owner()(address)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    local owner_lower=$(echo "$owner" | tr '[:upper:]' '[:lower:]')
    local expected_owner_lower=$(echo "$expected_owner" | tr '[:upper:]' '[:lower:]')
    [ "$owner_lower" = "$expected_owner_lower" ] && echo -e "${GREEN}✓${NC} $description" || echo -e "${RED}✗${NC} $description mismatch: expected $expected_owner"
}

echo ""
echo "++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++"
echo "Verifying deployment matches expected configuration based on env vars"
echo "++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++"
echo ""

# Check timelock delays
if [ -n "${ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS:-}" ]; then
    OPS_DELAY=$(cast call "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "getMinDelay()(uint256)" --rpc-url "$RPC_URL" | awk '{print $1}')
    [ "$OPS_DELAY" -eq "$OPS_DELAY_EXPECTED" ] && echo -e "${GREEN}✓${NC} Ops Timelock delay: $OPS_DELAY seconds" || echo -e "${RED}✗${NC} Ops Timelock delay: $OPS_DELAY seconds (expected $OPS_DELAY_EXPECTED)"
else
    echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS not set, skipping Ops Timelock checks"
fi

if [ -n "${ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS:-}" ]; then
    SAFE_EXIT_DELAY=$(cast call "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "getMinDelay()(uint256)" --rpc-url "$RPC_URL" | awk '{print $1}')
    if [ "$SAFE_EXIT_DELAY" -eq "$SAFE_EXIT_DELAY_EXPECTED" ]; then
        echo -e "${GREEN}✓${NC} SafeExit Timelock delay: $SAFE_EXIT_DELAY seconds"
    else
        echo -e "${RED}✗${NC} SafeExit Timelock delay: $SAFE_EXIT_DELAY seconds (expected $SAFE_EXIT_DELAY_EXPECTED)"
    fi
else
    echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS not set, skipping SafeExit Timelock checks"
fi

# Check timelock roles
[ -n "${ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS:-}" ] && {
    OPS_DEFAULT_ADMIN_ROLE=$(cast call "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "DEFAULT_ADMIN_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    OPS_PROPOSER_ROLE=$(cast call "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "PROPOSER_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    OPS_EXECUTOR_ROLE=$(cast call "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "EXECUTOR_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    check_has_role "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "$OPS_DEFAULT_ADMIN_ROLE" "$ESPRESSO_OPS_TIMELOCK_ADMIN" "Ops Timelock admin: $ESPRESSO_OPS_TIMELOCK_ADMIN"
    check_has_role "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "$OPS_PROPOSER_ROLE" "$ESPRESSO_OPS_TIMELOCK_PROPOSERS" "Ops Timelock proposer: $ESPRESSO_OPS_TIMELOCK_PROPOSERS"
    check_has_role "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "$OPS_EXECUTOR_ROLE" "$ESPRESSO_OPS_TIMELOCK_EXECUTORS" "Ops Timelock executor: $ESPRESSO_OPS_TIMELOCK_EXECUTORS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS not set, skipping Ops Timelock role checks"

[ -n "${ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS:-}" ] && {
    SAFE_EXIT_DEFAULT_ADMIN_ROLE=$(cast call "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "DEFAULT_ADMIN_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    SAFE_EXIT_PROPOSER_ROLE=$(cast call "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "PROPOSER_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    SAFE_EXIT_EXECUTOR_ROLE=$(cast call "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "EXECUTOR_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    check_has_role "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "$SAFE_EXIT_DEFAULT_ADMIN_ROLE" "$ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN" "SafeExit Timelock admin: $ESPRESSO_SAFE_EXIT_TIMELOCK_ADMIN"
    check_has_role "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "$SAFE_EXIT_PROPOSER_ROLE" "$ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS" "SafeExit Timelock proposer: $ESPRESSO_SAFE_EXIT_TIMELOCK_PROPOSERS"
    check_has_role "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "$SAFE_EXIT_EXECUTOR_ROLE" "$ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS" "SafeExit Timelock executor: $ESPRESSO_SAFE_EXIT_TIMELOCK_EXECUTORS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS not set, skipping SafeExit Timelock role checks"

echo ""

# Check contract ownership
[ -n "${ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS:-}" ] && [ -n "${ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS:-}" ] && {
    check_owner "$ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS" "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "EspToken owned by SafeExit Timelock: $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS not set, skipping EspToken ownership check"

[ -n "${ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS:-}" ] && [ -n "${ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS:-}" ] && {
    check_owner "$ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS" "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "FeeContract owned by Ops Timelock: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS not set, skipping FeeContract ownership check"

[ -n "${ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS:-}" ] && [ -n "${ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS:-}" ] && {
    check_owner "$ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS" "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "LightClient owned by Ops Timelock: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS not set, skipping LightClient ownership check"

[ -n "${ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS:-}" ] && [ -n "${ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS:-}" ] && {
    check_owner "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "StakeTable owned by Ops Timelock: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS not set, skipping StakeTable ownership check"

# Check admin roles
[ -n "${ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS:-}" ] && {
    ST_DEFAULT_ADMIN_ROLE=$(cast call "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "DEFAULT_ADMIN_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    check_has_role "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "$ST_DEFAULT_ADMIN_ROLE" "$ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS" "StakeTable admin: $ESPRESSO_SEQUENCER_OPS_TIMELOCK_ADDRESS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS not set, skipping StakeTable admin check"

[ -n "${ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS:-}" ] && {
    RC_DEFAULT_ADMIN_ROLE=$(cast call "$ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS" "DEFAULT_ADMIN_ROLE()(bytes32)" --rpc-url "$RPC_URL" 2>/dev/null | tr '\n' ' ' | xargs)
    check_has_role "$ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS" "$RC_DEFAULT_ADMIN_ROLE" "$ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS" "RewardClaim admin: $ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_ADDRESS"
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS not set, skipping RewardClaim admin check"

# Check pauser roles
[ -n "${ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS:-}" ] && {
    [ -n "${ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS:-}" ] && {
        ST_PAUSER_ROLE=$(cast call "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "PAUSER_ROLE()(bytes32)" --rpc-url "$RPC_URL")
        check_has_role "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "$ST_PAUSER_ROLE" "$ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS" "Multisig has PAUSER_ROLE on StakeTable: $ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS"
    }
    [ -n "${ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS:-}" ] && {
        RC_PAUSER_ROLE=$(cast call "$ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS" "PAUSER_ROLE()(bytes32)" --rpc-url "$RPC_URL")
        check_has_role "$ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS" "$RC_PAUSER_ROLE" "$ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS" "Multisig has PAUSER_ROLE on RewardClaim: $ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS"
    }
} || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_ETH_MULTISIG_PAUSER_ADDRESS not set, skipping pauser role checks"

# Check contract versions
[ -n "${ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS:-}" ] && check_version "$ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS" 3 "LightClient" || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS not set, skipping version check"
[ -n "${ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS:-}" ] && check_version "$ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS" 2 "EspToken" || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS not set, skipping version check"
[ -n "${ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS:-}" ] && check_version "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" 2 "StakeTable" || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS not set, skipping version check"
[ -n "${ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS:-}" ] && check_version "$ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS" 1 "RewardClaim" || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS not set, skipping version check"
[ -n "${ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS:-}" ] && check_version "$ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS" 1 "FeeContract" || echo -e "${YELLOW}⚠${NC} ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS not set, skipping version check"

# Check EspToken <-> RewardClaim link
[ -n "${ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS:-}" ] && [ -n "${ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS:-}" ] && {
    ESP_REWARD_CLAIM=$(cast call "$ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS" "rewardClaim()(address)" --rpc-url "$RPC_URL" 2>/dev/null)
    ESP_REWARD_CLAIM_LOWER=$(echo "$ESP_REWARD_CLAIM" | tr '[:upper:]' '[:lower:]')
    ESP_RC_ADDR_LOWER=$(echo "$ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS" | tr '[:upper:]' '[:lower:]')
    [ "$ESP_REWARD_CLAIM_LOWER" = "$ESP_RC_ADDR_LOWER" ] && echo -e "${GREEN}✓${NC} EspToken reward claim: $ESP_REWARD_CLAIM" || echo -e "${RED}✗${NC} EspToken reward claim: $ESP_REWARD_CLAIM (expected $ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS)"
    
    RC_ESP_TOKEN=$(cast call "$ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS" "espToken()(address)" --rpc-url "$RPC_URL" 2>/dev/null)
    RC_ESP_TOKEN_LOWER=$(echo "$RC_ESP_TOKEN" | tr '[:upper:]' '[:lower:]')
    ESP_TOKEN_ADDR_LOWER=$(echo "$ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS" | tr '[:upper:]' '[:lower:]')
    [ "$RC_ESP_TOKEN_LOWER" = "$ESP_TOKEN_ADDR_LOWER" ] && echo -e "${GREEN}✓${NC} RewardClaim espToken: $RC_ESP_TOKEN" || echo -e "${RED}✗${NC} RewardClaim espToken: $RC_ESP_TOKEN (expected $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS)"
}

# Check token supply
if [ -n "${ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS:-}" ]; then
    if [ -n "${ESP_TOKEN_INITIAL_SUPPLY:-}" ]; then
        ESP_TOKEN_SUPPLY=$(cast call "$ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS" "totalSupply()(uint256)" --rpc-url "$RPC_URL" 2>/dev/null | awk '{print $1}')
        ESP_INITIAL_SUPPLY_IN_WEI=$(echo "$ESP_TOKEN_INITIAL_SUPPLY * 10^18" | bc)
        [ $(echo "$ESP_TOKEN_SUPPLY == $ESP_INITIAL_SUPPLY_IN_WEI" | bc) -eq 1 ] && echo -e "${GREEN}✓${NC} EspToken supply: $ESP_TOKEN_SUPPLY" || echo -e "${RED}✗${NC} EspToken supply: $ESP_TOKEN_SUPPLY (expected $ESP_INITIAL_SUPPLY_IN_WEI)"
    else
        echo -e "${YELLOW}⚠${NC} ESP_TOKEN_INITIAL_SUPPLY not set, skipping supply check"
    fi
fi
# Check LightClient config
[ -n "${ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS:-}" ] && {
    if [ -n "${ESPRESSO_LIGHT_CLIENT_BLOCKS_PER_EPOCH:-}" ]; then
        LC_BLOCKS_PER_EPOCH=$(cast call "$ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS" "blocksPerEpoch()(uint64)" --rpc-url "$RPC_URL" 2>/dev/null | awk '{print $1}')
        [ "$LC_BLOCKS_PER_EPOCH" -eq "$ESPRESSO_LIGHT_CLIENT_BLOCKS_PER_EPOCH" ] && echo -e "${GREEN}✓${NC} LightClient blocks per epoch: $LC_BLOCKS_PER_EPOCH" || echo -e "${RED}✗${NC} LightClient blocks per epoch: $LC_BLOCKS_PER_EPOCH (expected $ESPRESSO_LIGHT_CLIENT_BLOCKS_PER_EPOCH)"
    else
        echo -e "${YELLOW}⚠${NC} ESPRESSO_LIGHT_CLIENT_BLOCKS_PER_EPOCH not set, skipping blocks per epoch check"
    fi
    if [ -n "${ESPRESSO_LIGHT_CLIENT_EPOCH_START_BLOCK:-}" ]; then
        LC_EPOCH_START_BLOCK=$(cast call "$ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS" "epochStartBlock()(uint64)" --rpc-url "$RPC_URL" 2>/dev/null | awk '{print $1}')
        [ "$LC_EPOCH_START_BLOCK" -eq "$ESPRESSO_LIGHT_CLIENT_EPOCH_START_BLOCK" ] && echo -e "${GREEN}✓${NC} LightClient epoch start block: $LC_EPOCH_START_BLOCK" || echo -e "${RED}✗${NC} LightClient epoch start block: $LC_EPOCH_START_BLOCK (expected $ESPRESSO_LIGHT_CLIENT_EPOCH_START_BLOCK)"
    else
        echo -e "${YELLOW}⚠${NC} ESPRESSO_LIGHT_CLIENT_EPOCH_START_BLOCK not set, skipping epoch start block check"
    fi
}

# Check StakeTable references
[ -n "${ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS:-}" ] && {
    ST_TOKEN=$(cast call "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "token()(address)" --rpc-url "$RPC_URL")
    ST_TOKEN_LOWER=$(echo "$ST_TOKEN" | tr '[:upper:]' '[:lower:]')
    ESP_TOKEN_ADDR_LOWER=$(echo "$ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS" | tr '[:upper:]' '[:lower:]')
    [ "$ST_TOKEN_LOWER" = "$ESP_TOKEN_ADDR_LOWER" ] && echo -e "${GREEN}✓${NC} StakeTable token: $ST_TOKEN" || echo -e "${RED}✗${NC} StakeTable token: $ST_TOKEN (expected $ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS)"
    
    ST_LIGHT_CLIENT=$(cast call "$ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS" "lightClient()(address)" --rpc-url "$RPC_URL")
    ST_LIGHT_CLIENT_LOWER=$(echo "$ST_LIGHT_CLIENT" | tr '[:upper:]' '[:lower:]')
    LC_ADDR_LOWER=$(echo "$ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS" | tr '[:upper:]' '[:lower:]')
    [ "$ST_LIGHT_CLIENT_LOWER" = "$LC_ADDR_LOWER" ] && echo -e "${GREEN}✓${NC} StakeTable light client: $ST_LIGHT_CLIENT" || echo -e "${RED}✗${NC} StakeTable light client: $ST_LIGHT_CLIENT (expected $ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS)"
}

