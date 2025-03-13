// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
import { IPlonkVerifier as V } from "../src/interfaces/IPlonkVerifier.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";

// Target contract
import { LightClient as LC } from "../src/LightClient.sol";
import { PlonkVerifierV2 as PV2 } from "../src/libraries/PlonkVerifierV2.sol";
import { LightClientV2Mock as LCV2 } from "./mocks/LightClientV2Mock.sol";
import { BN254 } from "bn254/BN254.sol";

/// @dev Common helpers for LightClient tests
contract LightClientCommonTest is Test {
    LCV2 public lc;
    LC.LightClientState public genesis;
    LC.StakeTableState public genesisStakeTableState;
    uint32 public constant MAX_HISTORY_SECONDS = 1 days;
    // this constant should be consistent with `hotshot_contract::light_client.rs`
    uint64 internal constant STAKE_TABLE_CAPACITY = 10;
    address payable public proxyAddr;
    address public admin = makeAddr("admin");
    address public prover = makeAddr("prover");
    uint64 public constant BLOCKS_PER_EPOCH = 3;

    /// @dev initialized ledger like genesis and system params
    function init() public {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = vm.toString(STAKE_TABLE_CAPACITY / 2);

        bytes memory result = vm.ffi(cmds);
        (LC.LightClientState memory state, LC.StakeTableState memory stakeState) =
            abi.decode(result, (LC.LightClientState, LC.StakeTableState));
        genesis = state;
        genesisStakeTableState = stakeState;

        // Now, we
        // - deploy LCV1
        // - deploy proxy while setting current impl to LCV1 and initialize
        // - set permissioned prover on the LC proxy
        // - deploy LCV2
        // - upgrade on the proxy to point to new impl as LCV2 and initialize
        LC lcv1 = new LC();
        bytes memory lcv1InitData = abi.encodeWithSignature(
            "initialize((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint32,address)",
            genesis,
            genesisStakeTableState,
            MAX_HISTORY_SECONDS,
            admin
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(lcv1), lcv1InitData);
        proxyAddr = payable(address(proxy));

        // set permissioned flag
        vm.expectEmit(true, true, true, true);
        emit LC.PermissionedProverRequired(prover);
        vm.prank(admin);
        LC(proxyAddr).setPermissionedProver(prover);

        // deploy PlonkVerifierV2 and LCV2
        LCV2 lcv2 = new LCV2();
        PV2 pv2 = new PV2();
        bytes memory lcv2InitData =
            abi.encodeWithSignature("initializeV2(uint64,address)", BLOCKS_PER_EPOCH, address(pv2));

        // upgrade proxy to new LC impl and initialize LCV2
        vm.prank(admin);
        LC(proxyAddr).upgradeToAndCall(address(lcv2), lcv2InitData);
        // now the proxy can be treated as LCV2
        lc = LCV2(proxyAddr);
    }

    function assertEq(BN254.ScalarField a, BN254.ScalarField b) public pure {
        assertEq(BN254.ScalarField.unwrap(a), BN254.ScalarField.unwrap(b));
    }

    /// @dev util to generate a single valid state proof and public inputs (for
    /// `newFinalizedState()` update)
    function genStateProof()
        public
        returns (
            LC.LightClientState memory newState,
            LC.StakeTableState memory nextStakeTable,
            V.PlonkProof memory newProof
        )
    {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-skip-blocks";
        cmds[2] = vm.toString(uint32(1));

        bytes memory result = vm.ffi(cmds);
        (newState, nextStakeTable, newProof) =
            abi.decode(result, (LC.LightClientState, LC.StakeTableState, V.PlonkProof));
    }

    /// @dev return a simulated new state that advance from current `finalizedState`
    function simulateNewState(uint64 elpasedBlock)
        internal
        returns (LC.LightClientState memory state)
    {
        vm.roll(elpasedBlock / 2 + block.number); // L1 moves twice as slow
        (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot) = lc.finalizedState();
        // blockCommRoots are just random different values from the latest state
        state = LC.LightClientState(
            viewNum + elpasedBlock,
            blockHeight + elpasedBlock,
            BN254.ScalarField.wrap(BN254.ScalarField.unwrap(blockCommRoot) + 50)
        );
    }
}

