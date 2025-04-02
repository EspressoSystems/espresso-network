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
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";

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
    string seed1 = "1";
    string seed2 = "255";

    function genClientWallet(address sender, string memory _seed)
        public
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
        (proxy, admin) = stakeTableDeployer.run(tokenAddress, address(lcMock), ESCROW_PERIOD);
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
        emit S.Staked(delegator, validator, 3 ether);
        stakeTable.delegate(validator, 3 ether);

        assertEq(token.balanceOf(delegator), INITIAL_BALANCE - 3 ether);
        assertEq(token.balanceOf(address(stakeTable)), 3 ether);

        // Withdraw from non-existent validator
        vm.expectRevert(S.NothingToWithdraw.selector);
        stakeTable.claimWithdrawal(makeAddr("nobody"));

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

        // Try to unstake after validator exit
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
    function test_revertIf_unstake_AfterValidatorExit() public {
        // TODO
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
        (uint8 majorVersion,,) = StakeTableV2Test(stakeTableRegisterTest.proxy()).getVersion();
        assertEq(majorVersion, 1);

        vm.startPrank(stakeTableRegisterTest.admin());
        address proxy = stakeTableRegisterTest.proxy();
        S(proxy).upgradeToAndCall(address(new StakeTableV2Test()), "");

        (uint8 majorVersionNew,,) = StakeTableV2Test(proxy).getVersion();
        assertEq(majorVersionNew, 2);

        assertNotEq(majorVersion, majorVersionNew);
        vm.stopPrank();
    }

    /// forge-config: default.allow_internal_expect_revert = true
    function test_upgrade_reverts_when_not_admin() public {
        address notAdmin = makeAddr("not_admin");
        S proxy = S(stakeTableRegisterTest.proxy());
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
        S proxy = S(stakeTableRegisterTest.proxy());
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        proxy.initialize(address(0), address(0), 0, address(0));
    }

    function test_initialize_function_is_protected_when_upgraded() public {
        vm.startPrank(stakeTableRegisterTest.admin());
        S proxy = S(stakeTableRegisterTest.proxy());
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
        S proxy = S(stakeTableRegisterTest.proxy());
        proxy.upgradeToAndCall(
            address(new StakeTableV2Test()), abi.encodeWithSignature("initializeV2(uint256)", 2)
        );

        StakeTableV2Test proxyV2 = StakeTableV2Test(stakeTableRegisterTest.proxy());
        assertEq(proxyV2.newValue(), 2);

        vm.expectRevert(Initializable.InvalidInitialization.selector);
        proxyV2.initializeV2(3);

        vm.stopPrank();
    }
}

