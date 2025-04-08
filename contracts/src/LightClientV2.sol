// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.28;

import { BN254 } from "bn254/BN254.sol";
import { LightClient } from "./LightClient.sol";
import { IPlonkVerifier } from "./interfaces/IPlonkVerifier.sol";
import { PlonkVerifierV2 } from "./libraries/PlonkVerifierV2.sol";
import { LightClientStateUpdateVKV2 as VkLib } from "./libraries/LightClientStateUpdateVKV2.sol";

/// @notice LightClient V2: with stake table snapshot update during epoch change.
contract LightClientV2 is LightClient {
    /// @notice When entering a new epoch and a new stake table snapshot.
    event NewEpoch(uint64 epoch);

    /// @notice number of blocks per epoch
    uint64 public _blocksPerEpoch;
    /// @notice stake table commitments for the current voting stakers
    StakeTableState public votingStakeTableState;

    /// @notice The finalized state for the epoch root of every epoch should NOT be skipped
    error MissingEpochRootUpdate();
    /// @notice Invocation on outdated APIs on V1
    error DeprecatedApi();

    /// @notice Initialize V2
    function initializeV2(uint64 blocksPerEpoch) public reinitializer(2) {
        _initializeV2(blocksPerEpoch);
    }

    /// @dev Avoid initialization problem during testing, avoid always upgrading from V1
    function _initializeV2(uint64 blocksPerEpoch) internal {
        if (blocksPerEpoch == 0) {
            revert InvalidArgs();
        }
        votingStakeTableState = genesisStakeTableState;
        _blocksPerEpoch = blocksPerEpoch;
    }

    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (2, 0, 0);
    }

    /// @dev override the V1's to disable calling it
    function newFinalizedState(LightClientState memory, IPlonkVerifier.PlonkProof memory)
        external
        pure
        override
    {
        revert DeprecatedApi();
    }

    /// @notice identical as LightClientV1's `setstateHistoryRetentionPeriod()`
    /// but this function name has the correct camelCase
    function setStateHistoryRetentionPeriod(uint32 historySeconds) public virtual onlyOwner {
        setstateHistoryRetentionPeriod(historySeconds);
    }

    /// @dev See detailed doc in `LightClient.sol`
    /// @param nextStakeTable the stake table to use in the next block (same as the current except
    function newFinalizedState(
        LightClientState memory newState,
        StakeTableState memory nextStakeTable,
        IPlonkVerifier.PlonkProof memory proof
    ) external virtual {
        //revert if we're in permissionedProver mode and the permissioned prover has not been set
        if (isPermissionedProverEnabled() && msg.sender != permissionedProver) {
            revert ProverNotPermissioned();
        }

        if (
            newState.viewNum <= finalizedState.viewNum
                || newState.blockHeight <= finalizedState.blockHeight
        ) {
            revert OutdatedState();
        }
        // format validity check
        BN254.validateScalarField(newState.blockCommRoot);

        // epoch-related checks
        uint64 lastUpdateEpoch = currentEpoch();
        uint64 newEpoch = epochFromBlockNumber(newState.blockHeight, _blocksPerEpoch);
        // advancing 1 epoch is only allowed if the epoch root (last block - 5) of the last epoch
        // was submitted
        // or if there is no last epoch (i.e. when current epoch is epoch 1)
        require(newEpoch < lastUpdateEpoch + 2, MissingEpochRootUpdate());
        if (
            lastUpdateEpoch != 0
                && epochFromBlockNumber(newState.blockHeight + 5, _blocksPerEpoch) != lastUpdateEpoch
        ) {
            require(isEpochRoot(finalizedState.blockHeight), MissingEpochRootUpdate());
        }
        BN254.validateScalarField(nextStakeTable.blsKeyComm);
        BN254.validateScalarField(nextStakeTable.schnorrKeyComm);
        BN254.validateScalarField(nextStakeTable.amountComm);

        // check plonk proof
        verifyProof(newState, nextStakeTable, proof);

        // upon successful verification, update the latest finalized state
        // during epoch change, also update to the new stake table
        finalizedState = newState;
        if (isEpochRoot(newState.blockHeight)) {
            votingStakeTableState = nextStakeTable;
            emit NewEpoch(newEpoch + 1);
        }

        updateStateHistory(uint64(currentBlockNumber()), uint64(block.timestamp), newState);

        emit NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
    }

    function _getVk()
        public
        pure
        virtual
        override
        returns (IPlonkVerifier.VerifyingKey memory vk)
    {
        vk = VkLib.getVk();
    }

    /// @dev compare to V1, we extend public input length from 7 to 11, use a newly generated VK,
    /// and enforce correct usage of the nextStakeTable outside the epoch change period.
    function verifyProof(
        LightClientState memory state,
        StakeTableState memory nextStakeTable,
        IPlonkVerifier.PlonkProof memory proof
    ) internal virtual {
        IPlonkVerifier.VerifyingKey memory vk = _getVk();

        // Prepare the public input
        uint256[11] memory publicInput;
        publicInput[0] = uint256(state.viewNum);
        publicInput[1] = uint256(state.blockHeight);
        publicInput[2] = BN254.ScalarField.unwrap(state.blockCommRoot);
        publicInput[3] = BN254.ScalarField.unwrap(votingStakeTableState.blsKeyComm);
        publicInput[4] = BN254.ScalarField.unwrap(votingStakeTableState.schnorrKeyComm);
        publicInput[5] = BN254.ScalarField.unwrap(votingStakeTableState.amountComm);
        publicInput[6] = votingStakeTableState.threshold;

        if (isEpochRoot(state.blockHeight)) {
            // during epoch change: use the next stake table
            publicInput[7] = BN254.ScalarField.unwrap(nextStakeTable.blsKeyComm);
            publicInput[8] = BN254.ScalarField.unwrap(nextStakeTable.schnorrKeyComm);
            publicInput[9] = BN254.ScalarField.unwrap(nextStakeTable.amountComm);
            publicInput[10] = nextStakeTable.threshold;
        } else {
            // use the previous stake table, effectively force nextStakeTable == votingStakeTable
            publicInput[7] = BN254.ScalarField.unwrap(votingStakeTableState.blsKeyComm);
            publicInput[8] = BN254.ScalarField.unwrap(votingStakeTableState.schnorrKeyComm);
            publicInput[9] = BN254.ScalarField.unwrap(votingStakeTableState.amountComm);
            publicInput[10] = votingStakeTableState.threshold;
        }

        // invoking PlonkVerifier2.sol::verify()
        if (!PlonkVerifierV2.verify(vk, publicInput, proof)) {
            revert InvalidProof();
        }
    }

    // === Epoch-related logic ===
    //

    /// @notice Returns the current epoch according the latest update on finalizedState
    /// @return current epoch (computed from the last known hotshot block number)
    function currentEpoch() public view virtual returns (uint64) {
        return epochFromBlockNumber(finalizedState.blockHeight, _blocksPerEpoch);
    }

    /// @notice Calculate the epoch number from the hotshot block number
    /// @dev same logic as `hotshot_types::utils::epoch_from_block_number()`
    function epochFromBlockNumber(uint64 blockNum, uint64 blocksPerEpoch)
        public
        pure
        virtual
        returns (uint64)
    {
        if (blocksPerEpoch == 0) {
            // this case is unreachable in our context since we reject zero-valued _blocksPerEpoch
            // at init time
            return 0;
        } else if (blockNum % blocksPerEpoch == 0) {
            return blockNum / blocksPerEpoch;
        } else {
            return blockNum / blocksPerEpoch + 1;
        }
    }

    /// @notice Decide if a block height is the an "epoch root" (defined as last block in epoch - 5)
    function isEpochRoot(uint64 blockHeight) public view virtual returns (bool) {
        if (blockHeight == 0) {
            return false;
        } else {
            return (blockHeight + 5) % _blocksPerEpoch == 0;
        }
    }
}