contract LightClient_constructor_Test is LightClientCommonTest {
    function setUp() public {
        init();
    }

    // NOTE: `contract LightClient_constructor_Test` for LightClient.sol (V1) is dropped since it's
    // already deployed, and LCV1 initialization sanity check is already done in `init()`
    //
    /// @dev test the proxy-upgradable LC contracts (V1 + V2) are properly initialized
    function test_ProxyInitialization() public {
        init();
        // V2 initialization
        assertEq(lc._blocksPerEpoch(), BLOCKS_PER_EPOCH);

        // V1 initialization
        (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot) = lc.genesisState();
        assertEq(genesis.viewNum, viewNum);
        assertEq(genesis.blockHeight, blockHeight);
        assertEq(genesis.blockCommRoot, blockCommRoot);

        (
            uint256 threshold,
            BN254.ScalarField stakeTableBlsKeyComm,
            BN254.ScalarField stakeTableSchnorrKeyComm,
            BN254.ScalarField stakeTableAmountComm
        ) = lc.genesisStakeTableState();
        assertEq(genesisStakeTableState.blsKeyComm, stakeTableBlsKeyComm);
        assertEq(genesisStakeTableState.schnorrKeyComm, stakeTableSchnorrKeyComm);
        assertEq(genesisStakeTableState.amountComm, stakeTableAmountComm);
        assertEq(genesisStakeTableState.threshold, threshold);

        assertEq(LCV2(proxyAddr).permissionedProver(), prover);
    }
}

