    // SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
// import {console} from "forge-std/console.sol";

using stdStorage for StdStorage;

import { ERC20 } from "solmate/utils/SafeTransferLib.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "../src/libraries/BLSSig.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { LightClient } from "../src/LightClient.sol";
import { LightClientMock } from "../test/mocks/LightClientMock.sol";
import { InitializedAt } from "../src/InitializedAt.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { IPlonkVerifier as V } from "../src/interfaces/IPlonkVerifier.sol";
import { TimelockController } from "@openzeppelin/contracts/governance/TimelockController.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
// Token contract
import { EspToken } from "../src/EspToken.sol";

// Target contract
import { StakeTable as S } from "../src/StakeTable.sol";
import { StakeTableMock } from "../test/mocks/StakeTableMock.sol";
import { DeployStakeTableScript } from "./script/StakeTable.s.sol";
import { DeployEspTokenScript } from "./script/EspToken.s.sol";
import { Timelock } from "../src/Timelock.sol";
// TODO: currently missing several tests
// TODO: test only owner methods access control

contract StakeTable_register_Test is Test {
    StakeTableMock public stakeTable;
    address payable public proxy;
    address public admin;
    EspToken public token;
    LightClientMock public lcMock;
    uint256 public constant INITIAL_BALANCE = 5 ether;
    uint256 public constant ESCROW_PERIOD = 1 weeks;
    uint16 public constant COMMISSION = 1234; // 12.34 %
    address public tokenGrantRecipient;
    address public delegator;
    address public validator;
    string seed1 = "1";
    string seed2 = "255";

    function genClientWallet(address sender, string memory _seed)
        private
        returns (BN254.G2Point memory, EdOnBN254.EdOnBN254Point memory, BN254.G1Point memory)
    {
        // Generate a BLS signature and other values using rust code
        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "gen-client-wallet";
        cmds[2] = vm.toString(sender);
        cmds[3] = _seed;

        bytes memory result = vm.ffi(cmds);
        (
            BN254.G1Point memory blsSig,
            BN254.G2Point memory blsVK,
            uint256 schnorrVKx,
            uint256 schnorrVKy,
        ) = abi.decode(result, (BN254.G1Point, BN254.G2Point, uint256, uint256, address));

        return (
            blsVK, // blsVK
            EdOnBN254.EdOnBN254Point(schnorrVKx, schnorrVKy), // schnorrVK
            blsSig // sig
        );
    }

    function setUp() public {
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        validator = makeAddr("validator");
        delegator = makeAddr("delegator");
        admin = makeAddr("admin");

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = "5";

        bytes memory result = vm.ffi(cmds);
        (
            LightClientMock.LightClientState memory state,
            LightClientMock.StakeTableState memory stakeState
        ) = abi.decode(result, (LightClient.LightClientState, LightClient.StakeTableState));
        LightClientMock.LightClientState memory genesis = state;
        LightClientMock.StakeTableState memory genesisStakeTableState = stakeState;

        lcMock = new LightClientMock(genesis, genesisStakeTableState, 864000);

        DeployEspTokenScript tokenDeployer = new DeployEspTokenScript();
        (address tokenAddress,) = tokenDeployer.run(tokenGrantRecipient);
        token = EspToken(tokenAddress);

        vm.prank(tokenGrantRecipient);
        token.transfer(address(validator), INITIAL_BALANCE);

        DeployStakeTableScript stakeTableDeployer = new DeployStakeTableScript();
        (proxy, admin) = stakeTableDeployer.run(
            tokenAddress, address(lcMock), ESCROW_PERIOD, admin, makeAddr("timelock")
        );
        stakeTable = StakeTableMock(proxy);
    }

    function test_Deployment_StoresBlockNumber() public {
        assertEq(stakeTable.initializedAtBlock(), block.number);
    }

    function testFuzz_RevertWhen_InvalidBLSSig(uint256 scalar) external {
        uint64 depositAmount = 10 ether;

        (BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK,) =
            genClientWallet(validator, seed1);

        // Prepare for the token transfer
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Ensure the scalar is valid
        // Note: Apparently BN254.scalarMul is not well defined when the scalar is 0
        scalar = bound(scalar, 1, BN254.R_MOD - 1);
        BN254.validateScalarField(BN254.ScalarField.wrap(scalar));
        BN254.G1Point memory badSig = BN254.scalarMul(BN254.P1(), BN254.ScalarField.wrap(scalar));
        BN254.validateG1Point(badSig);

        // Failed signature verification
        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        stakeTable.registerValidator(blsVK, schnorrVK, badSig, COMMISSION);
        vm.stopPrank();
    }

    function test_RevertWhen_NodeAlreadyRegistered() external {
        uint64 depositAmount = 10 ether;

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer
        vm.prank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Successful call to register
        vm.prank(validator);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // The node is already registered
        vm.prank(validator);
        vm.expectRevert(S.ValidatorAlreadyRegistered.selector);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);
    }

    function test_RevertWhen_NoTokenAllowanceOrBalance() external {
        uint64 depositAmount = 10 ether;

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        vm.prank(validator);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        vm.startPrank(delegator);
        // The call to register is expected to fail because the depositAmount has not been approved
        // and thus the stake table contract cannot lock the stake.
        vm.expectRevert(abi.encodeWithSelector(S.InsufficientAllowance.selector, 0, depositAmount));
        stakeTable.delegate(validator, depositAmount);

        // Prepare for the token transfer by giving the StakeTable contract the required allowance
        token.approve(address(stakeTable), depositAmount);

        // TODO MA: this error is from solady's ERC20 implementation, needs to be updated in case we
        // use another ERC20 implementation for our token. I think it's fair to expect a revert from
        // *our* ERC20 token if the does not have the balance.
        vm.expectRevert("TRANSFER_FROM_FAILED");
        stakeTable.delegate(validator, depositAmount);

        vm.stopPrank();
    }

    /// @dev Tests a correct registration
    function test_Registration_succeeds() external {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        vm.prank(validator);
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorRegistered(validator, blsVK, schnorrVK, COMMISSION);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);
    }

    /// @dev Tests a correct registration
    function test_RevertWhen_InvalidBlsVK_or_InvalidSchnorrVK_on_Registration() external {
        // generate a valid blsVK and schnorrVK
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer
        vm.startPrank(validator);

        // revert when the blsVK is the zero point
        BN254.G2Point memory zeroBlsVK = BN254.G2Point(
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0)
        );
        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        stakeTable.registerValidator(zeroBlsVK, schnorrVK, sig, COMMISSION);

        // revert when the schnorrVK is the zero point
        EdOnBN254.EdOnBN254Point memory zeroSchnorrVK = EdOnBN254.EdOnBN254Point(0, 0);
        vm.expectRevert(S.InvalidSchnorrVK.selector);
        stakeTable.registerValidator(blsVK, zeroSchnorrVK, sig, COMMISSION);

        vm.stopPrank();
    }

    function test_UpdateConsensusKeys_Succeeds() public {
        uint64 depositAmount = 10 ether;

        //Step 1: generate a new blsVK and schnorrVK and register this node
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer by granting allowance to the contract
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(validator), INITIAL_BALANCE);

        // Check event is emitted after calling successfully `register`
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorRegistered(validator, blsVK, schnorrVK, COMMISSION);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // Step 2: generate a new blsVK and schnorrVK
        (
            BN254.G2Point memory newBlsVK,
            EdOnBN254.EdOnBN254Point memory newSchnorrVK,
            BN254.G1Point memory newBlsSig
        ) = genClientWallet(validator, seed2);

        // Step 3: update the consensus keys
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ConsensusKeysUpdated(validator, newBlsVK, newSchnorrVK);
        stakeTable.updateConsensusKeys(newBlsVK, newSchnorrVK, newBlsSig);

        vm.stopPrank();
    }

    function test_RevertWhen_UpdateConsensusKeysWithSameBlsKey() public {
        uint64 depositAmount = 10 ether;

        //Step 1: generate a new blsVK and schnorrVK and register this node
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer by granting allowance to the contract
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(validator), INITIAL_BALANCE);

        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // Step 2: update the consensus keys with the same keys
        vm.expectRevert(S.BlsKeyAlreadyUsed.selector);
        stakeTable.updateConsensusKeys(blsVK, schnorrVK, sig);

        vm.stopPrank();
    }

    function test_RevertWhen_UpdateConsensusKeysWithEmptyKeys() public {
        uint64 depositAmount = 10 ether;

        //Step 1: generate a new blsVK and schnorrVK and register this node
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer by granting allowance to the contract
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(validator), INITIAL_BALANCE);

        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // empty keys
        BN254.G2Point memory emptyBlsVK = BN254.G2Point(
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0)
        );
        EdOnBN254.EdOnBN254Point memory emptySchnorrVK = EdOnBN254.EdOnBN254Point(0, 0);

        // Step 2: attempt to update the consensus keys with the same keys
        vm.expectRevert(S.InvalidSchnorrVK.selector);
        stakeTable.updateConsensusKeys(emptyBlsVK, emptySchnorrVK, sig);

        vm.stopPrank();
    }

    function test_RevertWhen_UpdateConsensusKeysWithInvalidSignature() public {
        uint64 depositAmount = 10 ether;

        //Step 1: generate a new blsVK and schnorrVK and register this node
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer by granting allowance to the contract
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(validator), INITIAL_BALANCE);

        BN254.G1Point memory badSig =
            BN254.G1Point(BN254.BaseField.wrap(0), BN254.BaseField.wrap(0));

        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // Step 2: generate a new blsVK and schnorrVK
        (BN254.G2Point memory newBlsVK, EdOnBN254.EdOnBN254Point memory newSchnorrVK,) =
            genClientWallet(validator, seed2);

        // Step 3: attempt to update the consensus keys with the new keys but invalid signature
        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        stakeTable.updateConsensusKeys(newBlsVK, newSchnorrVK, badSig);

        vm.stopPrank();
    }

    function test_RevertWhen_UpdateConsensusKeysWithZeroBlsKeyButNewSchnorrVK() public {
        uint64 depositAmount = 10 ether;

        //Step 1: generate a new blsVK and schnorrVK and register this node
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer by granting allowance to the contract
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(validator), INITIAL_BALANCE);

        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorRegistered(validator, blsVK, schnorrVK, COMMISSION);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // Step 2: generate an empty and new schnorrVK
        (, EdOnBN254.EdOnBN254Point memory newSchnorrVK,) = genClientWallet(validator, seed2);

        BN254.G2Point memory emptyBlsVK = BN254.G2Point(
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0)
        );

        // Step 3: empty bls key -> wrong signature
        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        stakeTable.updateConsensusKeys(emptyBlsVK, newSchnorrVK, sig);

        vm.stopPrank();
    }

    function test_RevertWhen_UpdateConsensusKeysWithZeroSchnorrVKButNewBlsVK() public {
        uint64 depositAmount = 10 ether;

        //Step 1: generate a new blsVK and schnorrVK and register this node
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer by granting allowance to the contract
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(validator), INITIAL_BALANCE);

        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // Step 2: generate a new blsVK
        (BN254.G2Point memory newBlsVK,, BN254.G1Point memory newSig) =
            genClientWallet(validator, seed2);

        // Step 3: generate empty schnorrVK
        EdOnBN254.EdOnBN254Point memory emptySchnorrVK = EdOnBN254.EdOnBN254Point(0, 0);

        // Step 4: update the consensus keys with the new bls keys but empty schnorrVK
        vm.expectRevert(S.InvalidSchnorrVK.selector);
        stakeTable.updateConsensusKeys(newBlsVK, emptySchnorrVK, newSig);

        vm.stopPrank();
    }

    function test_UpdateConsensusKeysWithNewBlsKeyButSameSchnorrVK_Succeeds() public {
        uint64 depositAmount = 10 ether;

        //Step 1: generate a new blsVK and schnorrVK and register this node
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        // Prepare for the token transfer by granting allowance to the contract
        vm.startPrank(validator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(validator), INITIAL_BALANCE);

        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorRegistered(validator, blsVK, schnorrVK, COMMISSION);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        // Step 2: generate an empty and new schnorrVK
        (BN254.G2Point memory newBlsVK,, BN254.G1Point memory newSig) =
            genClientWallet(validator, seed2);

        // Step 3: update the consensus keys with the same bls keys but new schnorrV
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ConsensusKeysUpdated(validator, newBlsVK, schnorrVK);
        stakeTable.updateConsensusKeys(newBlsVK, schnorrVK, newSig);

        vm.stopPrank();
    }

    function test_claimWithdrawal_succeeds() public {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(validator, seed1);

        vm.prank(tokenGrantRecipient);
        token.transfer(delegator, INITIAL_BALANCE);

        vm.prank(delegator);
        token.approve(address(stakeTable), INITIAL_BALANCE);
        assertEq(token.balanceOf(delegator), INITIAL_BALANCE);

        // register the node
        vm.prank(validator);
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorRegistered(validator, blsVK, schnorrVK, COMMISSION);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, COMMISSION);

        vm.startPrank(delegator);

        // Delegate some funds
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.Delegated(delegator, validator, 3 ether);
        stakeTable.delegate(validator, 3 ether);

        assertEq(token.balanceOf(delegator), INITIAL_BALANCE - 3 ether);
        assertEq(token.balanceOf(address(stakeTable)), 3 ether);

        // Withdraw without undelegation
        vm.expectRevert(S.NothingToWithdraw.selector);
        stakeTable.claimWithdrawal(validator);

        // Request partial undelegation of funds
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.Undelegated(delegator, validator, 1 ether);
        stakeTable.undelegate(validator, 1 ether);

        // Withdraw too early
        vm.expectRevert(S.PrematureWithdrawal.selector);
        stakeTable.claimWithdrawal(validator);

        // Withdraw after escrow period
        vm.warp(block.timestamp + ESCROW_PERIOD);
        stakeTable.claimWithdrawal(validator);
        assertEq(token.balanceOf(delegator), INITIAL_BALANCE - 2 ether);

        vm.stopPrank();

        // Validator exit
        vm.prank(validator);
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorExit(validator);
        stakeTable.deregisterValidator();

        vm.startPrank(delegator);

        // Withdraw too early
        vm.expectRevert(S.PrematureWithdrawal.selector);
        stakeTable.claimValidatorExit(validator);

        // Try to undelegate after validator exit
        vm.expectRevert(S.ValidatorInactive.selector);
        stakeTable.undelegate(validator, 1);

        // Withdraw after escrow period
        vm.warp(block.timestamp + ESCROW_PERIOD);
        stakeTable.claimValidatorExit(validator);

        // The delegator withdrew all their funds
        assertEq(token.balanceOf(delegator), INITIAL_BALANCE);

        vm.stopPrank();
    }

    // solhint-disable-next-line no-empty-blocks
    function test_revertIf_undelegate_AfterValidatorExit() public {
        // TODO
    }
}

