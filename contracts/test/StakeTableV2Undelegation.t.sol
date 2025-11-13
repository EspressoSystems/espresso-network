// SPDX-License-Identifier: MIT

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { PausableUpgradeable } from
    "openzeppelin-contracts-upgradeable/contracts/utils/PausableUpgradeable.sol";
import { EspToken } from "../src/EspToken.sol";

contract StakeTableV2UndelegationTest is Test {
    StakeTableUpgradeV2Test public stakeTableUpgradeTest;
    StakeTableV2 public proxy;
    EspToken public token;
    address public pauser;
    address public validator1;
    address public validator2;
    address public delegator1;
    address public delegator2;

    uint256 constant INITIAL_BALANCE = 1000 ether;
    uint256 constant ESCROW_PERIOD = 1 weeks;

    function setUp() public {
        stakeTableUpgradeTest = new StakeTableUpgradeV2Test();
        stakeTableUpgradeTest.setUp();
        pauser = makeAddr("pauser");

        validator1 = makeAddr("validator1");
        validator2 = makeAddr("validator2");
        delegator1 = makeAddr("delegator1");
        delegator2 = makeAddr("delegator2");

        vm.startPrank(stakeTableUpgradeTest.admin());
        StakeTableV2 baseProxy = StakeTableV2(address(stakeTableUpgradeTest.getStakeTable()));
        address admin = baseProxy.owner();
        StakeTableV2.InitialCommission[] memory emptyCommissions;

        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, admin, 0, emptyCommissions
        );
        baseProxy.upgradeToAndCall(address(new StakeTableV2()), initData);
        proxy = StakeTableV2(address(baseProxy));
        vm.stopPrank();

        token = stakeTableUpgradeTest.token();

        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator1, "100", 500, proxy);
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator2, "200", 500, proxy);

        address tokenGrantRecipient = stakeTableUpgradeTest.tokenGrantRecipient();
        vm.startPrank(tokenGrantRecipient);
        token.transfer(delegator1, INITIAL_BALANCE);
        token.transfer(delegator2, INITIAL_BALANCE);
        vm.stopPrank();

        vm.prank(delegator1);
        token.approve(address(proxy), type(uint256).max);
        vm.prank(delegator2);
        token.approve(address(proxy), type(uint256).max);

        vm.prank(delegator1);
        proxy.delegate(validator1, 500 ether);
        vm.prank(delegator2);
        proxy.delegate(validator2, 300 ether);
    }

    function test_UndelegationIdUniqueness() public {
        vm.prank(delegator1);
        proxy.undelegate(validator1, 100 ether);
        (uint64 id1,,) = proxy.getUndelegation(validator1, delegator1);

        vm.warp(block.timestamp + ESCROW_PERIOD);
        vm.prank(delegator1);
        proxy.claimWithdrawal(validator1);

        vm.prank(delegator1);
        proxy.undelegate(validator1, 50 ether);
        (uint64 id2,,) = proxy.getUndelegation(validator1, delegator1);

        assertEq(id1, 1, "First undelegation ID should be 1");
        assertEq(id2, 2, "Second undelegation ID should be 2");
        assertTrue(id1 != id2, "IDs should be unique");
    }

    function test_UndelegationIdIncrement() public {
        vm.prank(delegator1);
        proxy.undelegate(validator1, 100 ether);
        (uint64 id1,,) = proxy.getUndelegation(validator1, delegator1);

        vm.warp(block.timestamp + ESCROW_PERIOD);
        vm.prank(delegator1);
        proxy.claimWithdrawal(validator1);

        vm.prank(delegator2);
        proxy.undelegate(validator2, 50 ether);
        (uint64 id2,,) = proxy.getUndelegation(validator2, delegator2);

        assertEq(id1, 1, "First ID should be 1");
        assertEq(id2, 2, "Second ID should be 2");
    }

    function test_EventIncludesUndelegationId() public {
        uint256 unlocksAt = block.timestamp + ESCROW_PERIOD;

        vm.expectEmit();
        emit StakeTableV2.UndelegatedV2(delegator1, validator1, 1, 100 ether, unlocksAt);

        vm.prank(delegator1);
        proxy.undelegate(validator1, 100 ether);
        (uint64 id,,) = proxy.getUndelegation(validator1, delegator1);

        assertEq(id, 1, "Should return ID 1");
    }

    function test_ClaimEmitsCorrectId() public {
        vm.prank(delegator1);
        proxy.undelegate(validator1, 100 ether);
        (uint64 id,,) = proxy.getUndelegation(validator1, delegator1);

        vm.warp(block.timestamp + ESCROW_PERIOD);

        vm.expectEmit();
        emit StakeTableV2.WithdrawalClaimed(delegator1, validator1, id, 100 ether);

        vm.prank(delegator1);
        proxy.claimWithdrawal(validator1);
    }

    function test_SingleUndelegationLimit() public {
        vm.startPrank(delegator1);

        proxy.undelegate(validator1, 100 ether);

        vm.expectRevert(StakeTable.UndelegationAlreadyExists.selector);
        proxy.undelegate(validator1, 50 ether);

        vm.stopPrank();
    }

    function test_GetUndelegationReturnsId() public {
        vm.prank(delegator1);
        proxy.undelegate(validator1, 150 ether);
        uint256 expectedUnlocksAt = block.timestamp + ESCROW_PERIOD;

        (uint64 id, uint256 amount, uint256 unlocksAt) =
            proxy.getUndelegation(validator1, delegator1);

        assertEq(id, 1, "ID should be 1");
        assertEq(amount, 150 ether, "Amount should match");
        assertEq(unlocksAt, expectedUnlocksAt, "Unlock time should match");
    }

    function test_SecondUndelegationAfterClaim() public {
        vm.prank(delegator1);
        proxy.undelegate(validator1, 100 ether);
        (uint64 id1,,) = proxy.getUndelegation(validator1, delegator1);

        vm.warp(block.timestamp + ESCROW_PERIOD);

        vm.prank(delegator1);
        proxy.claimWithdrawal(validator1);

        vm.prank(delegator1);
        proxy.undelegate(validator1, 50 ether);

        (uint64 storedId, uint256 amount,) = proxy.getUndelegation(validator1, delegator1);

        assertEq(storedId, storedId, "Stored ID should be the new one");
        assertEq(amount, 50 ether, "Amount should match new undelegation");
        assertTrue(storedId > id1, "New ID should be greater");
    }

    function test_FullLifecycleWithIds() public {
        vm.prank(delegator1);
        proxy.undelegate(validator1, 200 ether);

        (uint64 storedId, uint256 storedAmount,) = proxy.getUndelegation(validator1, delegator1);

        assertEq(storedId, 1, "Stored ID should be 1");
        assertEq(storedAmount, 200 ether, "Stored amount should match");

        vm.warp(block.timestamp + ESCROW_PERIOD);

        vm.prank(delegator1);
        proxy.claimWithdrawal(validator1);

        vm.expectRevert(StakeTableV2.NoUndelegationFound.selector);
        vm.prank(delegator1);
        proxy.getUndelegation(validator1, delegator1);
    }

    function test_UndelegationIdGlobalIncrement() public {
        vm.prank(delegator1);
        proxy.undelegate(validator1, 100 ether);
        (uint64 id1,,) = proxy.getUndelegation(validator1, delegator1);

        vm.warp(block.timestamp + ESCROW_PERIOD);

        vm.prank(delegator1);
        proxy.claimWithdrawal(validator1);

        vm.prank(delegator2);
        proxy.undelegate(validator2, 50 ether);
        (uint64 id2,,) = proxy.getUndelegation(validator2, delegator2);

        assertEq(id1, 1, "First ID should be 1");
        assertEq(id2, 2, "Second ID should be 2");
        assertTrue(id2 > id1, "IDs should increment globally across validators");
    }

    function test_ActiveStakeTracking() public {
        uint256 activeStakeBefore = proxy.activeStake();

        vm.prank(delegator1);
        proxy.undelegate(validator1, 100 ether);

        uint256 activeStakeAfter = proxy.activeStake();
        assertEq(
            activeStakeBefore - activeStakeAfter,
            100 ether,
            "Active stake should decrease by undelegation amount"
        );
    }

    function test_RevertWhen_ClaimNonExistent() public {
        vm.expectRevert(StakeTable.NothingToWithdraw.selector);
        vm.prank(delegator1);
        proxy.claimWithdrawal(validator1);
    }

    function test_RevertWhen_ClaimBeforeUnlock() public {
        vm.startPrank(delegator1);

        proxy.undelegate(validator1, 100 ether);

        vm.expectRevert(StakeTable.PrematureWithdrawal.selector);
        proxy.claimWithdrawal(validator1);

        vm.stopPrank();
    }

    function test_claimWithdrawalAfterV1Undelegation() public {
        StakeTableUpgradeV2Test freshUpgradeTest = new StakeTableUpgradeV2Test();
        freshUpgradeTest.setUp();

        address v1Validator = makeAddr("v1Validator");
        address v1Delegator = makeAddr("v1Delegator");

        StakeTable v1StakeTable = freshUpgradeTest.getStakeTable();
        address admin = v1StakeTable.owner();
        EspToken testToken = freshUpgradeTest.token();

        freshUpgradeTest.registerValidatorOnStakeTableV1(v1Validator, "3", 500, v1StakeTable);

        address tokenGrantRecipient = freshUpgradeTest.tokenGrantRecipient();
        vm.prank(tokenGrantRecipient);
        testToken.transfer(v1Delegator, INITIAL_BALANCE);

        vm.startPrank(v1Delegator);
        testToken.approve(address(v1StakeTable), type(uint256).max);
        v1StakeTable.delegate(v1Validator, 500 ether);
        v1StakeTable.undelegate(v1Validator, 200 ether);
        vm.stopPrank();

        vm.warp(block.timestamp + ESCROW_PERIOD);

        StakeTableV2.InitialCommission[] memory emptyCommissions;
        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, admin, 0, emptyCommissions
        );
        StakeTableV2 implementation = new StakeTableV2();
        vm.prank(admin);
        v1StakeTable.upgradeToAndCall(address(implementation), initData);

        StakeTableV2 v2StakeTable = StakeTableV2(address(v1StakeTable));

        vm.expectEmit();
        // Undelegations from V1 StakeTable have ID 0
        emit StakeTableV2.WithdrawalClaimed(v1Delegator, v1Validator, 0, 200 ether);

        vm.prank(v1Delegator);
        v2StakeTable.claimWithdrawal(v1Validator);

        assertEq(
            testToken.balanceOf(v1Delegator), INITIAL_BALANCE - 300 ether, "Balance should match"
        );
    }
}
