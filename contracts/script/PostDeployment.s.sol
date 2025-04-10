// SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.0;

import "forge-std/Script.sol";
import "../src/EspToken.sol";
import "../src/StakeTable.sol";

/// @notice This script is used to verify the post-deployment state of the EspToken contract.
/// @dev how to run this script:
/// forge script contracts/script/PostDeployment.s.sol:PostDeploymentEsp \
///     --rpc-url $RPC_URL \
///     --sig "run(address,address,uint256,address,string,string)" \
///     $ESP_TOKEN_PROXY_ADDRESS \
///     $ESP_TOKEN_OWNER \
///     $TOTAL_SUPPLY \
///     $ESP_TOKEN_INITIAL_GRANT_RECIPIENT_ADDRESS \
///     $TOKEN_NAME \
///     $TOKEN_SYMBOL
contract PostDeploymentEsp is Script {
    error InvalidName(string expected, string actual);
    error InvalidSymbol(string expected, string actual);
    error InvalidOwner(address expected, address actual);
    error InvalidTotalSupply(uint256 expected, uint256 actual);
    error InvalidInitialGrantRecipientBalance(uint256 expected, uint256 actual);
    error InvalidVersion();

    function run(
        address proxyAddress,
        address owner,
        uint256 totalSupply,
        address initialGrantRecipient,
        string memory name,
        string memory symbol
    ) external view {
        EspToken token = EspToken(proxyAddress);

        if (keccak256(bytes(token.name())) != keccak256(bytes(name))) {
            revert InvalidName(name, token.name());
        }
        if (keccak256(bytes(token.symbol())) != keccak256(bytes(symbol))) {
            revert InvalidSymbol(symbol, token.symbol());
        }
        if (token.owner() != owner) {
            revert InvalidOwner(owner, token.owner());
        }
        if (token.totalSupply() != totalSupply) {
            revert InvalidTotalSupply(totalSupply, token.totalSupply());
        }
        if (token.balanceOf(initialGrantRecipient) != totalSupply) {
            revert InvalidInitialGrantRecipientBalance(
                totalSupply, token.balanceOf(initialGrantRecipient)
            );
        }
        (uint8 major, uint8 minor, uint8 patch) = token.getVersion();
        if (major != 1 || minor != 0 || patch != 0) {
            revert InvalidVersion();
        }
    }
}

/// @notice This script is used to verify the post-deployment state of the StakeTable contract.
/// @dev how to run this script:
/// forge script contracts/script/PostDeployment.s.sol:PostDeploymentStakeTable \
///     --rpc-url $RPC_URL \
///     --sig "run(address,address,address,address,uint256)" \
///     $STAKE_TABLE_PROXY_ADDRESS \
///     $ESP_TOKEN_PROXY_ADDRESS \
///     $STAKE_TABLE_OWNER \
///     $LIGHT_CLIENT_ADDRESS \
///     $EXIT_ESCROW_PERIOD
///     TODO verify that the token and light client addresses are proxies
contract PostDeploymentStakeTable is Script {
    error InvalidOwner(address expected, address actual);
    error InvalidTokenProxyAddress(address expected, address actual);
    error InvalidStakeTableBalance(uint256 expected, uint256 actual);
    error InvalidLightClientAddress(address expected, address actual);
    error InvalidExitEscrowPeriod(uint256 expected, uint256 actual);
    error InvalidVersion();

    function run(
        address proxyAddress,
        address tokenProxyAddress,
        address owner,
        address lightClientAddress,
        uint256 exitEscrowPeriod
    ) external view {
        StakeTable stakeTable = StakeTable(proxyAddress);
        EspToken token = EspToken(tokenProxyAddress);

        if (stakeTable.owner() != owner) {
            revert InvalidOwner(owner, stakeTable.owner());
        }
        if (address(stakeTable.token()) != tokenProxyAddress) {
            revert InvalidTokenProxyAddress(tokenProxyAddress, address(stakeTable.token()));
        }
        if (token.balanceOf(proxyAddress) != 0) {
            revert InvalidStakeTableBalance(0, token.balanceOf(proxyAddress));
        }
        if (address(stakeTable.lightClient()) != lightClientAddress) {
            revert InvalidLightClientAddress(lightClientAddress, address(stakeTable.lightClient()));
        }
        if (stakeTable.exitEscrowPeriod() != exitEscrowPeriod) {
            revert InvalidExitEscrowPeriod(exitEscrowPeriod, stakeTable.exitEscrowPeriod());
        }
        (uint8 major, uint8 minor, uint8 patch) = token.getVersion();
        if (major != 1 || minor != 0 || patch != 0) {
            revert InvalidVersion();
        }
    }
}

interface IGnosisSafe {
    function getOwners() external view returns (address[] memory);
    function getThreshold() external view returns (uint256);
}

/// @notice Use this script to check if an address is a Gnosis Safe.
/// @notice It expects the safe to have at least 2 owners and a threshold of at least 2.
/// @dev how to run this script:
/// forge script contracts/script/PostDeployment.s.sol:GnosisSafeCheck \
///     --rpc-url $RPC_URL \
///     --sig "run(address)" \
///     $SAFE_ADDRESS
contract GnosisSafeCheck is Script {
    error InvalidOwnersLength(uint256 length);
    error InvalidThreshold(uint256 threshold);

    function run(address safeAddress) external view {
        IGnosisSafe safe = IGnosisSafe(safeAddress);
        address[] memory owners = safe.getOwners();
        uint256 threshold = safe.getThreshold();
        if (owners.length < 2) {
            revert InvalidOwnersLength(owners.length);
        }
        if (threshold < 2) {
            revert InvalidThreshold(threshold);
        }
    }
}
