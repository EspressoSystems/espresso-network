// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { IPlonkVerifier } from "../../src/interfaces/IPlonkVerifier.sol";
import { LightClient } from "../../src/LightClient.sol";
import { PlonkVerifier } from "../../src/libraries/PlonkVerifier.sol";
import { LightClientStateUpdateVK as VkLib } from "../../src/libraries/LightClientStateUpdateVK.sol";

/// @dev a Fake V2 for testing upgradability purposes only
contract LightClientV2Fake is LightClient {
    /// @notice new field for testing purposes
    /// @dev In order to add a field to LightClientState struct one can: add a new contract variable
    /// that has the new struct type, or put the struct inside a map.
    uint256 public newField;

    struct ExtendedLightClientState {
        uint64 viewNum;
        uint64 blockHeight;
        BN254.ScalarField blockCommRoot;
        uint256 extraField;
    }

    /// @notice mapping to store the extended light client states in order to simplify upgrades
    ExtendedLightClientState public extendedFinalizedState;

    /// @notice Initialize v2
    /// @param _newField   New field amount
    /// @dev the reinitializer modifier is used to reinitialize new versions of a contract and
    /// is called at most once. The modifier has an uint64 argument which indicates the next
    /// contract version.
    /// when the base implementation contract is initialized for the first time, the _initialized
    /// version
    /// is set to 1. Since this is the 2nd implementation, the next contract version is 2.
    function initializeV2(uint256 _newField, uint256 _extraField) external reinitializer(2) {
        newField = _newField;
        extendedFinalizedState.viewNum = finalizedState.viewNum;
        extendedFinalizedState.blockHeight = finalizedState.blockHeight;
        extendedFinalizedState.blockCommRoot = finalizedState.blockCommRoot;
        extendedFinalizedState.extraField = _extraField;
    }

    /// @notice Use this to get the implementation contract version
    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (2, 0, 0);
    }

    // === State Modifying APIs ===
    //
    /// @notice Update the latest finalized light client state. It is expected to be updated
    /// periodically, especially an update for the last block for every epoch has to be submitted
    /// before any newer state can be accepted since the stake table commitments of that block
    /// become the snapshots used for vote verifications later on.
    function newFinalizedState(
        ExtendedLightClientState memory newState,
        IPlonkVerifier.PlonkProof memory proof
    ) external virtual {
        if (
            newState.viewNum <= finalizedState.viewNum
                || newState.blockHeight <= finalizedState.blockHeight
        ) {
            revert OutdatedState();
        }
        // format validity check
        BN254.validateScalarField(newState.blockCommRoot);

        // check plonk proof
        verifyProof(newState, proof);

        // New condition to check w.r.t. LightClient contract V1
        require(newField == 0, "newField can only be set to 0");

        // upon successful verification, update the latest finalized state
        // because newState is in memory and states[finalizedState] is in storage, they have
        // different data handling mechanisms
        // and this each field needs to be assigned individually

        extendedFinalizedState = newState;

        emit NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
    }

    /// @notice Verify the Plonk proof, marked as `virtual` for easier testing as we can swap VK
    /// used in inherited contracts.
    function verifyProof(
        ExtendedLightClientState memory state,
        IPlonkVerifier.PlonkProof memory proof
    ) internal virtual {
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

    function setNewField(uint256 _newField) public onlyOwner {
        newField = _newField;
    }
}