contract LightClient_permissionedProver_Test is LightClientCommonTest {
    LC.LightClientState newState;
    LC.StakeTableState nextStakeTable;
    V.PlonkProof newProof;

    function setUp() public {
        init();
        (newState, nextStakeTable, newProof) = genStateProof();
    }

    function test_NoProverPermissionsRequired() external {
        //ensure that the permissioned prover mode is set
        assert(lc.isPermissionedProverEnabled());

        //set permissioned flag to false
        vm.expectEmit(true, true, true, true);
        emit LC.PermissionedProverNotRequired();
        vm.prank(admin);
        lc.disablePermissionedProverMode();

        //assert that the contract is not permissioned
        assert(lc.isPermissionedProverEnabled() == false);

        // assert that the prover address is zero address when the contract is not permissioned
        assertEq(lc.permissionedProver(), address(0));

        //any prover can call the newFinalizedState method as the contract is not in permissioned
        // prover mode
        vm.expectEmit(true, true, true, true);
        emit LC.NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
        vm.prank(makeAddr("randomUser"));
        lc.newFinalizedState(newState, nextStakeTable, newProof);
    }

    function test_UpdatePermissionedProverWhenPermissionedProverModeDisabled() external {
        vm.startPrank(admin);
        vm.expectEmit(true, true, true, true);
        emit LC.PermissionedProverNotRequired();
        lc.disablePermissionedProverMode();
        assertEq(lc.permissionedProver(), address(0));

        address newProver = makeAddr("another prover");
        vm.expectEmit(true, true, true, true);
        emit LC.PermissionedProverRequired(newProver);
        lc.setPermissionedProver(newProver);
        assertEq(newProver, lc.permissionedProver());
        vm.stopPrank();
    }

    function test_UpdatePermissionedProverWhenPermissionedProverModeEnabled() external {
        assert(lc.isPermissionedProverEnabled());
        assertEq(lc.permissionedProver(), prover);

        address newProver = makeAddr("another prover");
        vm.expectEmit(true, true, true, true);
        emit LC.PermissionedProverRequired(newProver);
        vm.prank(admin);
        lc.setPermissionedProver(newProver);
        assertEq(newProver, lc.permissionedProver());
    }

    function testFuzz_UpdatePermissionedProverWhenPermissionedProverModeEnabled(address newProver)
        external
    {
        // otherwise it would have reverted with InvalidAddress()
        vm.assume(newProver != address(0));
        // otherwise it would have reverted with NoChangeRequired()
        vm.assume(newProver != prover);

        assert(lc.isPermissionedProverEnabled());
        assertEq(lc.permissionedProver(), prover);

        vm.expectEmit(true, true, true, true);
        emit LC.PermissionedProverRequired(newProver);
        vm.prank(admin);
        lc.setPermissionedProver(newProver);
        assertEq(newProver, lc.permissionedProver());
    }

    function test_OldProverNoLongerWorks() public {
        assertEq(lc.permissionedProver(), prover);
        address oldPermissionedProver = prover;

        address prover2 = makeAddr("prover2");
        vm.expectEmit(true, true, true, true);
        emit LC.PermissionedProverRequired(prover2);
        vm.prank(admin);
        lc.setPermissionedProver(prover2);
        assertEq(lc.permissionedProver(), prover2);

        //confirm that the old prover doesn't work
        vm.prank(oldPermissionedProver);
        vm.expectRevert(LC.ProverNotPermissioned.selector);
        lc.newFinalizedState(newState, nextStakeTable, newProof);

        //confirm that the new prover works
        vm.prank(prover2);
        vm.expectEmit(true, true, true, true);
        emit LC.NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
        lc.newFinalizedState(newState, nextStakeTable, newProof);
    }

    function test_RevertWhen_sameProverSentInUpdate() public {
        assertEq(lc.isPermissionedProverEnabled(), true);
        address currentProver = lc.permissionedProver();
        vm.prank(admin);
        vm.expectRevert(LC.NoChangeRequired.selector);
        lc.setPermissionedProver(currentProver);
    }

    function test_RevertWhen_UpdatePermissionedProverToZeroAddress() external {
        vm.expectRevert(LC.InvalidAddress.selector);
        vm.prank(admin);
        lc.setPermissionedProver(address(0));
    }

    function test_RevertWhen_NonAdminTriesToUpdatePermissionedProver() external {
        vm.expectRevert();
        vm.prank(makeAddr("not an admin"));
        lc.setPermissionedProver(makeAddr("new prover"));
    }

    function test_RevertWhen_ProverDoesNotHavePermissions() external {
        vm.expectRevert(LC.ProverNotPermissioned.selector);
        vm.prank(makeAddr("ProverWithNoPermissions"));
        lc.newFinalizedState(newState, nextStakeTable, newProof);
    }

    function test_RevertWhen_ProverAddressNotPermissionedEvenIfAdminAddress() external {
        vm.expectRevert(LC.ProverNotPermissioned.selector);
        vm.prank(admin);
        lc.newFinalizedState(newState, nextStakeTable, newProof);
    }
}

