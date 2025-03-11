// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
import { IPlonkVerifier as V } from "../src/interfaces/IPlonkVerifier.sol";

// Target contract
import { LightClient as LC } from "../src/LightClient.sol";
import { LightClientCommonTest } from "./LightClientV2.t.sol";

contract LightClientBench is LightClientCommonTest {
    constructor() {
        init();
    }

    /// @dev for benchmarking purposes only
    function testCorrectUpdateBench() external {
        vm.pauseGasMetering();
        (
            LC.LightClientState memory newState,
            LC.StakeTableState memory nextStakeTable,
            V.PlonkProof memory newProof
        ) = genStateProof();

        vm.prank(prover);
        vm.resumeGasMetering();
        lc.newFinalizedState(newState, nextStakeTable, newProof);
    }
}
