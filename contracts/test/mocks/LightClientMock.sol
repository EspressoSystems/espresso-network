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

    /// @dev Directly mutate finalizedState variable for test
    function setFinalizedState(LC.LightClientState memory state) public {
        finalizedState = state;
    }

    /// @dev override the production-implementation with test VK.
    function _getVk() public pure override returns (IPlonkVerifier.VerifyingKey memory vk) {
        vk = VkLib.getVk();
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