contract LightClient_newFinalizedState_Test is LightClientCommonTest {
    LC.LightClientState newState;
    LC.StakeTableState nextStakeTable;
    V.PlonkProof newProof;

    function setUp() public {
        init();
        (newState, nextStakeTable, newProof) = genStateProof();
    }

    /// @dev for benchmarking purposes only
    function testCorrectUpdate() external {
        vm.expectEmit(true, true, true, true);
        emit LC.NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
        vm.prank(prover);
        lc.newFinalizedState(newState, nextStakeTable, newProof);
    }

    /// @dev Test happy path for (the number of states + 1) consecutive new finalized blocks
    function test_ConsecutiveUpdate() external {
        uint64 numInitValidators = STAKE_TABLE_CAPACITY / 2;

        // Generating a few consecutive states and proofs
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-consecutive-finalized-states";
        cmds[2] = vm.toString(numInitValidators);

        bytes memory result = vm.ffi(cmds);
        (
            LC.LightClientState[] memory states,
            LC.StakeTableState[] memory nextStakeTables,
            V.PlonkProof[] memory proofs
        ) = abi.decode(result, (LC.LightClientState[], LC.StakeTableState[], V.PlonkProof[]));

        uint256 statesLen = states.length;
        uint64 viewNum;
        uint64 blockHeight;
        BN254.ScalarField blockCommRoot;
        for (uint256 i = 0; i < statesLen; i++) {
            vm.expectEmit(true, true, true, true);
            emit LC.NewState(states[i].viewNum, states[i].blockHeight, states[i].blockCommRoot);
            vm.prank(prover);
            lc.newFinalizedState(states[i], nextStakeTables[i], proofs[i]);

            (viewNum, blockHeight, blockCommRoot) = lc.finalizedState();
            assertEq(viewNum, states[i].viewNum);
            assertEq(blockHeight, states[i].blockHeight);
            assertEq(abi.encode(blockCommRoot), abi.encode(states[i].blockCommRoot));
        }
    }

    /// @dev Test happy path for updating after skipping a few blocks
    function test_UpdateAfterSkippedBlocks() external {
        // note: numBlockSkipped=1 is already tested in `testCorrectUpdate()`
        for (uint32 numBlockSkipped = 2; numBlockSkipped < 5; numBlockSkipped++) {
            string[] memory cmds = new string[](3);
            cmds[0] = "diff-test";
            cmds[1] = "mock-skip-blocks";
            cmds[2] = vm.toString(numBlockSkipped);

            bytes memory result = vm.ffi(cmds);
            (
                LC.LightClientState memory state,
                LC.StakeTableState memory _nextStakeTable,
                V.PlonkProof memory proof
            ) = abi.decode(result, (LC.LightClientState, LC.StakeTableState, V.PlonkProof));

            vm.expectEmit(true, true, true, true);
            emit LC.NewState(state.viewNum, state.blockHeight, state.blockCommRoot);
            vm.prank(prover);
            lc.newFinalizedState(state, _nextStakeTable, proof);
        }
    }

    /// @dev Test unhappy path when a valid but oudated finalized state is submitted
    function test_RevertWhen_OutdatedStateSubmitted() external {
        uint64 numBlockSkipped = 1;
        LC.LightClientState memory state = genesis;
        state.viewNum = 10;
        vm.startPrank(prover);
        lc.setFinalizedState(state);

        // outdated view num
        vm.expectRevert(LC.OutdatedState.selector);
        lc.newFinalizedState(newState, nextStakeTable, newProof);

        // outdated block height
        state.viewNum = genesis.viewNum;
        state.blockHeight = numBlockSkipped + 1;
        vm.expectRevert(LC.OutdatedState.selector);
        lc.newFinalizedState(newState, nextStakeTable, newProof);
        vm.stopPrank();
    }

    /// @dev Test unhappy path when user inputs contain malformed field elements
    function test_RevertWhen_MalformedFieldElements() external {
        LC.LightClientState memory badState = newState;

        // invalid scalar for blockCommRoot
        vm.startPrank(prover);
        badState.blockCommRoot = BN254.ScalarField.wrap(BN254.R_MOD);
        vm.expectRevert("Bn254: invalid scalar field");
        lc.newFinalizedState(badState, nextStakeTable, newProof);
        badState.blockCommRoot = newState.blockCommRoot;
    }

    /// @dev Test unhappy path when the plonk proof or the public inputs are wrong
    function test_RevertWhen_WrongProofOrWrongPublicInput() external {
        BN254.ScalarField randScalar = BN254.ScalarField.wrap(1234);
        LC.LightClientState memory badState = LC.LightClientState({
            viewNum: newState.viewNum,
            blockHeight: newState.blockHeight,
            blockCommRoot: newState.blockCommRoot
        });

        // wrong view num
        vm.startPrank(prover);
        badState.viewNum = newState.viewNum + 2;
        vm.expectRevert(LC.InvalidProof.selector);
        lc.newFinalizedState(badState, nextStakeTable, newProof);
        badState.viewNum = newState.viewNum;

        // wrong block height
        badState.blockHeight = newState.blockHeight + 1;
        vm.expectRevert(LC.InvalidProof.selector);
        lc.newFinalizedState(badState, nextStakeTable, newProof);
        badState.blockHeight = newState.blockHeight;

        // wrong blockCommRoot
        badState.blockCommRoot = randScalar;
        vm.expectRevert(LC.InvalidProof.selector);
        lc.newFinalizedState(badState, nextStakeTable, newProof);
        badState.blockCommRoot = newState.blockCommRoot;

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "dummy-proof";
        cmds[2] = vm.toString(uint64(42));

        bytes memory result = vm.ffi(cmds);
        (V.PlonkProof memory dummyProof) = abi.decode(result, (V.PlonkProof));
        vm.expectRevert(LC.InvalidProof.selector);
        lc.newFinalizedState(newState, nextStakeTable, dummyProof);

        vm.stopPrank();
    }
}

