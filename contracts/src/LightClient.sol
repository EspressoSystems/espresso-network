// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { UUPSUpgradeable } from
    "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";

import { BN254 } from "bn254/BN254.sol";
import { IPlonkVerifier } from "./interfaces/IPlonkVerifier.sol";
import { PlonkVerifier } from "./libraries/PlonkVerifier.sol";
import { LightClientStateUpdateVK as VkLib } from "./libraries/LightClientStateUpdateVK.sol";

/// @title Light Client Contract
/// @notice This contract serves as an always-on client
/// that verifies HotShot's state (Espresso's consensus state) which can be used by
/// Rollup contracts on L1 (Ethereum).
/// This state is submitted by any state-prover with evidence which is
/// a SNARK proof that proves consensus.
/// This contract also keeps track of the current epoch.
/// For this version, the epoch is not used. <br>
/// The light client state primarily consists of:<br>
/// - the merkle root of finalized block commitments,<br>
/// - the fee ledger commitment and <br>
/// - the active stake table commitment<br>
/// @dev You can use this contract to keep track of its finalized states in safe,
/// authenticated ways.
contract LightClient is Initializable, OwnableUpgradeable, UUPSUpgradeable {
    // === Events ===
    //
    /// @notice upgrade event when the proxy updates the implementation it's pointing to
    event Upgrade(address implementation);

    /// @notice when a permissioned prover is set, this event is emitted.
    event PermissionedProverRequired(address permissionedProver);

    /// @notice when the permissioned prover is unset, this event is emitted.
    event PermissionedProverNotRequired();

    // === System Parameters ===
    //
    // === Storage ===
    //
    /// @notice genesis stake commitment
    StakeTableState public genesisStakeTableState;

    /// @notice genesis block commitment
    LightClientState public genesisState;

    /// @notice Finalized HotShot's light client state
    LightClientState public finalizedState;

    /// @notice the address of the prover that can call the newFinalizedState function when the
    /// contract is in permissioned prover mode. This address is address(0) when the contract is
    /// not in permissioned prover mode
    address public permissionedProver;

    /// @notice Max number of seconds worth of state commitments to record based on this block
    /// timestamp
    uint32 public stateHistoryRetentionPeriod;

    /// @notice index of first block in block state series
    ///@dev use this instead of index 0 since old states would be set to zero to keep storage costs
    /// constant to stateHistoryRetentionPeriod
    uint64 public stateHistoryFirstIndex;

    /// @notice an array to store the L1 block heights, HotShot Block Heights and their respective
    /// state history
    /// commitments
    StateHistoryCommitment[] public stateHistoryCommitments;

    // === Data Structure ===
    //
    /// @notice The finalized HotShot state (as the digest of the entire HotShot state)
    /// @param viewNum The latest view number of the finalized HotShot chain
    /// @param blockHeight The block height of the latest finalized block
    /// @param blockCommRoot The merkle root of historical block commitments (BN254::ScalarField)
    struct LightClientState {
        uint64 viewNum;
        uint64 blockHeight;
        BN254.ScalarField blockCommRoot;
    }

    /// @notice The finalized HotShot Stake state (as the digest of the entire HotShot state)
    /// @param threshold The (stake-weighted) quorum threshold for a QC to be considered as valid
    /// @param blsKeyComm The commitment to the BlsVerKey column of the stake table
    /// @param schnorrKeyComm The commitment to the SchnorrVerKey column of the table
    /// @param amountComm The commitment to the stake amount column of the stake table
    struct StakeTableState {
        uint256 threshold;
        BN254.ScalarField blsKeyComm;
        BN254.ScalarField schnorrKeyComm;
        BN254.ScalarField amountComm;
    }

    /// @notice Simplified HotShot commitment struct
    /// @param l1BlockHeight the block height of l1 when this state update was stored
    /// @param l1BlockTimestamp the block timestamp of l1 when this state update was stored
    /// @param hotShotBlockHeight The block height of the latest finalized HotShot block
    /// @param hotShotBlockCommRoot The merkle root of historical block commitments
    /// (BN254::ScalarField)
    struct StateHistoryCommitment {
        uint64 l1BlockHeight;
        uint64 l1BlockTimestamp;
        uint64 hotShotBlockHeight;
        BN254.ScalarField hotShotBlockCommRoot;
    }

    /// @notice Event that a new finalized state has been successfully verified and updated
    event NewState(
        uint64 indexed viewNum, uint64 indexed blockHeight, BN254.ScalarField blockCommRoot
    );

    /// @notice The state is outdated and older than currently known `finalizedState`
    error OutdatedState();
    /// @notice Invalid user inputs: wrong format or non-sensible arguments
    error InvalidArgs();
    /// @notice Wrong plonk proof or public inputs.
    error InvalidProof();
    /// @notice Wrong stake table used, should match `finalizedState`
    error WrongStakeTableUsed();
    /// @notice Invalid address
    error InvalidAddress();
    /// @notice Only a permissioned prover can perform this action
    error ProverNotPermissioned();
    /// @notice If the same mode or prover is sent to the function, then no change is required
    error NoChangeRequired();
    /// @notice Invalid L1 Block for checking Light Client Updates, premature or in the future
    error InsufficientSnapshotHistory();
    /// @notice Invalid HotShot Block for checking HotShot commitments, premature or in the future
    error InvalidHotShotBlockForCommitmentCheck();
    /// @notice Invalid Max Block States
    error InvalidMaxStateHistory();

    /// @notice Constructor disables initializers to prevent the implementation contract from being
    /// initialized
    /// @dev This is standard practice for OpenZeppelin upgradeable contracts. Storage is on the
    /// proxy contract
    /// since it calls this cnotract via delegatecall
    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    /// @notice This contract is called by the proxy when you deploy this contract
    /// @param _genesis The initial state of the light client
    /// @param _stateHistoryRetentionPeriod The maximum retention period (in seconds) for the state
    /// history. the min retention period allowed is 1 hour and max 365 days
    /// @param owner The address of the contract owner
    function initialize(
        LightClientState memory _genesis,
        StakeTableState memory _genesisStakeTableState,
        uint32 _stateHistoryRetentionPeriod,
        address owner
    ) public initializer {
        __Ownable_init(owner); //sets owner of the contract
        __UUPSUpgradeable_init();
        _initializeState(_genesis, _genesisStakeTableState, _stateHistoryRetentionPeriod);
    }

    /// @notice returns the current block number
    function currentBlockNumber() public view virtual returns (uint256) {
        return block.number;
    }

    /// @notice Use this to get the implementation contract version
    /// @return majorVersion The major version of the contract
    /// @return minorVersion The minor version of the contract
    /// @return patchVersion The patch version of the contract
    function getVersion()
        public
        pure
        virtual
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (1, 0, 0);
    }

    /// @notice only the owner can authorize an upgrade
    function _authorizeUpgrade(address newImplementation) internal virtual override onlyOwner {
        emit Upgrade(newImplementation);
    }

    /// @dev Initialization of contract variables happens in this method because the LightClient
    /// contract is upgradable and thus has its constructor method disabled.
    /// @param _genesis The initial state of the light client
    /// @param _genesisStakeTableState The initial stake table state of the light client
    /// @param _stateHistoryRetentionPeriod The maximum retention period (in seconds) for the state
    /// history. The min retention period allowed is 1 hour and the max is 365 days.
    function _initializeState(
        LightClientState memory _genesis,
        StakeTableState memory _genesisStakeTableState,
        uint32 _stateHistoryRetentionPeriod
    ) internal {
        // The viewNum and blockHeight in the genesis state must be zero to indicate that this is
        // the initial state. Stake table commitments and threshold cannot be zero, otherwise it's
        // impossible to generate valid proof to move finalized state forward. The
        // stateHistoryRetentionPeriod must be at least 1 hour and no more than 365 days
        // to ensure proper state retention.
        if (
            _genesis.viewNum != 0 || _genesis.blockHeight != 0
                || BN254.ScalarField.unwrap(_genesisStakeTableState.blsKeyComm) == 0
                || BN254.ScalarField.unwrap(_genesisStakeTableState.schnorrKeyComm) == 0
                || BN254.ScalarField.unwrap(_genesisStakeTableState.amountComm) == 0
                || _genesisStakeTableState.threshold == 0 || _stateHistoryRetentionPeriod < 1 hours
                || _stateHistoryRetentionPeriod > 365 days
        ) {
            revert InvalidArgs();
        }

        genesisState = _genesis;
        genesisStakeTableState = _genesisStakeTableState;
        finalizedState = _genesis;

        stateHistoryRetentionPeriod = _stateHistoryRetentionPeriod;
    }

    // === State Modifying APIs ===
    //
    /// @notice Update the latest finalized light client state. It must be updated
    /// periodically, especially an update for the last block for every
    /// period has to be submitted
    /// before any newer state can be accepted since the stake table commitments of that block
    /// become the snapshots used for vote verifications later on.
    /// @dev if the permissionedProver is set, only the permissionedProver can call this function
    /// @dev the state history for `stateHistoryRetentionPeriod` L1 blocks are also recorded in the
    /// `stateHistoryCommitments` array
    /// @notice While `newState.stakeTable*` refers to the (possibly) new stake table states,
    /// the entire `newState` needs to be signed by stakers in `finalizedState`
    /// @param newState new light client state
    /// @param proof PlonkProof
    function newFinalizedState(
        LightClientState memory newState,
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

        // check plonk proof
        verifyProof(newState, proof);

        // upon successful verification, update the latest finalized state
        finalizedState = newState;

        updateStateHistory(uint64(currentBlockNumber()), uint64(block.timestamp), newState);

        emit NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
    }

    /// @notice a technically unnecessary but luckily zero-cost indirection for the benefit of
    /// having
    /// `IPlonkVerifier.VerifyingKey` rust alloy bindings, included only if appear in a public func.
    function _getVk() public pure virtual returns (IPlonkVerifier.VerifyingKey memory vk) {
        vk = VkLib.getVk();
    }

    /// @notice Verify the Plonk proof, marked as `virtual` for easier testing as we can swap VK
    /// used in inherited contracts.
    function verifyProof(LightClientState memory state, IPlonkVerifier.PlonkProof memory proof)
        internal
        virtual
    {
        IPlonkVerifier.VerifyingKey memory vk = _getVk();

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

    /// @notice set the permissionedProver to the non-zero address provided
    /// @dev this function can also be used to update the permissioned prover once it's a different
    /// address to the current permissioned prover
    function setPermissionedProver(address prover) public virtual onlyOwner {
        if (prover == address(0)) {
            revert InvalidAddress();
        }
        if (prover == permissionedProver) {
            revert NoChangeRequired();
        }
        permissionedProver = prover;
        emit PermissionedProverRequired(permissionedProver);
    }

    /// @notice set the permissionedProver to address(0)
    /// @dev if it was already disabled, then revert with the error, NoChangeRequired
    function disablePermissionedProverMode() public virtual onlyOwner {
        if (isPermissionedProverEnabled()) {
            permissionedProver = address(0);
            emit PermissionedProverNotRequired();
        } else {
            revert NoChangeRequired();
        }
    }

    /// @notice Updates the `stateHistoryCommitments` array when a new finalized state is added
    /// and prunes the most outdated element starting from the first element if they fall outside
    /// the
    /// `stateHistoryRetentionPeriod`.
    /// @dev the block timestamp is used to determine if the stateHistoryCommitments array
    /// should be pruned, based on the stateHistoryRetentionPeriod (seconds).
    /// @dev A FIFO approach is used to remove the most outdated element from the start of the
    /// array.
    /// However, only one outdated element is removed per invocation of this function, even if
    /// multiple elements exceed the retention period. As a result, some outdated elements may
    /// remain in the array temporarily until subsequent invocations of this function.
    /// @dev the `delete` method does not reduce the array length but resets the value at the
    /// specified index to zero. the stateHistoryFirstIndex variable acts as an offset to indicate
    /// the starting point for reading the array, since the length of the array is not reduced
    /// even after deletion.
    /// @param blockNumber The block number of the new finalized state.
    /// @param blockTimestamp The block timestamp used to check the retention period.
    /// @param state The new `LightClientState` being added to the array.
    function updateStateHistory(
        uint64 blockNumber,
        uint64 blockTimestamp,
        LightClientState memory state
    ) internal {
        if (
            stateHistoryCommitments.length != 0
                && blockTimestamp - stateHistoryCommitments[stateHistoryFirstIndex].l1BlockTimestamp
                    > stateHistoryRetentionPeriod
        ) {
            // The stateHistoryCommitments array has reached the maximum retention period
            // delete the oldest (first) non-empty element to maintain the FIFO structure.
            delete stateHistoryCommitments[stateHistoryFirstIndex];

            // increment the offset to the first non-zero element in the stateHistoryCommitments
            // array
            stateHistoryFirstIndex++;
        }

        // add the L1 Block & HotShot commitment to the stateHistoryCommitments
        stateHistoryCommitments.push(
            StateHistoryCommitment(
                blockNumber, blockTimestamp, state.blockHeight, state.blockCommRoot
            )
        );
    }

    /// @notice checks if the state updates lag behind the specified block threshold based on the
    /// provided block number.
    /// @dev Reverts if there isn't enough state history to make an accurate comparison.
    /// Reverts if the blockThreshold is zero
    /// @param blockNumber The block number to compare against the latest state updates.
    /// @param blockThreshold The number of blocks updates this contract is allowed to lag behind.
    /// @return bool returns true if the lag exceeds the blockThreshold; otherwise, false.
    function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 blockThreshold)
        public
        view
        virtual
        returns (bool)
    {
        uint256 updatesCount = stateHistoryCommitments.length;

        // Edge Case Handling:
        // 1. Provided block number is greater than the current block (invalid)
        // 2. No updates have occurred (i.e., state history is empty)
        // 3. Provided block number is earlier than the first recorded state update
        // the stateHistoryFirstIndex is used to check for the first nonZero element
        if (
            blockNumber > currentBlockNumber() || updatesCount == 0
                || blockNumber < stateHistoryCommitments[stateHistoryFirstIndex].l1BlockHeight
        ) {
            revert InsufficientSnapshotHistory();
        }

        uint256 eligibleStateUpdateBlockNumber; // the eligibleStateUpdateBlockNumber is <=
        // blockNumber
        bool stateUpdateFound; // if an eligible block number is found in the state update history,
        // then this variable is set to true

        // Search from the most recent state update back to find the first update <= blockNumber
        uint256 i = updatesCount - 1;
        while (!stateUpdateFound) {
            // Stop searching if we've exhausted the recorded state history
            if (i < stateHistoryFirstIndex) {
                break;
            }

            // Find the first update with a block height <= blockNumber
            if (stateHistoryCommitments[i].l1BlockHeight <= blockNumber) {
                stateUpdateFound = true;
                eligibleStateUpdateBlockNumber = stateHistoryCommitments[i].l1BlockHeight;
                break;
            }

            i--;
        }

        // If no snapshot is found, we don't have enough history stored
        // to tell whether HotShot was down.
        if (!stateUpdateFound) {
            revert InsufficientSnapshotHistory();
        }

        return blockNumber - eligibleStateUpdateBlockNumber > blockThreshold;
    }

    /// @notice get the HotShot commitment that represents the Merkle root containing the leaf at
    /// the provided hotShotBlockHeight where the block height in the array is greater than
    //  the provided hotShotBlockHeight.
    /// @dev if the provided hotShotBlockHeight is greater than or equal to the latest commitment in
    /// the array,
    /// the function reverts.
    /// @param hotShotBlockHeight the HotShot block height
    /// @return hotShotBlockCommRoot the HotShot commitment root
    /// @return hotshotBlockHeight the HotShot block height for the corresponding commitment root
    function getHotShotCommitment(uint256 hotShotBlockHeight)
        public
        view
        virtual
        returns (BN254.ScalarField hotShotBlockCommRoot, uint64 hotshotBlockHeight)
    {
        uint256 commitmentsHeight = stateHistoryCommitments.length;
        if (hotShotBlockHeight >= stateHistoryCommitments[commitmentsHeight - 1].hotShotBlockHeight)
        {
            revert InvalidHotShotBlockForCommitmentCheck();
        }
        for (uint256 i = stateHistoryFirstIndex; i < commitmentsHeight; i++) {
            // Finds and returns the first HotShot commitment whose height is greater than the
            // requested hotshot height
            if (stateHistoryCommitments[i].hotShotBlockHeight > hotShotBlockHeight) {
                return (
                    stateHistoryCommitments[i].hotShotBlockCommRoot,
                    stateHistoryCommitments[i].hotShotBlockHeight
                );
            }
        }
    }

    /// @notice get the number of state history commitments
    /// @return uint256 The number of state history commitments
    function getStateHistoryCount() public view returns (uint256) {
        return stateHistoryCommitments.length;
    }

    /// @notice sets the maximum retention period for storing block state history.
    /// @param historySeconds The maximum number of seconds for which state history updates
    /// will be stored, based on the block timestamp. It must be greater than or equal to
    /// the current state history retention period and must be at least 1 hour and max 365 days.
    /// @dev Reverts with `InvalidMaxStateHistory` if the provided value is less than 1 hour,
    /// more than 365 days or less than or equal to the current state history retention period.
    function setstateHistoryRetentionPeriod(uint32 historySeconds) public onlyOwner {
        if (
            historySeconds < 1 hours || historySeconds > 365 days
                || historySeconds <= stateHistoryRetentionPeriod
        ) {
            revert InvalidMaxStateHistory();
        }

        stateHistoryRetentionPeriod = historySeconds;
    }

    /// @notice Check if permissioned prover is enabled
    function isPermissionedProverEnabled() public view returns (bool) {
        return (permissionedProver != address(0));
    }
}
