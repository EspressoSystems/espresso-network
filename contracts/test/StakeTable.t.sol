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
import { LightClientCommonTest } from "./LightClientV2.t.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";

// Token contract
import { EspToken } from "../src/EspToken.sol";

// Target contract
import { StakeTable as S } from "../src/StakeTable.sol";

// TODO: currently missing several tests
// TODO: test only owner methods access control

contract StakeTable_register_Test is LightClientCommonTest {
    S public stakeTable;
    // NOTE: address admin defined in LightClientCommonTest
    EspToken public token;
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
            uint256 schnorrVKy
        ) = abi.decode(result, (BN254.G1Point, BN254.G2Point, uint256, uint256));

        return (
            blsVK, // blsVK
            EdOnBN254.EdOnBN254Point(schnorrVKx, schnorrVKy), // schnorrVK
            blsSig // sig
        );
    }

    function setUp() public {
        init(); // light client init
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        validator = makeAddr("validator");
        delegator = makeAddr("delegator");

        // deploy EspToken and its proxy
        EspToken tokenImpl = new EspToken();
        bytes memory initData =
            abi.encodeWithSignature("initialize(address,address)", admin, tokenGrantRecipient);
        ERC1967Proxy proxy = new ERC1967Proxy(address(tokenImpl), initData);
        token = EspToken(payable(address(proxy)));

        // transfer minted coin
        vm.prank(tokenGrantRecipient);
        token.transfer(address(validator), INITIAL_BALANCE);

        // deploy stake table and its proxy
        S staketableImpl = new S();
        initData = abi.encodeWithSignature(
            "initialize(address,address,uint256,address)",
            address(token),
            address(lc),
            ESCROW_PERIOD,
            admin
        );
        proxy = new ERC1967Proxy(address(staketableImpl), initData);
        stakeTable = S(payable(address(proxy)));
    }

    function test_Deployment_StoresBlockNumber() public view {
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

        // Delegating zero amount fails
        vm.expectRevert(S.ZeroAmount.selector);
        stakeTable.delegate(validator, 0);

        // Delegate some funds
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.Delegated(delegator, validator, 3 ether);
        stakeTable.delegate(validator, 3 ether);

        assertEq(token.balanceOf(delegator), INITIAL_BALANCE - 3 ether);
        assertEq(token.balanceOf(address(stakeTable)), 3 ether);

        // Withdraw from non-existent validator fails
        vm.expectRevert(S.NothingToWithdraw.selector);
        stakeTable.claimWithdrawal(makeAddr("nobody"));

        // Withdraw without undelegation fails
        vm.expectRevert(S.NothingToWithdraw.selector);
        stakeTable.claimWithdrawal(validator);

        // Undelegating zero amount fails
        vm.expectRevert(S.ZeroAmount.selector);
        stakeTable.undelegate(validator, 0);

        // Partial undelegation of funds ok
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.Undelegated(delegator, validator, 1 ether);
        stakeTable.undelegate(validator, 1 ether);

        // Withdraw too early fails
        vm.expectRevert(S.PrematureWithdrawal.selector);
        stakeTable.claimWithdrawal(validator);

        // Withdraw after escrow period ok
        vm.warp(block.timestamp + ESCROW_PERIOD);
        stakeTable.claimWithdrawal(validator);
        assertEq(token.balanceOf(delegator), INITIAL_BALANCE - 2 ether);

        vm.stopPrank();

        // Validator exit succeeds
        vm.prank(validator);
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorExit(validator);
        stakeTable.deregisterValidator();

        vm.startPrank(delegator);

        // Claim validator exit too early fails
        vm.expectRevert(S.PrematureWithdrawal.selector);
        stakeTable.claimValidatorExit(validator);

        // Undelegate after validator exit fails
        vm.expectRevert(S.ValidatorInactive.selector);
        stakeTable.undelegate(validator, 1);

        // Claim validator exit after escrow period ok
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

    function test_multiple_undelegations_after_exit_epoch_succeeds() public {
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

        (uint256 delegatedAmountBefore,) = stakeTable.validators(validator);
        stakeTable.undelegate(validator, 2 ether);
        (uint256 delegatedAmountAfter,) = stakeTable.validators(validator);
        assertEq(delegatedAmountAfter, delegatedAmountBefore - 2 ether);

        vm.expectRevert(S.UndelegationAlreadyExists.selector);
        stakeTable.undelegate(validator, 1 ether);

        // can't undelegate until the previous undelegation is withdrawn
        vm.warp(block.timestamp + ESCROW_PERIOD);
        vm.expectRevert(S.UndelegationAlreadyExists.selector);
        stakeTable.undelegate(validator, 1 ether);

        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.Withdrawal(delegator, 2 ether);
        stakeTable.claimWithdrawal(validator);
        assertEq(token.balanceOf(delegator), INITIAL_BALANCE - 3 ether + 2 ether);

        assertEq(stakeTable.delegations(validator, delegator), 1 ether);

        // now the delegator can undelegate again
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.Undelegated(delegator, validator, 1 ether);
        stakeTable.undelegate(validator, 1 ether);

        assertEq(stakeTable.delegations(validator, delegator), 0);
        (uint256 amountUndelegated, uint256 unlocksAt) =
            stakeTable.undelegations(validator, delegator);
        assertEq(amountUndelegated, 1 ether);
        assertEq(unlocksAt, block.timestamp + ESCROW_PERIOD);
        assertEq(token.balanceOf(address(stakeTable)), 1 ether);

        vm.expectRevert(S.PrematureWithdrawal.selector);
        stakeTable.claimWithdrawal(validator);
        (amountUndelegated, unlocksAt) = stakeTable.undelegations(validator, delegator);
        assertEq(amountUndelegated, 1 ether);
        assertEq(unlocksAt, block.timestamp + ESCROW_PERIOD);

        vm.warp(block.timestamp + ESCROW_PERIOD);
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.Withdrawal(delegator, 1 ether);
        stakeTable.claimWithdrawal(validator);

        assertEq(token.balanceOf(delegator), INITIAL_BALANCE);
        assertEq(token.balanceOf(address(stakeTable)), 0);

        vm.stopPrank();
    }
}

