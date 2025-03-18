// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import "forge-std/Script.sol";

import { LightClientMock as LCMock } from "../mocks/LightClientMock.sol";
import { LightClient as LC } from "../../src/LightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { LightClientV2Fake as LCV2 } from "../mocks/LightClientV2Fake.sol";

contract DeployLightClientTestScript is Script {
    function run(uint64 numInitValidators, uint32 stateHistoryRetentionPeriod, address owner)
        external
        returns (address payable proxyAddress, address admin, LC.LightClientState memory)
    {
        // TODO for a production deployment provide the right genesis state and value

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = vm.toString(uint256(numInitValidators));

        bytes memory result = vm.ffi(cmds);
        (LC.LightClientState memory state, LC.StakeTableState memory stakeState) =
            abi.decode(result, (LC.LightClientState, LC.StakeTableState));

        return deployContract(state, stakeState, stateHistoryRetentionPeriod, owner);
    }

    function runBench(uint64 numInitValidators, uint32 stateHistoryRetentionPeriod)
        external
        returns (address payable, address, LC.LightClientState memory)
    {
        address payable lcTestProxy;
        address admin;
        LC.LightClientState memory state;
        string memory seedPhrase = vm.envString("MNEMONIC");
        (admin,) = deriveRememberKey(seedPhrase, 0);
        (lcTestProxy, admin, state) =
            this.run(numInitValidators, stateHistoryRetentionPeriod, admin);
        LCMock lc = LCMock(lcTestProxy);
        vm.prank(admin);
        lc.setPermissionedProver(admin);

        return (lcTestProxy, admin, state);
    }

    /// @notice deploys the impl, proxy & initializes the impl
    /// @return proxyAddress The address of the proxy
    /// @return admin The address of the admin
    /// @return the light client state
    function deployContract(
        LC.LightClientState memory state,
        LC.StakeTableState memory stakeState,
        uint32 stateHistoryRetentionPeriod,
        address owner
    ) public returns (address payable proxyAddress, address admin, LC.LightClientState memory) {
        vm.startBroadcast(owner);

        LCMock lightClientContract = new LCMock();

        // Encode the initializer function call
        bytes memory data = abi.encodeWithSignature(
            "initialize((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint32,address)",
            state,
            stakeState,
            stateHistoryRetentionPeriod,
            owner
        );

        // our proxy
        ERC1967Proxy proxy = new ERC1967Proxy(address(lightClientContract), data);
        vm.stopBroadcast();

        proxyAddress = payable(address(proxy));

        return (proxyAddress, owner, state);
    }
}

/// @notice Deploys the upgradable light client contract
/// the admin is not a multisig wallet but is the same as the associated mnemonic
/// used in staging deployments only
contract DeployLightClientContractWithoutMultiSigScript is Script {
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
        (LC.LightClientState memory state, LC.StakeTableState memory stakeTableState) =
            abi.decode(result, (LC.LightClientState, LC.StakeTableState));

        return deployContract(state, stakeTableState, stateHistoryRetentionPeriod);
    }

    /// @notice deploys the impl, proxy & initializes the impl
    /// @return proxyAddress The address of the proxy
    /// @return admin The address of the admin

    function deployContract(
        LC.LightClientState memory state,
        LC.StakeTableState memory stakeTableState,
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
            stakeTableState,
            stateHistoryRetentionPeriod,
            admin
        );

        // our proxy
        ERC1967Proxy proxy = new ERC1967Proxy(address(lightClientContract), data);
        vm.stopBroadcast();

        proxyAddress = payable(address(proxy));

        return (proxyAddress, admin, state, stakeTableState);
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
        string memory seedPhrase = vm.envString("MNEMONIC");
        uint32 seedPhraseOffset = uint32(vm.envUint("MNEMONIC_OFFSET"));
        (address admin,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        vm.startBroadcast(admin);

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

/// @notice Upgrades the light client contract first by deploying the new implementation
/// and then calling the upgradeToAndCall method of the proxy
/// @dev This is used when the admin is not a multisig wallet
/// used in staging deployments only
contract UpgradeToSameLightClientWithoutMultisigAdminScript is Script {
    /// @notice runs the upgrade
    /// @param mostRecentlyDeployedProxy address of deployed proxy
    /// @return address of the proxy
    /// TODO get the most recent deployment from the devops tooling
    function run(address mostRecentlyDeployedProxy) external returns (address) {
        // get the deployer info from the environment and start broadcast as the deployer
        string memory seedPhrase = vm.envString("MNEMONIC");
        uint32 seedPhraseOffset = uint32(vm.envUint("MNEMONIC_OFFSET"));
        (address admin,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        vm.startBroadcast(admin);

        address proxy = upgradeLightClient(mostRecentlyDeployedProxy, address(new LC()));
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