contract LightClient_StateHistoryTest is LightClientCommonTest {
    LC.LightClientState newState;
    LC.StakeTableState nextStakeTable;
    V.PlonkProof newProof;

    function setUp() public {
        init();
        (newState, nextStakeTable, newProof) = genStateProof();

        //assert initial conditions
        assertEq(lc.stateHistoryFirstIndex(), 0);
        assertEq(lc.stateHistoryRetentionPeriod(), 1 days);
    }

    function test_1lBlockUpdatesIsUpdated() public {
        vm.prank(prover);
        lc.newFinalizedState(newState, nextStakeTable, newProof);

        // test that finalized state update will be added to the commitment history with correct
        // fields
        assertEq(lc.getStateHistoryCount(), 1);
        (,, uint64 blockHeight, BN254.ScalarField blockCommRoot) = lc.stateHistoryCommitments(0);
        assertEq(blockHeight, newState.blockHeight);
        assertEq(blockCommRoot, newState.blockCommRoot);
    }

    function testFuzz_setStateHistoryRetentionPeriod(uint256 duration) public {
        vm.prank(admin);
        duration = bound(duration, 1 days + 1, 365 days - 1);
        lc.setStateHistoryRetentionPeriod(uint32(duration));
        assertEq(duration, lc.stateHistoryRetentionPeriod());
    }

    function test_revertNonAdminSetMaxStateHistoryAllowed() public {
        address alice = makeAddr("alice");
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, alice));
        lc.setStateHistoryRetentionPeriod(1 days);
    }

    function test_revertWithInvalidRetentionPeriod() public {
        vm.startPrank(admin);
        vm.expectRevert(LC.InvalidMaxStateHistory.selector);
        lc.setStateHistoryRetentionPeriod(366 days);

        vm.expectRevert(LC.InvalidMaxStateHistory.selector);
        lc.setStateHistoryRetentionPeriod(1 hours - 1);

        uint32 curDuration = lc.stateHistoryRetentionPeriod();
        vm.expectRevert(LC.InvalidMaxStateHistory.selector);
        lc.setstateHistoryRetentionPeriod(curDuration - 1);
    }

    function test_CorrectStateHistoryHandling() public {
        // populate the first update
        lc.setFinalizedState(simulateNewState(10));
        assertEq(lc.stateHistoryFirstIndex(), 0);
        assertEq(lc.getStateHistoryCount(), 1);

        // populate the second update
        lc.setFinalizedState(simulateNewState(10));
        assertEq(lc.stateHistoryFirstIndex(), 0);
        assertEq(lc.getStateHistoryCount(), 2);

        // elapse time beyond the retention period and populate the 3rd update
        vm.warp(MAX_HISTORY_SECONDS + 1 hours);
        lc.setFinalizedState(simulateNewState(10));
        // the first two are both expired out of the retention, but only the first one is pruned,
        // this is intentional design decision "prune one per invocation"
        assertEq(lc.stateHistoryFirstIndex(), 1);
        assertEq(lc.getStateHistoryCount(), 3);

        // continue to populate more updates (already had 3)
        uint256 numUpdates = 8;
        for (uint256 i = 0; i < numUpdates - 3; i++) {
            lc.setFinalizedState(simulateNewState(10));
        }
        // only the first 2 are expired
        assertEq(lc.stateHistoryFirstIndex(), 2);
        assertEq(lc.getStateHistoryCount(), numUpdates);

        // check that the (latest - oldest) < retention period
        (, uint64 latestTimestamp,,) = lc.stateHistoryCommitments(lc.getStateHistoryCount() - 1);
        (, uint64 oldestTimestamp,,) = lc.stateHistoryCommitments(lc.stateHistoryFirstIndex());
        assertLe(latestTimestamp - oldestTimestamp, lc.stateHistoryRetentionPeriod());

        // check that expired ones are pruned (set to zero) and un-pruned ones are non-zeros
        for (uint256 i = 0; i < lc.stateHistoryFirstIndex(); i++) {
            (uint64 a, uint256 b, uint64 c, BN254.ScalarField d) = lc.stateHistoryCommitments(i);
            assertEq(a, 0);
            assertEq(b, 0);
            assertEq(c, 0);
            assertEq(BN254.ScalarField.unwrap(d), 0);
        }
        for (uint256 i = lc.stateHistoryFirstIndex(); i < lc.getStateHistoryCount(); i++) {
            (, uint256 activeBlockTimestamp,,) = lc.stateHistoryCommitments(i);
            assertNotEq(activeBlockTimestamp, 0);
        }
    }

    function test_getHotShotCommitmentHappyPath() public {
        uint256 numPruned = 4;
        uint256 numUpdates = 20;
        assertGe(numUpdates, 2 * numPruned);

        LC.LightClientState[] memory states = new LC.LightClientState[](numUpdates);
        // fill in hotshot updates that will be pruned
        for (uint64 i = 0; i < numPruned; i++) {
            states[i] = simulateNewState(i + 5);
            lc.setFinalizedState(states[i]);
        }
        // advancing time to effectively mark all `numPruned` updates above expired
        vm.warp(MAX_HISTORY_SECONDS + 1 hours);
        for (uint64 i = 0; i < numUpdates - numPruned; i++) {
            states[i + numPruned] = simulateNewState(i + 5);
            lc.setFinalizedState(states[i + numPruned]);
        }
        // sanity check
        assertEq(lc.stateHistoryFirstIndex(), numPruned);
        assertEq(lc.getStateHistoryCount(), numUpdates);

        // actual unpruned updates themselves should be found
        for (uint256 i = numPruned; i < numUpdates; i++) {
            // will find the first one that's *greater* than queried block height, thus -1
            (BN254.ScalarField root, uint64 height) =
                lc.getHotShotCommitment(states[i].blockHeight - 1);
            assertEq(root, states[i].blockCommRoot);
            assertEq(height, states[i].blockHeight);
        }

        // return the oldest for all queries that is earlier than the oldest commitment stored
        for (uint256 i = 0; i < states[numPruned].blockHeight; i++) {
            (BN254.ScalarField root, uint64 height) = lc.getHotShotCommitment(i);
            assertEq(root, states[numPruned].blockCommRoot);
            assertEq(height, states[numPruned].blockHeight);
        }

        // return the next update for any height in the middle of two updates (except the last
        // update)
        for (uint256 i = numPruned; i < numUpdates - 1; i++) {
            uint64 lastUpdateHeight = states[i].blockHeight;
            uint64 nextUpdateHeight = states[i + 1].blockHeight;
            for (uint256 j = lastUpdateHeight; j < nextUpdateHeight; j++) {
                (BN254.ScalarField root, uint64 height) = lc.getHotShotCommitment(j);
                assertEq(root, states[i + 1].blockCommRoot);
                assertEq(height, states[i + 1].blockHeight);
            }
        }
    }

    function test_RevertWhen_QueryLatestOrLaterHotshotCommitment() public {
        uint256 numUpdates = 5;
        LC.LightClientState[] memory states = new LC.LightClientState[](numUpdates);
        for (uint64 i = 0; i < numUpdates; i++) {
            states[i] = simulateNewState(i + 5);
            lc.setFinalizedState(states[i]);
        }

        uint64 latestHeight = states[numUpdates - 1].blockHeight;
        vm.expectRevert(LC.InvalidHotShotBlockForCommitmentCheck.selector);
        lc.getHotShotCommitment(latestHeight);
        vm.expectRevert(LC.InvalidHotShotBlockForCommitmentCheck.selector);
        lc.getHotShotCommitment(latestHeight + 1);
        vm.expectRevert(LC.InvalidHotShotBlockForCommitmentCheck.selector);
        lc.getHotShotCommitment(latestHeight + 10);
    }
}

