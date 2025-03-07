// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import { Script } from "forge-std/Script.sol";
import { LightClientArbitrum } from "../src/LightClientArbitrum.sol";
import { LightClient as LC } from "../src/LightClient.sol";
import { Upgrades } from "openzeppelin-foundry-upgrades/Upgrades.sol";

contract DeployLightClientArbitrum is Script {
    // Deployment Errors
    error SetPermissionedProverFailed();
    error OwnerTransferFailed();
    error RetentionPeriodIsNotSetCorrectly();
    error InitialStateIsNotSetCorrectly();

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
            "LightClientArbitrum.sol:LightClientArbitrum",
            abi.encodeCall(
                LC.initialize, (state, stakeState, stateHistoryRetentionPeriod, deployer)
            )
        );

        LightClientArbitrum lightClientArbitrumProxy = LightClientArbitrum(proxyAddress);

        // Currently, the light client is in prover mode so set the permissioned prover
        address permissionedProver = vm.envAddress("PERMISSIONED_PROVER_ADDRESS");
        lightClientArbitrumProxy.setPermissionedProver(permissionedProver);

        // transfer ownership to the multisig
        lightClientArbitrumProxy.transferOwnership(owner);

        // verify post deployment details
        if (lightClientArbitrumProxy.permissionedProver() != permissionedProver) {
            revert SetPermissionedProverFailed();
        }
        if (lightClientArbitrumProxy.owner() != owner) revert OwnerTransferFailed();
        if (lightClientArbitrumProxy.stateHistoryRetentionPeriod() != stateHistoryRetentionPeriod) {
            revert RetentionPeriodIsNotSetCorrectly();
        }
        if (lightClientArbitrumProxy.stateHistoryFirstIndex() != 0) {
            revert InitialStateIsNotSetCorrectly();
        }
        if (lightClientArbitrumProxy.getStateHistoryCount() != 0) {
            revert InitialStateIsNotSetCorrectly();
        }

        // Get the implementation address
        implementationAddress = Upgrades.getImplementationAddress(proxyAddress);

        vm.stopBroadcast();

        return (proxyAddress, implementationAddress, state);
    }
}
