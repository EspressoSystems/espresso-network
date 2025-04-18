pragma solidity ^0.8.0;

import { Script } from "forge-std/Script.sol";
import { LightClientV2 } from "../src/LightClientV2.sol";
import { LightClient as LC } from "../src/LightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
/// @notice Deploys the upgradable light client contract
/// the admin is not a multisig wallet but is the same as the associated mnemonic
/// used in staging deployments only

/// TODO we made these scripts in a rush for deployment and they are not very well tested
/// we plan to consolidate everything in the rust deployment scripts
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
        bytes memory data = abi.encodeWithSignature(
            "initializeV2(uint64,uint64)",
            vm.envUint("BLOCKS_PER_EPOCH"),
            vm.envUint("EPOCH_START_BLOCK")
        );

        address proxy =
            upgradeLightClient(mostRecentlyDeployedProxy, address(new LightClientV2()), data);
        return proxy;
    }

    /// @notice upgrades the light client contract by calling the upgrade function the
    /// implementation contract via
    /// the proxy
    /// @param proxyAddress address of proxy
    /// @param newLightClient address of new implementation
    /// @param data data to be passed to the new implementation
    /// @return address of the proxy
    function upgradeLightClient(address proxyAddress, address newLightClient, bytes memory data)
        public
        returns (address)
    {
        LC proxy = LC(proxyAddress); //make the function call on the previous implementation
        proxy.upgradeToAndCall(newLightClient, data); //proxy address now points to the new
            // implementation
        vm.stopBroadcast();
        return address(proxy);
    }
}
