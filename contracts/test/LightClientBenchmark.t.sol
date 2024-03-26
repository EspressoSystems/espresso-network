// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
import { IPlonkVerifier as V } from "../src/interfaces/IPlonkVerifier.sol";

// Target contract
import { LightClient as LC } from "../src/LightClient.sol";
import { LightClientTest as LCTest } from "./mocks/LightClientTest.sol";
import { LightClientCommonTest } from "./LightClient.t.sol";
import { BN254 } from "bn254/BN254.sol";

contract LightClient_newFinalizedState_Test is LightClientCommonTest {
    function setUp() public {
        init();
        lc.switchToOptimizedTranscript();
    }

    /// @dev for benchmarking purposes only
    function testCorrectUpdate() external {
        // Generating a few consecutive states and proofs
        string[] memory cmds = new string[](6);
        cmds[0] = "diff-test";
        cmds[1] = "mock-consecutive-finalized-states";
        cmds[2] = vm.toString(BLOCKS_PER_EPOCH_TEST);
        cmds[3] = vm.toString(STAKE_TABLE_CAPACITY / 2);
        cmds[4] = vm.toString(uint64(3));
        cmds[5] = vm.toString(uint64(3));

        bytes memory result = vm.ffi(cmds);
        (LC.LightClientState[] memory states, V.PlonkProof[] memory proofs) =
            abi.decode(result, (LC.LightClientState[], V.PlonkProof[]));
        vm.expectEmit(true, true, true, true);
        emit LC.NewState(states[0].viewNum, states[0].blockHeight, states[0].blockCommRoot);
        lc.newFinalizedState(states[0], proofs[0]);
    }
}
