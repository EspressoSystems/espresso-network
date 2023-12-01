// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { IPlonkVerifier } from "./interfaces/IPlonkVerifier.sol";
import { PlonkVerifier } from "./libraries/PlonkVerifier.sol";

// TODO: replace these, for now WIP and test only
import { VkTest } from "../test/mocks/Transfer1In2Out24DepthVk.sol";

/// @notice A light client for HotShot consensus. Keeping track of its finalized states in safe,
/// authenticated ways.
contract LightClient {
    // === Constants ===
    //
    /// @notice System parameter: number of blocks per epoch
    uint32 public immutable BLOCKS_PER_EPOCH;

    // === Storage ===
    //
    /// @notice genesis block commitment
    LightClientState public genesisState;
    /// @notice global storage of the finalized HotShot's light client state
    LightClientState public finalizedState;
    /// @notice current (finalized) epoch number
    uint64 public currentEpoch;
    /// @notice The commitment of the stake table used in current voting (i.e. snapshot at the start
    /// of last epoch)
    bytes32 public votingStakeTableCommitment;
    /// @notice The commitment of the stake table frozen for change (i.e. snapshot at the start of
    /// last epoch)
    bytes32 public frozenStakeTableCommitment;

    // === Data Structure ===
    //
    /// @notice The finalized HotShot state (as the digest of the entire HotShot state)
    /// @param viewNum The latest view number of the finalized HotShot chain
    /// @param blockHeight The block height of the latest finalized block
    /// @param blockCommRoot The merkle root of historical block commitments (BN254::ScalarField)
    /// @param feeLedgerComm The commitment to the fee ledger state (type: BN254::ScalarField)
    /// @param stakeTableBlsKeyComm The commitment to the BlsVerKey column of the stake table
    /// @param stakeTableSchnorrKeyComm The commitment to the SchnorrVerKey column of the table
    /// @param stakeTableAmountComm The commitment to the stake amount column of the stake table
    /// @param threshold The (stake-weighted) quorum threshold for a QC to be considered as valid
    struct LightClientState {
        uint64 viewNum;
        uint64 blockHeight;
        uint256 blockCommRoot;
        uint256 feeLedgerComm;
        uint256 stakeTableBlsKeyComm;
        uint256 stakeTableSchnorrKeyComm;
        uint256 stakeTableAmountComm;
        uint256 threshold;
    }

    /// @notice Event that a new finalized state has been successfully verified and updated
    event NewState(uint64 indexed viewNum, uint64 indexed blockHeight, uint256 blockCommRoot);

    /// @notice The state is outdated and older than currently known `finalizedState`
    error OutdatedState();
    /// @notice Warning that the last block of the current epoch is not yet submitted before newer
    /// states are proposed.
    error MissingLastBlockForCurrentEpoch(uint64 expectedBlockHeight);
    /// @notice Invalid user inputs: wrong format or non-sensible arguments
    error InvalidArgs();

    constructor(LightClientState memory genesis, uint32 numBlockPerEpoch) {
        if (genesis.viewNum != 0 || genesis.blockHeight != 0) {
            revert InvalidArgs();
        }

        genesisState = genesis;
        finalizedState = genesis;
        currentEpoch = 0;
        BLOCKS_PER_EPOCH = numBlockPerEpoch;
        // TODO: (alex) initialized stake table or at least store its contract address ref here
    }

    // === State Modifying APIs ===
    //
    /// @notice Update the latest finalized light client state. It is expected to be updated
    /// periodically, especially an update for the last block for every epoch has to be submitted
    /// before any newer state can be accepted since the stake table commitments of that block
    /// become the snapshots used for vote verifications later on.
    function newFinalizedState(
        LightClientState memory newState,
        IPlonkVerifier.PlonkProof memory proof
    ) external {
        if (
            newState.viewNum <= finalizedState.viewNum
                || newState.blockHeight <= finalizedState.blockHeight
        ) {
            revert OutdatedState();
        }
        uint64 epochEndingBlockHeight = (currentEpoch + 1) * BLOCKS_PER_EPOCH - 1;
        bool isNewEpoch = finalizedState.blockHeight == epochEndingBlockHeight;
        if (!isNewEpoch && newState.blockHeight > epochEndingBlockHeight) {
            revert MissingLastBlockForCurrentEpoch(epochEndingBlockHeight);
        }
        // format validity check
        BN254.validateScalarField(newState.blockCommRoot);
        BN254.validateScalarField(newState.feeLedgerComm);
        BN254.validateScalarField(newState.stakeTableBlsKeyComm);
        BN254.validateScalarField(newState.stakeTableSchnorrKeyComm);
        BN254.validateScalarField(newState.stakeTableAmountComm);

        // check plonk proof
        // TODO: (alex) replace the vk with the correct one
        IPlonkVerifier.VerifyingKey memory vk = VkTest.getVk();
        uint256[] memory publicInput = preparePublicInput(newState, isNewEpoch);
        PlonkVerifier.verify(vk, publicInput, proof, bytes(""));

        // upon successful verification, update state.
        // If the newState is in a new epoch, only then should we increment the `currentEpoch`, and
        // update the stake table. The `finalizedState` (before update) should have the
        // `epochEndingBlockHeight`
        if (isNewEpoch) {
            _advanceEpoch();
        }

        finalizedState = newState;
        emit NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
    }

    // === Pure or View-only APIs ===
    /// @dev Transform a state into an array of field elements, prepared as public inputs of the
    /// plonk proof verification
    /// @param isNewEpoch Indicate if the `state` is in the new epoch, thus won't reuse threshold
    /// from `finalizedState`
    function preparePublicInput(LightClientState memory state, bool isNewEpoch)
        internal
        view
        returns (uint256[] memory)
    {
        uint256[] memory publicInput = new uint256[](8);
        publicInput[0] = uint256(state.viewNum);
        publicInput[1] = uint256(state.blockHeight);
        publicInput[2] = state.blockCommRoot;
        publicInput[3] = state.feeLedgerComm;
        publicInput[4] = state.stakeTableBlsKeyComm;
        publicInput[5] = state.stakeTableSchnorrKeyComm;
        publicInput[6] = state.stakeTableAmountComm;
        if (isNewEpoch) {
            publicInput[7] = state.threshold;
        } else {
            publicInput[7] = finalizedState.threshold;
        }
        return publicInput;
    }

    /// @notice Advance to the next epoch (without any precondition check!)
    /// @dev This meant to be invoked only internally after appropriate precondition checks are done
    function _advanceEpoch() private {
        bytes32 newStakeTableComm = keccak256(
            abi.encodePacked(
                finalizedState.stakeTableBlsKeyComm,
                finalizedState.stakeTableSchnorrKeyComm,
                finalizedState.stakeTableAmountComm
            )
        );

        votingStakeTableCommitment = frozenStakeTableCommitment;
        frozenStakeTableCommitment = newStakeTableComm;
        currentEpoch += 1;
    }
}
