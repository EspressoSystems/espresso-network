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

// Token contract
import { EspToken } from "../src/EspToken.sol";

// Target contract
import { StakeTable as S } from "../src/StakeTable.sol";
import { StakeTableMock } from "../test/mocks/StakeTableMock.sol";
import { DeployStakeTableScript } from "./script/StakeTable.s.sol";
import { DeployEspTokenScript } from "./script/EspToken.s.sol";

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
    uint64 public hotShotBlocksPerEpoch = 1;
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

    function setUpCustom() public {
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        validator = makeAddr("validator");
        delegator = makeAddr("delegator");

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
        (address tokenAddress, address admin) = tokenDeployer.run(tokenGrantRecipient);
        token = EspToken(tokenAddress);

        vm.prank(tokenGrantRecipient);
        token.transfer(address(validator), INITIAL_BALANCE);

        DeployStakeTableScript stakeTableDeployer = new DeployStakeTableScript();
        (proxy, admin) = stakeTableDeployer.run(tokenAddress, address(lcMock), ESCROW_PERIOD);
        stakeTable = StakeTableMock(proxy);
    }

    // TODO remove?
    function setUp() public {
        setUpCustom();
    }

    function test_Deployment_StoresBlockNumber() public {
        setUp();
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

    // commenting out epoch related tests for now
    // function testFuzz_RevertWhen_InvalidNextRegistrationEpoch(uint64 rand) external {
    //     LCMock.setCurrentEpoch(3);
    //     uint64 currentEpoch = stakeTable.currentEpoch();

    //     uint64 depositAmount = 10 ether;
    //     vm.prank(validator);
    //     token.approve(address(stakeTable), depositAmount);

    //     (
    //         BN254.G2Point memory blsVK,
    //         EdOnBN254.EdOnBN254Point memory schnorrVK,
    //         BN254.G1Point memory sig
    //     ) = genClientWallet(validator);

    //     // Invalid next registration epoch
    //     uint64 validUntilEpoch = uint64(bound(rand, 0, currentEpoch - 1));
    //     vm.prank(validator);
    //     vm.expectRevert(
    //         abi.encodeWithSelector(
    //             S.InvalidNextRegistrationEpoch.selector, currentEpoch + 1, validUntilEpoch
    //         )
    //     );
    //     stakeTable.registerValidator
    //         blsVK,
    //         schnorrVK,
    //         depositAmount,
    //         sig,
    //         validUntilEpoch
    //     );

    //     // Valid next registration epoch
    //     validUntilEpoch = uint64(bound(rand, currentEpoch + 1, type(uint64).max));
    //     vm.prank(validator);
    //     stakeTable.registerValidator
    //         blsVK,
    //         schnorrVK,
    //         depositAmount,
    //         sig,
    //         validUntilEpoch
    //     );
    // }

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

    // TODO: MA the contract currently doesn't allow updating the schorr key without updating the
    // BLS key. Need to discuss with product team if this is fine.
    //
    // function test_UpdateConsensusKeysWithSameBlsKeyButNewSchnorrVK_Succeeds() public {
    //     uint64 depositAmount = 10 ether;
    //     string memory seed = "123";

    //     //Step 1: generate a new blsVK and schnorrVK and register this node
    //     (
    //         BN254.G2Point memory blsVK,
    //         EdOnBN254.EdOnBN254Point memory schnorrVK,
    //         BN254.G1Point memory blsSig
    //     ) = genClientWallet(validator, seed1);

    //     // Prepare for the token transfer by granting allowance to the contract
    //     vm.startPrank(validator);
    //     token.approve(address(stakeTable), depositAmount);

    //     // Balances before registration
    //     assertEq(token.balanceOf(validator), INITIAL_BALANCE);

    //     vm.expectEmit(false, false, false, true, address(stakeTable));
    //     emit S.ValidatorRegistered(validator, blsVK, schnorrVK,
    // COMMISSION);
    //     stakeTable.registerValidator(blsVK, schnorrVK, blsSig, COMMISSION);

    //     // Step 2: generate a new schnorrVK
    //     seed = "234";
    //     (, EdOnBN254.EdOnBN254Point memory newSchnorrVK,) =
    //         genClientWallet(validator, seed1);

    //     // Step 3: update the consensus keys with the new schnorrVK
    //     vm.expectEmit(false, false, false, true, address(stakeTable));
    //     emit S.ConsensusKeysUpdated(validator, blsVK, newSchnorrVK);
    //     stakeTable.updateConsensusKeys(blsVK, newSchnorrVK, blsSig);

    //     vm.stopPrank();
    // }

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

        // Request undelegation of rest of funds
        stakeTable.undelegate(validator, 2 ether);

        // Try to undelegate more
        vm.expectRevert(abi.encodeWithSelector(S.InsufficientBalance.selector, 0));
        stakeTable.undelegate(validator, 1);

        // Withdraw after escrow period
        vm.warp(block.timestamp + ESCROW_PERIOD);
        stakeTable.claimWithdrawal(validator);
        assertEq(token.balanceOf(delegator), INITIAL_BALANCE);

        vm.stopPrank();
    }

    // solhint-disable-next-line no-empty-blocks
    function test_revertIf_undelegate_AfterValidatorExit() public {
        // TODO
    }

    // TODO: using openzeppelin contracts for this now
    // test set admin succeeds
    // function test_setAdmin_succeeds() public {
    //     vm.prank(tokenGrantRecipient);
    //     vm.expectEmit(false, false, false, true, address(stakeTable));
    //     emit Ownable.OwnershipTransferred(tokenGrantRecipient, makeAddr("admin"));
    //     stakeTable.transferOwnership(makeAddr("admin"));
    //     assertEq(stakeTable.owner(), makeAddr("admin"));
    // }

    // TODO: using openzeppelin contracts for this now
    // test set admin fails if not admin or invalid admin address
    // function test_revertWhen_setAdmin_NotAdminOrInvalidAdminAddress() public {
    //     vm.startPrank(makeAddr("randomUser"));
    //     vm.expectRevert(
    //         abi.encodeWithSelector(
    //             Ownable.OwnableUnauthorizedAccount.selector, makeAddr("randomUser")
    //         )
    //     );
    //     stakeTable.transferOwnership(makeAddr("admin"));
    //     vm.stopPrank();

    //     vm.prank(tokenGrantRecipient);
    //     vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableInvalidOwner.selector,
    // address(0)));
    //     stakeTable.transferOwnership(address(0));
    // }

    // TESTS FOR CURRENT EPOCH
    // function test_initialEpoch_isZero() public view {
    //     // assert the current block height is initialBlockHeight
    //     uint64 initialBlockHeight = 0;
    //     (, uint64 currentBlockHeight,) = lcMock.finalizedState();
    //     assertEq(currentBlockHeight, initialBlockHeight);

    //     // Calculate the expected epoch
    //     uint64 expectedEpoch = 0;

    //     // Call the currentEpoch function
    //     uint64 currentEpoch = stakeTable.currentEpoch();

    //     // Assert that the current epoch is calculated correctly
    //     assertEq(currentEpoch, expectedEpoch);
    //     assertEq(currentEpoch, 0);
    // }

    // function test_currentEpoch_isUpdated() public {
    //     test_initialEpoch_isZero();

    //     // set new finalized state on the light client contract
    //     lcMock.setFinalizedState(LightClient.LightClientState(0, 10, BN254.ScalarField.wrap(0)));

    //     // verify the current epoch is updated and is non-zero
    //     assertNotEq(stakeTable.currentEpoch(), 0);

    //     // verify the expected epoch
    //     uint64 expectedEpoch = 10; // 10 / 1
    //     assertEq(stakeTable.currentEpoch(), expectedEpoch);
    // }

    // function test_currentEpoch_blocksPerEpochNotOne() public {
    //     setUpCustom(10, /*churn*/ 3 /*blocksPerEpoch*/ );
    //     test_initialEpoch_isZero();
    //     lcMock.setFinalizedState(LightClient.LightClientState(0, 2, BN254.ScalarField.wrap(0)));
    //     assertEq(stakeTable.currentEpoch(), 0);
    //     lcMock.setFinalizedState(LightClient.LightClientState(0, 3, BN254.ScalarField.wrap(0)));
    //     assertEq(stakeTable.currentEpoch(), 1);
    // }

    // // test various edge cases for the currentEpoch
    // function test_currentEpoch_edgeCases() public {
    //     // test edge case when the block height is less than the hotShotBlocksPerEpoch
    //     uint64 hotShotBlockHeight = 0;
    //     lcMock.setFinalizedState(
    //         LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
    //     );
    //     assertEq(stakeTable.currentEpoch(), 0);

    //     // test edge case when the block height is exactly divisible by the hotShotBlocksPerEpoch
    //     hotShotBlockHeight = 1;
    //     lcMock.setFinalizedState(
    //         LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
    //     );
    //     assertEq(stakeTable.currentEpoch(), 1);

    //     // test edge case when the block height is greater than the hotShotBlocksPerEpoch
    //     hotShotBlockHeight = 2;
    //     lcMock.setFinalizedState(
    //         LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
    //     );
    //     assertEq(stakeTable.currentEpoch(), 2);

    //     // test edge case when the block height is very large
    //     hotShotBlockHeight = type(uint64).max;
    //     lcMock.setFinalizedState(
    //         LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
    //     );
    //     assertEq(stakeTable.currentEpoch(), hotShotBlockHeight / hotShotBlocksPerEpoch);
    // }

    // // TESTS FOR NEXT REGISTRATION EPOCH

    // /// @notice test the next available epoch (registration/exit) when the current epoch is zero
    // function test_nextAvailableEpoch_whenCurrentEpochIsZero() public {
    //     // test for registration
    //     // check that the current epoch is zero
    //     assertEq(stakeTable.currentEpoch(), 0);

    //     // check that the first registration epoch is equal to one
    //     assertEq(stakeTable.registrationEpoch(), 1);

    //     // assert that the next registration epoch is equal to the  registration
    //     // epoch
    //     stakeTable.mockPushToRegistrationQueue();
    //     assertEq(stakeTable.registrationEpoch(), 1);
    //     assertEq(stakeTable.numPendingRegistrationsInEpoch(), 1);

    //     // test for exit
    //     // assert that the next exit epoch is equal to the  exit epoch
    //     assertEq(stakeTable.exitEpoch(), 1);
    //     stakeTable.mockPushToExitQueue();
    //     assertEq(stakeTable.exitEpoch(), 1); // the epoch is one as you just pushed one exit
    //     assertEq(stakeTable.numPendingExitsInEpoch(), 1);
    // }

    // /// @notice test the next available epoch (registration/exit) when the current epoch + 1
    // /// is greater than the  registration/exit epoch
    // function test_nextAvailableEpoch_whenCurrentEpochPlusOneIsGreaterThanregistrationEpoch()
    //     public
    // {
    //     // test for registration
    //     // set the current epoch to 1 by updating the latest hotshot block number on the LC
    // contract
    //     lcMock.setFinalizedState(LightClient.LightClientState(0, 1, BN254.ScalarField.wrap(0)));
    //     assertEq(stakeTable.currentEpoch(), 1);

    //     // assert that the registrationEpoch is 1
    //     assertEq(stakeTable.registrationEpoch(), 1);
    //     assertGe(stakeTable.currentEpoch() + 1, stakeTable.registrationEpoch());

    //     // assert that the next registration epoch is equal to stakeTable.currentEpoch() + 1
    //     stakeTable.mockPushToRegistrationQueue();
    //     assertEq(stakeTable.registrationEpoch(), stakeTable.currentEpoch() + 1);
    //     assertEq(stakeTable.numPendingRegistrationsInEpoch(), 0);

    //     // test for exit
    //     assertEq(stakeTable.exitEpoch(), 1);

    //     // assert that the next exit epoch is equal to 2
    //     stakeTable.mockPushToExitQueue();
    //     assertEq(stakeTable.exitEpoch(), stakeTable.currentEpoch() + 1);
    //     assertEq(stakeTable.numPendingExitsInEpoch(), 0);
    // }

    // /// @notice test nextAvailableEpoch when firstAvailableEpoch (registration/exit) is greater
    // than
    // /// currentEpoch + 1
    // function test_nextAvailableEpoch_whenFirstAvailableEpochIsGreaterThanCurrentEpochPlusOne()
    //     public
    // {
    //     // set the current epoch to 1 by updating the latest hotshot block number on the LC
    // contract
    //     lcMock.setFinalizedState(LightClient.LightClientState(0, 1, BN254.ScalarField.wrap(0)));
    //     assertEq(stakeTable.currentEpoch(), 1);

    //     // set the  registration epoch to 3
    //     uint64 registrationEpoch = 3;
    //     stakeTable.setRegistrationEpoch(registrationEpoch);

    //     // assert that the next registration epoch is greater than the current epoch
    //     assertGt(stakeTable.registrationEpoch(), stakeTable.currentEpoch());

    //     // assert that the next registration epoch is equal to 3
    //     stakeTable.mockPushToRegistrationQueue();
    //     assertEq(stakeTable.registrationEpoch(), registrationEpoch);
    //     assertEq(stakeTable.numPendingRegistrationsInEpoch(), 1);

    //     // set the  registration epoch to max uint64
    //     registrationEpoch = type(uint64).max;
    //     stakeTable.setRegistrationEpoch(registrationEpoch);

    //     // assert that the next registration epoch is equal to max uint64
    //     stakeTable.mockPushToRegistrationQueue();
    //     assertEq(stakeTable.registrationEpoch(), registrationEpoch);
    //     assertEq(stakeTable.numPendingRegistrationsInEpoch(), 2);

    //     // test for exit
    //     // set the  exit epoch to 3
    //     uint64 exitEpoch = 3;
    //     stakeTable.setExitEpoch(exitEpoch);

    //     // assert that the next exit epoch is equal to 3
    //     stakeTable.mockPushToExitQueue();
    //     assertEq(stakeTable.exitEpoch(), exitEpoch);
    //     assertEq(stakeTable.numPendingExitsInEpoch(), 1);
    // }

    // /// @notice test registrationEpoch when the current epoch + 1 is equal to the
    // /// registration/exit epoch
    // function test_registrationEpoch_whenCurrentEpochPlusOneIsEqualToregistrationEpoch() public {
    //     // test for registration
    //     // set the current epoch to 1 by updating the latest hotshot block number on the LC
    // contract
    //     lcMock.setFinalizedState(LightClient.LightClientState(0, 1, BN254.ScalarField.wrap(0)));
    //     assertEq(stakeTable.currentEpoch(), 1);

    //     // set the  registration epoch to 2
    //     uint64 registrationEpoch = 2;
    //     stakeTable.setRegistrationEpoch(registrationEpoch);

    //     // assert that the next registration epoch is equal to 2
    //     stakeTable.mockPushToRegistrationQueue();
    //     assertEq(stakeTable.registrationEpoch(), registrationEpoch);
    //     assertEq(stakeTable.numPendingRegistrationsInEpoch(), 1);

    //     // test for exit
    //     // set the  exit epoch to 2
    //     uint64 exitEpoch = 2;
    //     stakeTable.setExitEpoch(exitEpoch);

    //     // assert that the next exit epoch is equal to 2
    //     stakeTable.mockPushToExitQueue();
    //     assertEq(stakeTable.exitEpoch(), exitEpoch);
    //     assertEq(stakeTable.numPendingExitsInEpoch(), 1);
    // }

    // // test pushToRegistrationQueue reverts when the current epoch is max uint64
    // // note, the current epoch is max uint64 only when the hotshot blocks per epoch is 1
    // // and the hotshot block number is max uint64
    // function test_revertWhen_pushToRegistrationQueue_whenCurrentEpochIsMaxUint64() public {
    //     // set the current hotshot block number to max uint64
    //     lcMock.setFinalizedState(
    //         LightClient.LightClientState(0, type(uint64).max, BN254.ScalarField.wrap(0))
    //     );
    //     assertEq(stakeTable.currentEpoch(), type(uint64).max);

    //     // set the hotshot blocks per epoch to 1
    //     vm.prank(tokenGrantRecipient);
    //     stakeTable.mockUpdateHotShotBlocksPerEpoch(1);
    //     assertEq(stakeTable.hotShotBlocksPerEpoch(), 1);

    //     // push to registration queue and expect a panic due to arithmetic overflow
    //     vm.expectRevert(stdError.arithmeticError);
    //     stakeTable.mockPushToRegistrationQueue();
    // }
}
