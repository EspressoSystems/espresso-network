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
    uint64 public blocksPerEpoch;
    /// @notice the block height when Epoch-related logic gets activated
    uint64 public epochStartBlock;
    /// @notice the first epoch where dynamic stake table is activated, not the numerical value
    /// epoch=1
    uint64 private firstEpoch;
    /// @notice stake table commitments for the current voting stakers
    StakeTableState public votingStakeTableState;

    /// @notice The finalized state for the epoch root of every epoch should NOT be skipped
    error MissingEpochRootUpdate();
    /// @notice Invocation on outdated APIs on V1
    error DeprecatedApi();

    /// @notice Initialize V2
    function initializeV2(uint64 _blocksPerEpoch, uint64 _epochStartBlock)
        public
        reinitializer(2)
    {
        // the transitioning period (from epoch root to last block) takes 5 blocks,
        // thus each epoch should definitely be longer than that.
        if (_blocksPerEpoch <= 5) {
            revert InvalidArgs();
        }
        votingStakeTableState = genesisStakeTableState;
        blocksPerEpoch = _blocksPerEpoch;
        epochStartBlock = _epochStartBlock;
        firstEpoch = epochFromBlockNumber(_epochStartBlock, _blocksPerEpoch);
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

    function updateEpochStartBlock(uint64 newEpochStartBlock) public virtual onlyOwner {
        epochStartBlock = newEpochStartBlock;
        firstEpoch = epochFromBlockNumber(epochStartBlock, blocksPerEpoch);
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
        BN254.validateScalarField(nextStakeTable.blsKeyComm);
        BN254.validateScalarField(nextStakeTable.schnorrKeyComm);
        BN254.validateScalarField(nextStakeTable.amountComm);

        // epoch-related checks
        uint64 lastUpdateEpoch = currentEpoch();
        uint64 newStateEpoch = epochFromBlockNumber(newState.blockHeight, blocksPerEpoch);
        // after epoch gets activated, for each epoch, we allow updates from
        // `start_block..=epoch_root` but disallow any transitioning blocks
        // `epoch_block+1..=last_block`, effectively
        // `epoch_root` is the "last block" of every epoch from light client's perspective.
        if (newStateEpoch >= firstEpoch) {
            require(!isGtEpochRoot(newState.blockHeight), MissingEpochRootUpdate());
        }
        if (newStateEpoch > firstEpoch) {
            // disallow skipping an epoch without an update
            require(newStateEpoch - lastUpdateEpoch < 2, MissingEpochRootUpdate());
            // advancing 1 epoch is only allowed if the epoch root (last block - 5) of the last
            // epoch was submitted
            if (newStateEpoch == lastUpdateEpoch + 1 && !isEpochRoot(finalizedState.blockHeight)) {
                revert MissingEpochRootUpdate();
            }
        }

        // check plonk proof
        verifyProof(newState, nextStakeTable, proof);

        // upon successful verification, update the latest finalized state
        finalizedState = newState;
        // during epoch change, also update to the new stake table
        // NOTE:
        // 1. only update the stake table after PoS activation (i.e. after `epochStartBlock`)
        // 2. towards the end of each epoch, only update next stake table in the epoch root
        // (lastBlockInEpoch - 5), the light client update for the last 4 blocks of every epoch
        // are skipped. We simply don't accept them. From LC contract's perspective, the stake
        // table has been updated, thus only accepting quorum signatures (thus SNARK proof) from
        // the new stakers.
        if (newStateEpoch >= firstEpoch && isEpochRoot(newState.blockHeight)) {
            votingStakeTableState = nextStakeTable;
            emit NewEpoch(newStateEpoch + 1);
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

        if (state.blockHeight >= epochStartBlock && isEpochRoot(state.blockHeight)) {
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
        return epochFromBlockNumber(finalizedState.blockHeight, blocksPerEpoch);
    }

    /// @notice Calculate the epoch number from the hotshot block number
    /// @dev same logic as `hotshot_types::utils::epoch_from_block_number()`
    function epochFromBlockNumber(uint64 _blockNum, uint64 _blocksPerEpoch)
        public
        pure
        virtual
        returns (uint64)
    {
        if (_blocksPerEpoch == 0) {
            // this case is unreachable in our context since we reject zero-valued blocksPerEpoch
            // at init time
            return 0;
        } else if (_blockNum == 0) {
            return 1;
        } else if (_blockNum % _blocksPerEpoch == 0) {
            return _blockNum / _blocksPerEpoch;
        } else {
            return _blockNum / _blocksPerEpoch + 1;
        }
    }

    /// @notice Decide if a block height is the an "epoch root" (defined as last block in epoch - 5)
    /// @dev see
    /// <https://github.com/EspressoSystems/espresso-network/blob/2a904fa17838961cef130d0e87d7b371acaaea42/hotshot-types/src/utils.rs#L475>
    function isEpochRoot(uint64 blockHeight) public view virtual returns (bool) {
        if (blockHeight == 0 || blocksPerEpoch == 0) {
            return false;
        } else {
            // it's safe to assume +5 won't overflow in practice
            return (blockHeight + 5) % blocksPerEpoch == 0;
        }
    }

    /// @notice Returns true if the given block number is greater than the epoch root block
    function isGtEpochRoot(uint64 blockHeight) public view virtual returns (bool) {
        if (blockHeight == 0 || blocksPerEpoch == 0) {
            return false;
        } else {
            // it's safe to assume -5 won't underflow in practice
            return blockHeight % blocksPerEpoch > blocksPerEpoch - 5;
        }
    }
}