/// @dev Testing liveness signal function `lagOverEscapeHatchThreshold()`
/// @dev Functional test for `lagOverEscapeHatchThreshold(queriedBlock, threshold T)`
/// case 1: updates[i] --x-- queriedBlock --y-- updates[i+1]: return true if T >= x, else false
/// case 2: lastUpdate --x-- queriedBlock: returns true if T >= x, else false
/// case 3: -- queriedBlock --y-- firstUpdate: revert InsufficientSnapshotHistory
/// case 4: -- now -- queriedBlock: revert InsufficientSnapshotHistory
/// case 5: no update: revert InsufficientSnapshotHistory
contract LightClient_LivenessDetectionTest is LightClientCommonTest {
    uint256 numUpdates = 5;
    LC.LightClientState[] states;
    uint64[] updateL1Heights;

    function setUp() public {
        init();

        for (uint64 i = 0; i < numUpdates; i++) {
            states.push(simulateNewState(i + 7));
            lc.setFinalizedState(states[i]);
        }
        // record the l1 heights at time of their updates
        for (uint64 i = 0; i < numUpdates; i++) {
            (uint64 l1Height,,,) = lc.stateHistoryCommitments(i);
            updateL1Heights.push(l1Height);
        }
    }

    /// @dev Case 1: updates[i] --x-- queriedBlock --y-- updates[i+1]
    function test_LagBetweenUpdates() public view {
        for (uint256 i = 0; i < numUpdates - 1; i++) {
            uint256 updateInterval = updateL1Heights[i + 1] - updateL1Heights[i];
            for (
                uint256 queriedBlock = updateL1Heights[i];
                queriedBlock < updateL1Heights[i + 1];
                queriedBlock++
            ) {
                for (uint256 threshold = 0; threshold <= updateInterval + 5; threshold++) {
                    if (queriedBlock - updateL1Heights[i] > threshold) {
                        assertTrue(lc.lagOverEscapeHatchThreshold(queriedBlock, threshold));
                    } else {
                        assertFalse(lc.lagOverEscapeHatchThreshold(queriedBlock, threshold));
                    }
                }
            }
        }
    }

    /// @dev case 2: lastUpdate --x-- queriedBlock
    function test_LagAfterLastUpdate() public {
        vm.roll(block.number + 10);
        uint256 lastUpdateL1Height = updateL1Heights[numUpdates - 1];
        for (uint256 queriedBlock = lastUpdateL1Height; queriedBlock < block.number; queriedBlock++)
        {
            for (uint256 threshold = 0; threshold <= 10 + 5; threshold++) {
                if (queriedBlock - lastUpdateL1Height > threshold) {
                    assertTrue(lc.lagOverEscapeHatchThreshold(queriedBlock, threshold));
                } else {
                    assertFalse(lc.lagOverEscapeHatchThreshold(queriedBlock, threshold));
                }
            }
        }
    }

    /// @dev case 3: -- queriedBlock --y-- firstUpdate
    function test_RevertWhen_QueryBeforeFirstUpdate() public {
        uint256 firstUpdateL1Height = updateL1Heights[0];
        for (uint256 queriedBlock = 0; queriedBlock < firstUpdateL1Height; queriedBlock++) {
            vm.expectRevert(LC.InsufficientSnapshotHistory.selector);
            lc.lagOverEscapeHatchThreshold(queriedBlock, 0);
        }
    }

    /// @dev case 4: -- now -- queriedBlock
    function test_RevertWhen_QueryFutureBlocks() public {
        // this should be fine
        lc.lagOverEscapeHatchThreshold(block.number, 0);

        // but querying anything in the future should revert
        vm.expectRevert(LC.InsufficientSnapshotHistory.selector);
        lc.lagOverEscapeHatchThreshold(block.number + 1, 0);
        vm.expectRevert(LC.InsufficientSnapshotHistory.selector);
        lc.lagOverEscapeHatchThreshold(block.number + 5, 0);
        vm.expectRevert(LC.InsufficientSnapshotHistory.selector);
        lc.lagOverEscapeHatchThreshold(block.number + 50, 0);
    }

    /// @dev case 5: no update
    function test_RevertWhen_QueryButNoUpdate() public {
        init(); // re-init
        assertEq(lc.getStateHistoryCount(), 0);

        vm.expectRevert(LC.InsufficientSnapshotHistory.selector);
        lc.lagOverEscapeHatchThreshold(0, 0);
        vm.expectRevert(LC.InsufficientSnapshotHistory.selector);
        lc.lagOverEscapeHatchThreshold(block.number, 0);
    }
}