contract StakeTableV2Test is S {
    uint256 public newValue;

    function initializeV2(uint256 _newValue) public reinitializer(2) {
        newValue = _newValue;
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
}

contract StakeTableMissingFieldTest is Test {
    struct Validator {
        uint256 delegatedAmount;
        ValidatorStatus status;
    }

    enum ValidatorStatus {
        Unknown,
        Active,
        Exited
    }

    struct Undelegation {
        uint256 amount;
        uint256 unlocksAt;
    }

    LightClient public lightClient;
    ERC20 public token;
    mapping(address account => Validator validator) public validators;
    mapping(bytes32 blsKeyHash => bool used) public blsKeys;
    mapping(address validator => uint256 unlocksAt) public validatorExits;
    mapping(address validator => mapping(address delegator => uint256 amount)) delegations;
    mapping(address validator => mapping(address delegator => Undelegation)) undelegations;
    // missing field: exitEscrowPeriod
}

contract StakeTableFieldsReorderedTest is Test {
    struct Validator {
        uint256 delegatedAmount;
        ValidatorStatus status;
    }

    enum ValidatorStatus {
        Unknown,
        Active,
        Exited
    }

    struct Undelegation {
        uint256 amount;
        uint256 unlocksAt;
    }

    ERC20 public token;
    mapping(address account => Validator validator) public validators;
    mapping(bytes32 blsKeyHash => bool used) public blsKeys;
    mapping(address validator => uint256 unlocksAt) public validatorExits;
    mapping(address validator => mapping(address delegator => uint256 amount)) delegations;
    mapping(address validator => mapping(address delegator => Undelegation)) undelegations;
    uint256 exitEscrowPeriod;
    LightClient public lightClient; //re-ordered field
}

contract StakeTableUpgradeTest is Test {
    StakeTable_register_Test stakeTableRegisterTest;

    function setUp() public {
        stakeTableRegisterTest = new StakeTable_register_Test();
        stakeTableRegisterTest.setUp();
    }

    function test_upgrade_succeeds() public {
        (uint8 majorVersion,,) = S(stakeTableRegisterTest.stakeTable()).getVersion();
        assertEq(majorVersion, 1);

        vm.startPrank(stakeTableRegisterTest.admin());
        address proxy = address(stakeTableRegisterTest.stakeTable());
        S(proxy).upgradeToAndCall(address(new StakeTableV2Test()), "");

        (uint8 majorVersionNew,,) = StakeTableV2Test(proxy).getVersion();
        assertEq(majorVersionNew, 2);

        assertNotEq(majorVersion, majorVersionNew);
        vm.stopPrank();
    }

    /// forge-config: default.allow_internal_expect_revert = true
    function test_upgrade_reverts_when_not_admin() public {
        address notAdmin = makeAddr("not_admin");
        S proxy = stakeTableRegisterTest.stakeTable();
        (uint8 majorVersion,,) = proxy.getVersion();
        assertEq(majorVersion, 1);

        vm.startPrank(notAdmin);

        address impl = address(new StakeTableV2Test());
        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, notAdmin)
        );

        proxy.upgradeToAndCall(impl, "");

        (uint8 majorVersionNew,,) = proxy.getVersion();
        assertEq(majorVersionNew, 1);

        assertEq(majorVersion, majorVersionNew);
        vm.stopPrank();
    }

    function test_initialize_function_is_protected() public {
        S proxy = stakeTableRegisterTest.stakeTable();
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        proxy.initialize(address(0), address(0), 0, address(0));
    }

    function test_initialize_function_is_protected_when_upgraded() public {
        vm.startPrank(stakeTableRegisterTest.admin());
        S proxy = stakeTableRegisterTest.stakeTable();
        proxy.upgradeToAndCall(address(new StakeTableV2Test()), "");

        vm.expectRevert(Initializable.InvalidInitialization.selector);
        proxy.initialize(address(0), address(0), 0, address(0));

        vm.stopPrank();
    }

    function test_storage_layout_is_compatible() public {
        string[] memory cmds = new string[](4);
        cmds[0] = "node";
        cmds[1] = "contracts/test/script/compare-storage-layout.js";
        cmds[2] = "StakeTable";
        cmds[3] = "StakeTableV2Test";

        bytes memory output = vm.ffi(cmds);
        string memory result = string(output);

        assertEq(result, "true");
    }

    function test_storage_layout_is_incompatible_if_field_is_missing() public {
        string[] memory cmds = new string[](4);
        cmds[0] = "node";
        cmds[1] = "contracts/test/script/compare-storage-layout.js";
        cmds[2] = "StakeTable";
        cmds[3] = "StakeTableMissingFieldTest";

        bytes memory output = vm.ffi(cmds);
        string memory result = string(output);

        assertEq(result, "false");
    }

    function test_storage_layout_is_incompatible_if_fields_are_reordered() public {
        string[] memory cmds = new string[](4);
        cmds[0] = "node";
        cmds[1] = "contracts/test/script/compare-storage-layout.js";
        cmds[2] = "StakeTable";
        cmds[3] = "StakeTableFieldsReorderedTest";

        bytes memory output = vm.ffi(cmds);
        string memory result = string(output);

        assertEq(result, "false");
    }

    function test_storage_layout_is_incompatible_between_diff_contracts() public {
        string[] memory cmds = new string[](4);
        cmds[0] = "node";
        cmds[1] = "contracts/test/script/compare-storage-layout.js";
        cmds[2] = "StakeTable";
        cmds[3] = "LightClient";

        bytes memory output = vm.ffi(cmds);
        string memory result = string(output);

        assertEq(result, "false");
    }

    function test_reinitialize_succeeds_only_once() public {
        vm.startPrank(stakeTableRegisterTest.admin());
        S proxy = stakeTableRegisterTest.stakeTable();
        proxy.upgradeToAndCall(
            address(new StakeTableV2Test()), abi.encodeWithSignature("initializeV2(uint256)", 2)
        );

        StakeTableV2Test proxyV2 = StakeTableV2Test(address(stakeTableRegisterTest.stakeTable()));
        assertEq(proxyV2.newValue(), 2);

        vm.expectRevert(Initializable.InvalidInitialization.selector);
        proxyV2.initializeV2(3);

        vm.stopPrank();
    }
}
