// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { LightClient as LC } from "../../src/LightClient.sol";
import { IPlonkVerifier } from "../../src/interfaces/IPlonkVerifier.sol";
import { PlonkVerifier } from "../../src/libraries/PlonkVerifier.sol";
import { LightClientStateUpdateVKMock as VkLib } from "./LightClientStateUpdateVKMock.sol";

/// @dev A helper that wraps LightClient contract for testing
contract LightClientMock is LC {
    bool internal hotShotDown;
    uint256 internal frozenL1Height;

    constructor(
        LC.LightClientState memory genesis,
        LC.StakeTableState memory genesisStakeTableState,
        uint32 maxHistorySeconds
    ) LC() {
        _initializeState(genesis, genesisStakeTableState, maxHistorySeconds);
    }

    /// @dev Directly mutate finalizedState variable for test
    function setFinalizedState(LC.LightClientState memory state) public {
        finalizedState = state;
    }

    /// @dev override the production-implementation with test VK.
    function verifyProof(LC.LightClientState memory state, IPlonkVerifier.PlonkProof memory proof)
        internal
        view
        override
    {
        IPlonkVerifier.VerifyingKey memory vk = VkLib.getVk();

        // Prepare the public input
        uint256[7] memory publicInput;
        publicInput[0] = uint256(state.viewNum);
        publicInput[1] = uint256(state.blockHeight);
        publicInput[2] = BN254.ScalarField.unwrap(state.blockCommRoot);
        publicInput[3] = BN254.ScalarField.unwrap(genesisStakeTableState.blsKeyComm);
        publicInput[4] = BN254.ScalarField.unwrap(genesisStakeTableState.schnorrKeyComm);
        publicInput[5] = BN254.ScalarField.unwrap(genesisStakeTableState.amountComm);
        publicInput[6] = genesisStakeTableState.threshold;

        if (!PlonkVerifier.verify(vk, publicInput, proof)) {
            revert InvalidProof();
        }
    }

    function setStateHistory(StateHistoryCommitment[] memory _stateHistoryCommitments) public {
        // delete the previous stateHistoryCommitments
        delete stateHistoryCommitments;

        // Set the stateHistoryCommitments to the new values
        for (uint256 i = 0; i < _stateHistoryCommitments.length; i++) {
            stateHistoryCommitments.push(_stateHistoryCommitments[i]);
        }
    }

    function setHotShotDownSince(uint256 l1Height) public {
        hotShotDown = true;
        frozenL1Height = l1Height;
    }

    function setHotShotUp() public {
        hotShotDown = false;
    }

    /// @dev override the production-implementation with frozen data
    function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 threshold)
        public
        view
        override
        returns (bool)
    {
        return hotShotDown
            ? blockNumber - frozenL1Height > threshold
            : super.lagOverEscapeHatchThreshold(blockNumber, threshold);
    }
}
