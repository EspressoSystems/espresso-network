// SPDX-License-Identifier: MIT

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { EspToken } from "../src/EspToken.sol";

contract StakeTableV2DelegationTest is Test {
    StakeTableUpgradeV2Test public stakeTableUpgradeTest;
    StakeTableV2 public proxy;
    EspToken public token;
    address public pauser;
    address public validator;
    address public delegator;

    uint256 constant INITIAL_BALANCE = 1000 ether;

    function setUp() public {
        stakeTableUpgradeTest = new StakeTableUpgradeV2Test();
        stakeTableUpgradeTest.setUp();
        pauser = makeAddr("pauser");
        validator = makeAddr("validator");
        delegator = makeAddr("delegator");

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

        address tokenGrantRecipient = stakeTableUpgradeTest.tokenGrantRecipient();
        vm.prank(tokenGrantRecipient);
        token.transfer(delegator, INITIAL_BALANCE);

        vm.prank(delegator);
        token.approve(address(proxy), type(uint256).max);
    }

    function test_RevertWhen_DelegateToZeroAddress() public {
        vm.prank(delegator);
        vm.expectRevert(StakeTable.ValidatorInactive.selector);
        proxy.delegate(address(0), 1 ether);
    }

    function test_RevertWhen_DelegateToNonExistentValidator() public {
        vm.prank(delegator);
        vm.expectRevert(StakeTable.ValidatorInactive.selector);
        proxy.delegate(makeAddr("non-existent"), 1 ether);
    }

    function test_RevertWhen_DelegateToExitedValidator() public {
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);

        vm.prank(delegator);
        proxy.delegate(validator, 100 ether);

        vm.prank(validator);
        proxy.deregisterValidator();

        vm.prank(delegator);
        vm.expectRevert(StakeTable.ValidatorAlreadyExited.selector);
        proxy.delegate(validator, 100 ether);
    }

    function test_RevertWhen_DelegateZeroAmount() public {
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);

        vm.prank(delegator);
        vm.expectRevert(StakeTable.ZeroAmount.selector);
        proxy.delegate(validator, 0);
    }

    function test_RevertWhen_DelegateInsufficientAllowance() public {
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);

        address delegator2 = makeAddr("delegator2");
        address tokenGrantRecipient = stakeTableUpgradeTest.tokenGrantRecipient();
        vm.prank(tokenGrantRecipient);
        token.transfer(delegator2, INITIAL_BALANCE);

        vm.prank(delegator2);
        vm.expectRevert(
            abi.encodeWithSelector(StakeTable.InsufficientAllowance.selector, 0, 100 ether)
        );
        proxy.delegate(validator, 100 ether);
    }

    function test_RevertWhen_DelegateTokenTransferFails() public {
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);

        address delegator2 = makeAddr("delegator2");
        vm.prank(delegator2);
        token.approve(address(proxy), type(uint256).max);

        vm.prank(delegator2);
        vm.expectRevert("TRANSFER_FROM_FAILED");
        proxy.delegate(validator, 100 ether);
    }

    function assertDelegateSuccess(address _delegator, address _validator, uint256 amount)
        internal
    {
        uint256 delegatorBalanceBefore = token.balanceOf(_delegator);
        uint256 activeStakeBefore = proxy.activeStake();
        (uint256 delegatedAmountBefore,) = proxy.validators(_validator);
        uint256 delegationBefore = proxy.delegations(_validator, _delegator);

        vm.expectEmit();
        emit StakeTable.Delegated(_delegator, _validator, amount);

        vm.prank(_delegator);
        proxy.delegate(_validator, amount);

        assertEq(token.balanceOf(_delegator), delegatorBalanceBefore - amount);
        (uint256 delegatedAmountAfter,) = proxy.validators(_validator);
        assertEq(delegatedAmountAfter, delegatedAmountBefore + amount);
        assertEq(proxy.delegations(_validator, _delegator), delegationBefore + amount);
        assertEq(proxy.activeStake(), activeStakeBefore + amount);
    }

    function test_Delegate_Success() public {
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);
        assertDelegateSuccess(delegator, validator, 100 ether);
    }

    function test_Delegate_MultipleDelegations() public {
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);

        assertDelegateSuccess(delegator, validator, 100 ether);
        assertDelegateSuccess(delegator, validator, 50 ether);
    }

    function test_Delegate_ToMultipleValidators() public {
        address validator2 = makeAddr("validator2");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator2, "200", 500, proxy);

        assertDelegateSuccess(delegator, validator, 100 ether);
        assertDelegateSuccess(delegator, validator2, 200 ether);
    }

    function test_Delegate_MultipleDelegatorsToOneValidator() public {
        address delegator2 = makeAddr("delegator2");
        address tokenGrantRecipient = stakeTableUpgradeTest.tokenGrantRecipient();
        vm.prank(tokenGrantRecipient);
        token.transfer(delegator2, INITIAL_BALANCE);
        vm.prank(delegator2);
        token.approve(address(proxy), type(uint256).max);

        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "100", 500, proxy);

        assertDelegateSuccess(delegator, validator, 100 ether);
        assertDelegateSuccess(delegator2, validator, 200 ether);
    }
}
