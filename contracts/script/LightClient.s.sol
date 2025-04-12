pragma solidity ^0.8.20;

import { Script } from "forge-std/Script.sol";
import { Upgrades, Options } from "openzeppelin-foundry-upgrades/Upgrades.sol";
import { LightClient as LC } from "../src/LightClient.sol";
import { LightClientV2Fake as LCV2 } from "../test/mocks/LightClientV2Fake.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

/// @notice Deploy the upgradeable light client contract using the OpenZeppelin Upgrades plugin.
contract DeployLightClientScript is Script {
    string public contractName = vm.envString("LIGHT_CLIENT_CONTRACT_ORIGINAL_NAME");

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
contract LightClientContractUpgradeScript is Script {
    string internal originalContractName = vm.envString("LIGHT_CLIENT_CONTRACT_ORIGINAL_NAME");
    string internal upgradeContractName = vm.envString("LIGHT_CLIENT_CONTRACT_UPGRADE_NAME");

    /// @dev First the new implementation contract is deployed via the deployer wallet.
    /// It then uses the SAFE SDK via an ffi command to perform the upgrade through a Safe Multisig
    /// wallet.
    function run() public returns (address implementationAddress, bytes memory result) {
        Options memory opts;
        opts.referenceContract = originalContractName;

        // validate that the new implementation contract is upgrade safe
        Upgrades.validateUpgrade(upgradeContractName, opts);

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

        bytes memory initData = abi.encodeWithSignature("setNewField(uint256)", 2);

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
/// @dev this is used when upgrading to the same base contract file which is being actively modified
/// before mainnet
contract UpgradeLightClientContractWithSameContractScript is Script {
    string internal originalContractName = vm.envString("LIGHT_CLIENT_CONTRACT_ORIGINAL_NAME");
    string internal upgradeContractName = vm.envString("LIGHT_CLIENT_CONTRACT_UPGRADE_NAME");

    function run() public returns (address implementationAddress, bytes memory result) {
        Options memory opts;
        opts.referenceContract = originalContractName;

        // validate that the new implementation contract is upgrade safe
        Upgrades.validateUpgrade(upgradeContractName, opts);

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
        LC implementationContract = new LC();

        vm.stopBroadcast();

        bytes memory initData = abi.encodeWithSignature("setNewField(uint256)", 2);

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

/// @notice Deploys the upgradable light client contract
/// the admin is not a multisig wallet but is the same as the associated mnemonic
/// used in staging deployments only
contract DeployLightClientContractScript is Script {
    function run(uint32 numInitValidators, uint32 stateHistoryRetentionPeriod)
        external
        returns (
            address payable proxyAddress,
            address admin,
            LC.LightClientState memory,
            LC.StakeTableState memory
        )
    {
        // TODO for a production deployment provide the right genesis state and value

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = vm.toString(uint256(numInitValidators));

        bytes memory result = vm.ffi(cmds);
        (LC.LightClientState memory state, LC.StakeTableState memory stakeState) =
            abi.decode(result, (LC.LightClientState, LC.StakeTableState));

        return deployContract(state, stakeState, stateHistoryRetentionPeriod);
    }

    /// @notice deploys the impl, proxy & initializes the impl
    /// @return proxyAddress The address of the proxy
    /// @return admin The address of the admin

    function deployContract(
        LC.LightClientState memory state,
        LC.StakeTableState memory stakeState,
        uint32 stateHistoryRetentionPeriod
    )
        private
        returns (
            address payable proxyAddress,
            address admin,
            LC.LightClientState memory,
            LC.StakeTableState memory
        )
    {
        // get the deployer info from the environment and start broadcast as the deployer
        string memory seedPhrase = vm.envString("MNEMONIC");
        uint32 seedPhraseOffset = uint32(vm.envUint("MNEMONIC_OFFSET"));
        (admin,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        vm.startBroadcast(admin);

        LC lightClientContract = new LC();

        // Encode the initializer function call
        bytes memory data = abi.encodeWithSignature(
            "initialize((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint32,address)",
            state,
            stakeState,
            stateHistoryRetentionPeriod,
            admin
        );

        // our proxy
        ERC1967Proxy proxy = new ERC1967Proxy(address(lightClientContract), data);
        vm.stopBroadcast();

        proxyAddress = payable(address(proxy));

        return (proxyAddress, admin, state, stakeState);
    }
}

/// @notice Upgrades the light client contract first by deploying the new implementation
/// and then calling the upgradeToAndCall method of the proxy
/// @dev This is used when the admin is not a multisig wallet
/// used in staging deployments only
contract UpgradeLightClientWithoutMultisigAdminScript is Script {
    /// @notice runs the upgrade
    /// @param mostRecentlyDeployedProxy address of deployed proxy
    /// @return address of the proxy
    /// TODO get the most recent deployment from the devops tooling
    function run(address mostRecentlyDeployedProxy) external returns (address) {
        // get the deployer info from the environment and start broadcast as the deployer
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);

        address proxy = upgradeLightClient(mostRecentlyDeployedProxy, address(new LCV2()));
        return proxy;
    }

    /// @notice upgrades the light client contract by calling the upgrade function the
    /// implementation contract via
    /// the proxy
    /// @param proxyAddress address of proxy
    /// @param newLightClient address of new implementation
    /// @return address of the proxy
    function upgradeLightClient(address proxyAddress, address newLightClient)
        public
        returns (address)
    {
        LC proxy = LC(proxyAddress); //make the function call on the previous implementation
        proxy.upgradeToAndCall(newLightClient, ""); //proxy address now points to the new
            // implementation
        vm.stopBroadcast();
        return address(proxy);
    }
}