contract StakeTableTimelockTest is Test {
    address impl;
    address proxy;
    address tokenGrantRecipient;
    address validator;
    address delegator;
    address admin;
    address[] proposers = [makeAddr("proposer")];
    address[] executors = [makeAddr("executor")];
    LightClientMock lcMock;
    EspToken token;
    StakeTableMock stakeTable;
    Timelock timelock;
    uint256 public constant INITIAL_BALANCE = 5 ether;
    uint256 public constant ESCROW_PERIOD = 1 weeks;
    uint256 public constant DELAY = 15 seconds;

    function setUp() public {
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        validator = makeAddr("validator");
        delegator = makeAddr("delegator");
        admin = makeAddr("admin");

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = "5";

        bytes memory result = vm.ffi(cmds);
        (
            LightClientMock.LightClientState memory state,
            LightClientMock.StakeTableState memory stakeState
        ) = abi.decode(result, (LightClient.LightClientState, LightClient.StakeTableState));
        LightClientMock.LightClientState memory genesis = state;
        LightClientMock.StakeTableState memory genesisStakeTableState = stakeState;

        lcMock = new LightClientMock(genesis, genesisStakeTableState, 864000);

        DeployEspTokenScript tokenDeployer = new DeployEspTokenScript();
        (address tokenAddress,) = tokenDeployer.run(tokenGrantRecipient);
        token = EspToken(tokenAddress);

        vm.prank(tokenGrantRecipient);
        token.transfer(address(validator), INITIAL_BALANCE);

        //deploy timelock
        timelock = new Timelock(DELAY, proposers, executors, admin);

        DeployStakeTableScript stakeTableDeployer = new DeployStakeTableScript();
        (proxy, admin) = stakeTableDeployer.run(
            tokenAddress, address(lcMock), ESCROW_PERIOD, admin, address(timelock)
        );
        stakeTable = StakeTableMock(proxy);
    }

    function test_initialize_sets_timelock() public {
        vm.startPrank(admin);
        stakeTable.transferOwnership(address(timelock));
        assertEq(stakeTable.timelock(), address(timelock));
        assertEq(stakeTable.owner(), address(timelock));
    }

    function test_timelock_upgrade_proposal_and_execution_succeeds() public {
        vm.startPrank(admin);
        stakeTable.transferOwnership(address(timelock));
        vm.stopPrank();

        vm.startPrank(proposers[0]);

        // Encode upgrade call
        bytes memory data =
            abi.encodeWithSignature("upgradeToAndCall(address,bytes)", address(new S()), "");

        bytes32 txId = timelock.hashOperation(address(stakeTable), 0, data, bytes32(0), bytes32(0));

        timelock.schedule(address(stakeTable), 0, data, bytes32(0), bytes32(0), DELAY);

        vm.stopPrank();

        vm.assertFalse(timelock.isOperationReady(txId));

        vm.warp(block.timestamp + DELAY + 1);

        vm.assertTrue(timelock.isOperationReady(txId));

        vm.startPrank(executors[0]);
        timelock.execute(address(proxy), 0, data, bytes32(0), bytes32(0));
        vm.stopPrank();
        vm.assertTrue(timelock.isOperationDone(txId));
    }

    function test_expect_revert_when_timelock_is_not_owner() public {
        vm.startPrank(proposers[0]);

        // Encode upgrade call
        bytes memory data =
            abi.encodeWithSignature("upgradeToAndCall(address,bytes)", address(new S()), "");

        bytes32 txId = timelock.hashOperation(address(stakeTable), 0, data, bytes32(0), bytes32(0));
        timelock.schedule(address(stakeTable), 0, data, bytes32(0), bytes32(0), DELAY);

        vm.stopPrank();

        vm.assertFalse(timelock.isOperationReady(txId));

        vm.warp(block.timestamp + DELAY + 1);

        vm.assertTrue(timelock.isOperationReady(txId));

        vm.startPrank(executors[0]);
        vm.expectRevert(
            abi.encodeWithSelector(
                OwnableUpgradeable.OwnableUnauthorizedAccount.selector, address(timelock)
            )
        );
        timelock.execute(address(proxy), 0, data, bytes32(0), bytes32(0));
        vm.stopPrank();
        vm.assertFalse(timelock.isOperationDone(txId));
    }

    function test_expect_revert_when_timelock_upgrade_proposal_and_execution_before_delay()
        public
    {
        vm.startPrank(proposers[0]);

        // Encode upgrade call
        bytes memory data =
            abi.encodeWithSignature("upgradeToAndCall(address,bytes)", address(new S()), "");

        bytes32 txId = timelock.hashOperation(address(stakeTable), 0, data, bytes32(0), bytes32(0));
        timelock.schedule(address(stakeTable), 0, data, bytes32(0), bytes32(0), DELAY);

        vm.stopPrank();

        vm.startPrank(executors[0]);
        vm.expectRevert(
            abi.encodeWithSelector(
                TimelockController.TimelockUnexpectedOperationState.selector,
                txId,
                bytes32(1 << uint8(TimelockController.OperationState.Ready))
            )
        );
        timelock.execute(address(proxy), 0, data, bytes32(0), bytes32(0));
        vm.stopPrank();
        vm.assertFalse(timelock.isOperationDone(txId));
    }

    function test_expect_revert_when_timelock_upgrade_proposal_and_execution_without_correct_permission(
    ) public {
        vm.startPrank(makeAddr("notProposer"));

        // Encode upgrade call
        bytes memory data =
            abi.encodeWithSignature("upgradeToAndCall(address,bytes)", address(new S()), "");

        bytes32 txId = timelock.hashOperation(address(stakeTable), 0, data, bytes32(0), bytes32(0));
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                address(makeAddr("notProposer")),
                timelock.PROPOSER_ROLE()
            )
        );
        timelock.schedule(address(stakeTable), 0, data, bytes32(0), bytes32(0), DELAY);
        vm.stopPrank();

        vm.startPrank(proposers[0]);
        timelock.schedule(address(stakeTable), 0, data, bytes32(0), bytes32(0), DELAY);
        vm.stopPrank();

        vm.warp(block.timestamp + DELAY + 1);

        vm.startPrank(makeAddr("notExecutor"));
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                address(makeAddr("notExecutor")),
                timelock.EXECUTOR_ROLE()
            )
        );
        timelock.execute(address(proxy), 0, data, bytes32(0), bytes32(0));
        vm.stopPrank();
        vm.assertFalse(timelock.isOperationDone(txId));
    }

    function test_expect_revert_when_execute_with_wrong_salt() public {
        // Encode upgrade call
        bytes memory data =
            abi.encodeWithSignature("upgradeToAndCall(address,bytes)", address(new S()), "");

        bytes32 correctSalt = keccak256("salt-A");
        bytes32 wrongSalt = keccak256("salt-B");

        bytes32 wrongTxId =
            timelock.hashOperation(address(stakeTable), 0, data, wrongSalt, bytes32(0));
        vm.startPrank(proposers[0]);
        timelock.schedule(address(stakeTable), 0, data, correctSalt, bytes32(0), DELAY);
        vm.stopPrank();

        vm.warp(block.timestamp + DELAY + 1);

        vm.startPrank(executors[0]);
        vm.expectRevert(
            abi.encodeWithSelector(
                TimelockController.TimelockUnexpectedOperationState.selector,
                wrongTxId,
                bytes32(1 << uint8(TimelockController.OperationState.Ready))
            )
        );
        timelock.execute(address(stakeTable), 0, data, wrongSalt, bytes32(0));
        vm.stopPrank();
    }

    function test_unauthorized_cannot_upgrade() public {
        vm.startPrank(makeAddr("notAdmin"));

        try stakeTable.upgradeToAndCall(address(new S()), "") {
            fail();
        } catch (bytes memory lowLevelData) {
            // Handle custom error
            bytes4 selector;
            assembly {
                selector := mload(add(lowLevelData, 32))
            }
            assertEq(selector, OwnableUpgradeable.OwnableUnauthorizedAccount.selector);
        }
    }
}