/// @dev Ensure production-deployed V1 can be upgraded to V2 properly
contract LightClient_V1ToV2UpgradeTest is LightClientCommonTest {
    string sepoliaRpcUrl = "https://0xrpc.io/sep";
    address proxy = 0x303872BB82a191771321d4828888920100d0b3e4;

    function test_ForkTest_UpgradeToV2() public {
        // create fork on Sepolia on which we have deployed LightClient
        // proxy: https://sepolia.etherscan.io/address/0x303872bb82a191771321d4828888920100d0b3e4
        vm.createSelectFork(sepoliaRpcUrl, 7844940); // March 6th, 2025
        assertEq(block.number, 7844940);
        (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion) = LC(proxy).getVersion();
        assertEq(majorVersion, 1);
        assertEq(minorVersion, 0);
        assertEq(patchVersion, 0);
        (
            uint256 genesisThreshold,
            BN254.ScalarField genesisBlsKeyComm,
            BN254.ScalarField genesisSchnorrKeyComm,
            BN254.ScalarField genesisAmountComm
        ) = LC(proxy).genesisStakeTableState();

        // first deploy LCV2
        LC lcv2 = new LCV2();
        PV2 pv2 = new PV2();
        bytes memory lcv2InitData =
            abi.encodeWithSignature("initializeV2(uint64,address)", BLOCKS_PER_EPOCH, address(pv2));
        // upgrade V1 to V2 and initialize LCV2
        admin = LC(proxy).owner();
        vm.prank(admin);
        LC(proxy).upgradeToAndCall(address(lcv2), lcv2InitData);

        // test LCV2 is successfully in effect
        (majorVersion, minorVersion, patchVersion) = LCV2(proxy).getVersion();
        assertEq(majorVersion, 2);
        assertEq(minorVersion, 0);
        assertEq(patchVersion, 0);
        assertEq(LCV2(proxy)._blocksPerEpoch(), BLOCKS_PER_EPOCH);
        (
            uint256 threshold,
            BN254.ScalarField blsKeyComm,
            BN254.ScalarField schnorrKeyComm,
            BN254.ScalarField amountComm
        ) = LCV2(proxy).votingStakeTableState();
        assertEq(threshold, genesisThreshold);
        assertEq(blsKeyComm, genesisBlsKeyComm);
        assertEq(schnorrKeyComm, genesisSchnorrKeyComm);
        assertEq(amountComm, genesisAmountComm);
    }
}
