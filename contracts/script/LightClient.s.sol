pragma solidity ^0.8.20;

import { Script } from "forge-std/Script.sol";
import { Upgrades, Options } from "openzeppelin-foundry-upgrades/Upgrades.sol";
import { LightClient as LC } from "../src/LightClient.sol";
import { LightClientV2 as LCV2 } from "../src/LightClientV2.sol";
/// @notice Deploy the upgradeable light client contract using the OpenZeppelin Upgrades plugin.

contract DeployLightClientScript is Script {
    string public contractName = "LightClient.sol:LightClient";

    // Deployment Errors
    error SetPermissionedProverFailed();
    error OwnerTransferFailed();
    error RetentionPeriodIsNotSetCorrectly();

    /// @dev Deploys both the proxy and the implementation contract.
    /// The proxy admin is set as the owner of the contract upon deployment.
    /// The `owner` parameter should be the address of the multisig wallet to ensure proper
    /// ownership management.
    /// @param numInitValidators number of the validators initially
    /// @param stateHistoryRetentionPeriod state history retention period in seconds
    /// @param owner The address that will be set as the owner of the proxy (typically a multisig
    /// wallet).
    function run(uint32 numInitValidators, uint32 stateHistoryRetentionPeriod, address owner)
        public
        returns (
            address proxyAddress,
            address implementationAddress,
            LC.LightClientState memory lightClientState
        )
    {
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("DEPLOYER_MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("DEPLOYER_MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = vm.toString(uint256(numInitValidators));

        bytes memory result = vm.ffi(cmds);
        (LC.LightClientState memory state, LC.StakeTableState memory stakeState) =
            abi.decode(result, (LC.LightClientState, LC.StakeTableState));

        proxyAddress = Upgrades.deployUUPSProxy(
            contractName,
            abi.encodeCall(
                LC.initialize, (state, stakeState, stateHistoryRetentionPeriod, deployer)
            )
        );

        LC lightClientProxy = LC(proxyAddress);

        // Currently, the light client is in prover mode so set the permissioned prover
        address permissionedProver = vm.envAddress("PERMISSIONED_PROVER_ADDRESS");
        lightClientProxy.setPermissionedProver(permissionedProver);

        // transfer ownership to the multisig
        lightClientProxy.transferOwnership(owner);

        // verify post deployment details
        if (lightClientProxy.permissionedProver() != owner) revert SetPermissionedProverFailed();
        if (lightClientProxy.owner() != owner) revert OwnerTransferFailed();
        if (lightClientProxy.stateHistoryRetentionPeriod() != stateHistoryRetentionPeriod) {
            revert RetentionPeriodIsNotSetCorrectly();
        }

        // Get the implementation address
        implementationAddress = Upgrades.getImplementationAddress(proxyAddress);

        vm.stopBroadcast();

        return (proxyAddress, implementationAddress, state);
    }
}

/// @notice Upgrades the light client contract first by deploying the new implementation
/// and then executing the upgrade via the Safe Multisig wallet using the SAFE SDK.
contract LightClientContractUpgradeToV2Script is Script {
    string internal originalContractName = "LightClient.sol:LightClient";
    string internal upgradeContractName = "LightClientV2.sol:LightClientV2";

    /// @dev First the new implementation contract is deployed via the deployer wallet.
    /// It then uses the SAFE SDK via an ffi command to perform the upgrade through a Safe Multisig
    /// wallet.
    function run() public returns (address implementationAddress, bytes memory result) {
        Options memory opts;
        opts.referenceContract = originalContractName;

        // validate that the new implementation contract is upgrade safe
        // Upgrades.validateUpgrade(upgradeContractName, opts);

        // get the deployer to depley the new implementation contract
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("DEPLOYER_MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("DEPLOYER_MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);

        // deploy the new implementation contract
        LCV2 implementationContract = new LCV2();

        vm.stopBroadcast();

        bytes memory initData = abi.encodeWithSignature(
            "initializeV2(uint64,uint64)",
            vm.envUint("BLOCKS_PER_EPOCH"),
            vm.envUint("EPOCH_START_BLOCK")
        );
        // call upgradeToAndCall command so that the proxy can be upgraded to call from the new
        // implementation above and
        // execute the command via the Safe Multisig wallet
        string[] memory cmds = new string[](3);
        cmds[0] = "bash";
        cmds[1] = "-c";
        cmds[2] = string(
            abi.encodePacked(
                "source .env.contracts && ts-node contracts/script/multisigTransactionProposals/safeSDK/upgradeProxy.ts upgradeProxy ",
                vm.toString(vm.envAddress("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS")),
                " ",
                vm.toString(address(implementationContract)),
                " ",
                vm.toString(initData)
            )
        );

        result = vm.ffi(cmds);
        return (address(implementationContract), result);
    }
}

/// @notice Upgrades the light client contract first by deploying the new implementation
/// and then executing the upgrade via the Safe Multisig wallet using the SAFE SDK.
contract LightClientContractUpgradeToSameVersionScript is Script {
    /// @dev First the new implementation contract is deployed via the deployer wallet.
    /// It then uses the SAFE SDK via an ffi command to perform the upgrade through a Safe Multisig
    /// wallet.
    function run() public returns (address implementationAddress, bytes memory result) {
        // get the deployer to deploy the new implementation contract
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("DEPLOYER_MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("DEPLOYER_MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);

        // deploy the new implementation contract
        LC implementationContract = new LC();

        vm.stopBroadcast();

        // no initlaization needed for this upgrade as it's the same version
        bytes memory initData = "";

        // call upgradeToAndCall command so that the proxy can be upgraded to call from the new
        // implementation above and
        // execute the command via the Safe Multisig wallet
        string[] memory cmds = new string[](3);
        cmds[0] = "bash";
        cmds[1] = "-c";
        cmds[2] = string(
            abi.encodePacked(
                "source .env.contracts && ts-node contracts/script/multisigTransactionProposals/safeSDK/upgradeProxy.ts upgradeProxy ",
                vm.toString(vm.envAddress("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS")),
                " ",
                vm.toString(address(implementationContract)),
                " ",
                vm.toString(initData)
            )
        );

        result = vm.ffi(cmds);
        return (address(implementationContract), result);
    }
}

/// @notice Upgrades the light client contract first by deploying the new implementation
/// and then executing the upgrade via the Safe Multisig wallet using the SAFE SDK.
contract LightClientContractUpgradeToV2PatchScript is Script {
    /// @dev First the new implementation contract is deployed via the deployer wallet.
    /// It then uses the SAFE SDK via an ffi command to perform the upgrade through a Safe Multisig
    /// wallet.
    function run() public returns (address implementationAddress, bytes memory result) {
        // get the deployer to depley the new implementation contract
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("DEPLOYER_MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("DEPLOYER_MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);

        // deploy the new implementation contract
        LCV2 implementationContract = new LCV2();

        vm.stopBroadcast();

        // no initlaization needed for this patch, but a call to updateEpochStartBlock is needed
        bytes memory data = abi.encodeWithSignature(
            "updateEpochStartBlock(uint64)", vm.envUint("EPOCH_START_BLOCK")
        );
        // call upgradeToAndCall command so that the proxy can be upgraded to call from the new
        // implementation above and
        // execute the command via the Safe Multisig wallet
        string[] memory cmds = new string[](3);
        cmds[0] = "bash";
        cmds[1] = "-c";
        cmds[2] = string(
            abi.encodePacked(
                "source .env.contracts && ts-node contracts/script/multisigTransactionProposals/safeSDK/upgradeProxy.ts upgradeProxy ",
                vm.toString(vm.envAddress("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS")),
                " ",
                vm.toString(address(implementationContract)),
                " ",
                vm.toString(data)
            )
        );

        result = vm.ffi(cmds);
        return (address(implementationContract), result);
    }
}