contract StakeTableVotesTest is Test {
    StakeTable_register_Test internal stakeTableRegisterTest;
    S internal stakeTable;
    address internal delegator;
    address internal validator;
    address internal tokenGrantRecipient;
    uint256 internal initialBalance;
    uint16 internal commission;
    EspToken internal token;

    function setUp() public {
        stakeTableRegisterTest = new StakeTable_register_Test();
        stakeTableRegisterTest.setUp();
        stakeTable = S(stakeTableRegisterTest.proxy());
        delegator = stakeTableRegisterTest.delegator();
        validator = stakeTableRegisterTest.validator();
        initialBalance = stakeTableRegisterTest.INITIAL_BALANCE();
        tokenGrantRecipient = stakeTableRegisterTest.tokenGrantRecipient();
        token = stakeTableRegisterTest.token();
        commission = stakeTableRegisterTest.COMMISSION();
        //register validator
        stakeTableRegisterTest.test_Registration_succeeds();
    }

    function test_voting_units_are_determined_by_staked_amount_and_delegated_to_validator()
        public
    {
        vm.startPrank(tokenGrantRecipient);
        token.transfer(delegator, initialBalance);
        vm.stopPrank();

        vm.startPrank(delegator);
        token.approve(address(stakeTable), initialBalance);

        stakeTable.delegate(validator, initialBalance);
        uint256 delegatorVotes = stakeTable.getVotes(delegator);
        assertEq(delegatorVotes, 0);

        uint256 validatorVotes = stakeTable.getVotes(validator);
        assertEq(validatorVotes, initialBalance);
        //change block number
        vm.roll(block.number + 1);
        uint256 totalSupply = stakeTable.getPastTotalSupply(1);
        assertEq(totalSupply, initialBalance);

        assertEq(stakeTable.delegates(delegator), validator);
        assertEq(stakeTable.getPastVotes(validator, 0), 0);
        assertEq(stakeTable.getPastVotes(validator, 1), initialBalance);
    }

    function test_expect_revert_when_validator_stakes_to_itself() public {
        vm.startPrank(validator);

        vm.expectRevert(S.ValidatorCannotDelegate.selector);
        stakeTable.delegate(validator, 1);

        assertEq(stakeTable.delegates(validator), address(0));
        vm.roll(block.number + 1);
        assertEq(stakeTable.getPastTotalSupply(1), 0);
        assertEq(stakeTable.getPastVotes(validator, 0), 0);
        assertEq(stakeTable.getPastVotes(validator, 1), 0);

        vm.stopPrank();
    }

    function test_expect_revert_when_staker_already_staked_to_validator() public {
        vm.startPrank(tokenGrantRecipient);
        token.transfer(delegator, initialBalance);
        vm.stopPrank();

        uint256 stakeAmount = 1 ether;

        vm.startPrank(delegator);

        token.approve(address(stakeTable), 2 ether);
        stakeTable.delegate(validator, stakeAmount);

        assertEq(stakeTable.delegatorValidator(delegator), validator);
        (uint256 stakedAmount,) = stakeTable.validators(validator);
        assertEq(stakedAmount, stakeAmount);
        vm.stopPrank();

        // register another validator
        address otherValidator = makeAddr("other_validator");
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableRegisterTest.genClientWallet(otherValidator, "255");

        vm.startPrank(otherValidator);
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit S.ValidatorRegistered(otherValidator, blsVK, schnorrVK, commission);
        stakeTable.registerValidator(blsVK, schnorrVK, sig, commission);
        vm.stopPrank();

        vm.startPrank(delegator);
        vm.expectRevert(S.DelegatorAlreadyStaked.selector);
        stakeTable.delegate(otherValidator, stakeAmount);

        assertEq(stakeTable.delegates(delegator), validator);
        vm.roll(block.number + 1);
        assertEq(stakeTable.getPastTotalSupply(1), stakeAmount);
        assertEq(stakeTable.getPastVotes(validator, 0), 0);
        assertEq(stakeTable.getPastVotes(validator, 1), stakeAmount);
        assertEq(stakeTable.getPastVotes(otherValidator, 0), 0);
        assertEq(stakeTable.getPastVotes(otherValidator, 1), 0);
    }

    function test_multiple_stakes_to_the_same_validator_are_summed_up() public {
        vm.startPrank(tokenGrantRecipient);
        token.transfer(delegator, initialBalance);
        vm.stopPrank();

        uint256 stakeAmount = 1 ether;

        vm.startPrank(delegator);

        token.approve(address(stakeTable), 2 ether);

        stakeTable.delegate(validator, stakeAmount);
        vm.roll(block.number + 1);
        stakeTable.delegate(validator, stakeAmount);

        assertEq(stakeTable.delegatorValidator(delegator), validator);
        (uint256 validatorStakedAmount,) = stakeTable.validators(validator);

        uint256 totalStakedAmount = stakeAmount * 2;
        assertEq(validatorStakedAmount, totalStakedAmount);
        assertEq(stakeTable.delegations(validator, delegator), totalStakedAmount);

        assertEq(stakeTable.delegates(validator), address(0));
        vm.roll(block.number + 1);
        assertEq(stakeTable.getPastTotalSupply(block.number - 1), totalStakedAmount);
        assertEq(stakeTable.getPastVotes(validator, 0), 0);
        assertEq(stakeTable.getPastVotes(validator, block.number - 1), totalStakedAmount);

        vm.stopPrank();
    }

    function test_unstake_partially_adjusts_voting_power_but_staker_still_delegated_to_validator()
        public
    {
        // stake
        test_voting_units_are_determined_by_staked_amount_and_delegated_to_validator();
        assertEq(block.number, 2);

        vm.startPrank(delegator);
        uint256 unStakeAmount = 1 ether;
        uint256 votingPowerBefore = stakeTable.getVotes(validator);
        uint256 totalStakedAmount = stakeTable.delegations(validator, delegator);
        assertEq(votingPowerBefore, totalStakedAmount);

        // unstake at the next block
        vm.roll(block.number + 1);
        stakeTable.undelegate(validator, unStakeAmount);
        uint256 votingPowerAfter = stakeTable.getVotes(validator);
        assertEq(votingPowerAfter, totalStakedAmount - unStakeAmount);

        uint256 blockNumAtUnstake = block.number;
        vm.roll(blockNumAtUnstake + 1);

        // verify that the total supply decreased after block 1 by the unstaked amount
        assertEq(stakeTable.getPastTotalSupply(1), totalStakedAmount);
        assertEq(stakeTable.getPastTotalSupply(blockNumAtUnstake), votingPowerAfter);

        // verify that the delegator is still delegated to the validator
        assertEq(stakeTable.delegatorValidator(delegator), validator);
        assertEq(stakeTable.delegates(delegator), validator);

        // verify that the validator has the current staked amount after the partial undelegation
        (uint256 stakedAmount,) = stakeTable.validators(validator);
        assertEq(stakedAmount, stakeTable.getVotes(validator));

        //verify that the voting power decreased at various block numbers
        assertEq(stakeTable.getPastVotes(validator, 0), 0);
        assertEq(stakeTable.getPastVotes(validator, blockNumAtUnstake - 1), totalStakedAmount);
        assertEq(
            stakeTable.getPastVotes(validator, blockNumAtUnstake), totalStakedAmount - unStakeAmount
        );

        vm.stopPrank();
    }

    function test_unstake_fully_removes_all_voting_power_for_relative_to_that_staker() public {
        // stake
        test_voting_units_are_determined_by_staked_amount_and_delegated_to_validator();
        assertEq(block.number, 2);

        vm.startPrank(delegator);
        uint256 votingPowerBefore = stakeTable.getVotes(validator);
        assertEq(votingPowerBefore, initialBalance);
        uint256 unStakeAmount = initialBalance;

        // unstake at block number 2
        stakeTable.undelegate(validator, unStakeAmount);
        uint256 votingPowerAfter = stakeTable.getVotes(validator);
        assertEq(votingPowerAfter, 0);
        assertEq(stakeTable.delegations(validator, delegator), 0);
        (uint256 stakedAmount,) = stakeTable.validators(validator);
        assertEq(stakedAmount, 0);
        vm.roll(block.number + 1);

        // at block number 1 all tokens were staked to the validator
        uint256 totalSupply = stakeTable.getPastTotalSupply(1);
        assertEq(totalSupply, initialBalance);

        // at block number 2, un_stake_amount was unstaked from the validator
        totalSupply = stakeTable.getPastTotalSupply(2);
        assertEq(totalSupply, votingPowerAfter);

        assertEq(stakeTable.delegates(delegator), address(0));
        assertEq(stakeTable.delegatorValidator(delegator), address(0));
        assertEq(stakeTable.delegations(validator, delegator), 0);
        (uint256 delegatedAmount,) = stakeTable.validators(validator);
        assertEq(delegatedAmount, 0);
        assertEq(stakeTable.getVotes(validator), 0);
        assertEq(stakeTable.getPastVotes(validator, 1), initialBalance);
        assertEq(stakeTable.getPastVotes(validator, 2), 0);

        vm.stopPrank();
    }

    function test_delegate_all_tokens_to_validator() public {
        vm.startPrank(tokenGrantRecipient);
        token.transfer(delegator, initialBalance);
        vm.stopPrank();

        vm.startPrank(delegator);
        token.approve(address(stakeTable), initialBalance);
        stakeTable.delegate(validator);
        vm.stopPrank();

        vm.roll(block.number + 1);

        assertEq(stakeTable.delegatorValidator(delegator), validator);
        assertEq(stakeTable.delegates(delegator), validator);
        assertEq(stakeTable.delegations(validator, delegator), initialBalance);
        assertEq(stakeTable.getVotes(validator), initialBalance);
        assertEq(stakeTable.getPastVotes(validator, 0), 0);
        assertEq(stakeTable.getPastTotalSupply(0), 0);
        assertEq(stakeTable.getPastVotes(validator, 1), initialBalance);
        assertEq(stakeTable.getPastTotalSupply(1), initialBalance);
    }

    function test_undelegate_all_tokens_from_validator() public {
        test_delegate_all_tokens_to_validator();
        uint256 totalDelegatedAmount = stakeTable.delegations(validator, delegator);

        vm.roll(block.number + 1);

        uint256 undelegateBlockNumber = block.number;
        vm.startPrank(delegator);
        stakeTable.undelegate(validator);
        vm.stopPrank();

        vm.roll(block.number + 1);

        assertEq(stakeTable.delegatorValidator(delegator), address(0));
        assertEq(stakeTable.delegates(delegator), address(0));
        assertEq(stakeTable.delegations(validator, delegator), 0);
        assertEq(stakeTable.getVotes(validator), 0);
        assertEq(stakeTable.getPastVotes(validator, undelegateBlockNumber), 0);
        assertEq(stakeTable.getPastTotalSupply(undelegateBlockNumber), 0);
        assertEq(
            stakeTable.getPastVotes(validator, undelegateBlockNumber - 1), totalDelegatedAmount
        );
        assertEq(stakeTable.getPastTotalSupply(undelegateBlockNumber - 1), totalDelegatedAmount);
    }

    // todo add fuzz tests
}
