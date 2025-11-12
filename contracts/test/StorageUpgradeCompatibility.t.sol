// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

// Storage Layout Compatibility Tests
//
// These tests verify that proposed contract upgrades are storage-compatible with deployed
// contracts.
//
// Ideally, we would commit verified storage layouts to the repo for comparison. As an interim
// solution, these tests query Etherscan (via cast) and Sepolia RPC to fetch deployed contract
// metadata at test time. Because an `ETHERSCAN_API_KEY` is required these tests are excluded from
// `just contracts-test-forge`.
//
// Run `just contracts-test-network` to execute these tests.

import { Test } from "forge-std/Test.sol";

interface IVersioned {
    function getVersion() external view returns (uint8, uint8, uint8);
}

contract UpgradeTestHelper is Test {
    address sepoliaStakeTableProxy = 0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037;
    address sepoliaEspTokenProxy = 0xb3e655a030e2e34a18b72757b40be086a8F43f3b;

    /// Check storage layout compatibility between deployed proxy and all upgrade versions.
    /// Returns true if all versions are compatible, false otherwise.
    ///
    /// @param network Network name ("sepolia" or "mainnet")
    /// @param proxyAddress Address of the deployed proxy contract
    /// @param contractBaseName Base name of contract (e.g., "StakeTable", "EspToken")
    /// @param maxMajorVersion Highest major version to check (e.g., 2 for V2)
    /// @return compatible True if all versions are storage compatible
    function isStorageLayoutCompatible(
        string memory network,
        address proxyAddress,
        string memory contractBaseName,
        uint8 maxMajorVersion
    ) internal returns (bool compatible) {
        string memory rpcUrl;
        if (keccak256(bytes(network)) == keccak256(bytes("sepolia"))) {
            rpcUrl = "https://ethereum-sepolia-rpc.publicnode.com";
        } else if (keccak256(bytes(network)) == keccak256(bytes("mainnet"))) {
            rpcUrl = "https://ethereum-rpc.publicnode.com";
        } else {
            revert("Unsupported network");
        }

        vm.setEnv("RPC_URL", vm.envOr("RPC_URL", rpcUrl));
        vm.createSelectFork(rpcUrl);

        // Read implementation address from EIP-1967 slot
        bytes32 implSlot = 0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc;
        address impl = address(uint160(uint256(vm.load(proxyAddress, implSlot))));
        require(impl != address(0), "Invalid implementation address");

        // Get deployed version from proxy
        (uint8 majorVersion,,) = IVersioned(proxyAddress).getVersion();

        // Check storage layout for all versions from deployed to max
        for (uint8 v = majorVersion; v <= maxMajorVersion; v++) {
            string memory contractName;
            if (v == 1) {
                contractName = contractBaseName;
            } else {
                contractName = string.concat(contractBaseName, "V", vm.toString(v));
            }

            string[] memory cmds = new string[](4);
            cmds[0] = "node";
            cmds[1] = "contracts/test/script/compare-storage-layout-deployed.js";
            cmds[2] = vm.toString(impl);
            cmds[3] = contractName;

            bytes memory output = vm.ffi(cmds);
            string memory result = string(output);
            if (keccak256(bytes(result)) != keccak256(bytes("true"))) {
                return false;
            }
        }
        return true;
    }

    /// Verify storage layout compatibility between deployed proxy and all upgrade versions.
    /// Reverts if any version is incompatible.
    ///
    /// This function automatically detects the deployed implementation version from the proxy
    /// and tests compatibility with all versions from the deployed version up to maxMajorVersion.
    ///
    /// It works by:
    /// 1. Reading the implementation address from the proxy's EIP-1967 storage slot
    /// 2. Calling getVersion() on the proxy to determine the deployed version
    /// 3. Testing storage compatibility from deployed version through maxMajorVersion
    ///
    /// Example:
    ///   // Proxy has StakeTable V1 deployed, test upgrade path through V2
    ///   ensureStorageLayoutCompatible("sepolia", proxyAddr, "StakeTable", 2);
    ///   // This will check: StakeTable (V1) and StakeTableV2
    ///
    /// @param network Network name ("sepolia" or "mainnet")
    /// @param proxyAddress Address of the deployed proxy contract
    /// @param contractBaseName Base name of contract (e.g., "StakeTable", "EspToken")
    /// @param maxMajorVersion Highest major version to check (e.g., 2 for V2)
    function ensureStorageLayoutCompatible(
        string memory network,
        address proxyAddress,
        string memory contractBaseName,
        uint8 maxMajorVersion
    ) internal {
        require(
            isStorageLayoutCompatible(network, proxyAddress, contractBaseName, maxMajorVersion),
            "Storage layout incompatible"
        );
    }
}

contract NetworkStorageLayoutSanityTest is UpgradeTestHelper {
    function test_Network_StorageLayout_Sanity_IncompatibleMissingField() public {
        bool compatible = isStorageLayoutCompatible(
            "sepolia", sepoliaStakeTableProxy, "StakeTableMissingFieldTest", 1
        );
        assertFalse(compatible, "Missing field should be incompatible");
    }

    function test_Network_StorageLayout_Sanity_IncompatibleReorderedFields() public {
        bool compatible = isStorageLayoutCompatible(
            "sepolia", sepoliaStakeTableProxy, "StakeTableFieldsReorderedTest", 1
        );
        assertFalse(compatible, "Reordered fields should be incompatible");
    }
}

contract NetworkStorageLayoutTest is UpgradeTestHelper {
    function test_Network_StorageLayout_StakeTable_Sepolia() public {
        uint8 maxVersion = 2;
        ensureStorageLayoutCompatible("sepolia", sepoliaStakeTableProxy, "StakeTable", maxVersion);
    }

    function test_Network_StorageLayout_EspToken_Sepolia() public {
        uint8 maxVersion = 2;
        ensureStorageLayoutCompatible("sepolia", sepoliaEspTokenProxy, "EspToken", maxVersion);
    }
}
