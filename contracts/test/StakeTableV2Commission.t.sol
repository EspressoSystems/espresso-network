// SPDX-License-Identifier: MIT

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { PausableUpgradeable } from
    "openzeppelin-contracts-upgradeable/contracts/utils/PausableUpgradeable.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";
import { StakeTable as S } from "../src/StakeTable.sol";

contract StakeTableV2CommissionTest is Test {
    StakeTableUpgradeV2Test public stakeTableUpgradeTest;
    StakeTableV2 public proxy;
    address public pauser;

    function setUp() public {
        stakeTableUpgradeTest = new StakeTableUpgradeV2Test();
        stakeTableUpgradeTest.setUp();
        pauser = makeAddr("pauser");

        // Upgrade to V2
        vm.startPrank(stakeTableUpgradeTest.admin());
        S baseProxy = stakeTableUpgradeTest.getStakeTable();
        address admin = baseProxy.owner();
        StakeTableV2.InitialCommission[] memory emptyCommissions;
        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, admin, 0, emptyCommissions
        );
        baseProxy.upgradeToAndCall(address(new StakeTableV2()), initData);
        proxy = StakeTableV2(address(baseProxy));
        vm.stopPrank();
    }

    function test_CommissionUpdate_Success() public {
        address validator = makeAddr("validator");
        uint16 initialCommission = 500; // 5%
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(
            validator, "123", initialCommission, proxy
        );

        vm.startPrank(validator);

        uint16 newCommission = initialCommission + proxy.maxCommissionIncrease();
        vm.expectEmit(true, false, false, true);
        emit StakeTableV2.CommissionUpdated(
            validator, block.timestamp, initialCommission, newCommission
        );
        proxy.updateCommission(newCommission);

        // Wait until the time limit expires and increase again
        vm.warp(block.timestamp + proxy.minCommissionIncreaseInterval() + 1);
        uint16 thirdCommission = newCommission + proxy.maxCommissionIncrease();
        vm.expectEmit(true, false, false, true);
        emit StakeTableV2.CommissionUpdated(
            validator, block.timestamp, newCommission, thirdCommission
        );
        proxy.updateCommission(thirdCommission);
        vm.stopPrank();
    }

    function test_CommissionUpdate_RevertWhenExceedsMax() public {
        address validator = makeAddr("validator");
        uint16 initialCommission = 500; // 5%
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(
            validator, "123", initialCommission, proxy
        );

        vm.startPrank(validator);

        vm.warp(block.timestamp + proxy.minCommissionIncreaseInterval() + 1);

        uint16 tooHighCommission = initialCommission + proxy.maxCommissionIncrease() + 1;
        vm.expectRevert(StakeTableV2.CommissionIncreaseExceedsMax.selector);
        proxy.updateCommission(tooHighCommission);
        vm.stopPrank();
    }

    function test_CommissionUpdate_RevertWhenTooSoon() public {
        address validator = makeAddr("validator");
        uint16 initialCommission = 500; // 5%
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(
            validator, "123", initialCommission, proxy
        );

        vm.startPrank(validator);

        // First update should succeed immediately
        uint16 firstUpdate = 600; // 6%
        proxy.updateCommission(firstUpdate);

        // Try to update again immediately (too soon) - this should fail
        uint16 secondUpdate = 700; // 7%
        vm.expectRevert(StakeTableV2.CommissionUpdateTooSoon.selector);
        proxy.updateCommission(secondUpdate);
        vm.stopPrank();
    }

    function test_CommissionUpdate_RevertWhenValidatorInactive() public {
        // Try to update commission without being registered
        address validator = makeAddr("validator");
        vm.startPrank(validator);
        vm.expectRevert(S.ValidatorInactive.selector);
        proxy.updateCommission(1000);
        vm.stopPrank();
    }

    function test_CommissionUpdate_RevertWhenValidatorExited() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);

        // Validator exits
        vm.startPrank(validator);
        proxy.deregisterValidator();

        // Try to update commission after exit
        vm.expectRevert(S.ValidatorAlreadyExited.selector);
        proxy.updateCommission(1000);
        vm.stopPrank();
    }

    function test_CommissionUpdate_RevertWhenPaused() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);

        // Pause the contract
        vm.prank(pauser);
        proxy.pause();

        // Try to update commission while paused
        vm.startPrank(validator);
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        proxy.updateCommission(1000);
        vm.stopPrank();
    }

    function test_CommissionUpdate_DecreaseMaxDelta() public {
        address validator = makeAddr("validator");
        uint16 maxCommission = proxy.MAX_COMMISSION_BPS();
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(
            validator, "123", maxCommission, proxy
        );

        vm.startPrank(validator);

        vm.expectEmit(true, false, false, true);
        uint16 minCommission = 0;
        emit StakeTableV2.CommissionUpdated(
            validator, block.timestamp, maxCommission, minCommission
        );
        proxy.updateCommission(minCommission);
        vm.stopPrank();
    }

    function test_SetMinCommissionUpdateInterval_Success() public {
        uint256 newInterval = 14 days;

        vm.startPrank(stakeTableUpgradeTest.admin());
        vm.expectEmit(true, false, false, true);
        emit StakeTableV2.MinCommissionUpdateIntervalUpdated(newInterval);
        proxy.setMinCommissionUpdateInterval(newInterval);

        assertEq(proxy.minCommissionIncreaseInterval(), newInterval);
        vm.stopPrank();
    }

    function test_SetMaxCommissionIncrease_Success() public {
        uint16 newMaxIncrease = 1000; // 10%

        vm.startPrank(stakeTableUpgradeTest.admin());
        vm.expectEmit(true, false, false, true);
        emit StakeTableV2.MaxCommissionIncreaseUpdated(newMaxIncrease);
        proxy.setMaxCommissionIncrease(newMaxIncrease);

        assertEq(proxy.maxCommissionIncrease(), newMaxIncrease);
        vm.stopPrank();
    }

    function test_SetMinCommissionUpdateInterval_RevertWhenNotAdmin() public {
        address notAdmin = makeAddr("notAdmin");
        uint256 newInterval = 14 days;
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, notAdmin, adminRole
            )
        );
        proxy.setMinCommissionUpdateInterval(newInterval);
        vm.stopPrank();
    }

    function test_SetMaxCommissionIncrease_RevertWhenNotAdmin() public {
        address notAdmin = makeAddr("notAdmin");
        uint16 newMaxIncrease = 1000;
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, notAdmin, adminRole
            )
        );
        proxy.setMaxCommissionIncrease(newMaxIncrease);
        vm.stopPrank();
    }

    function test_DefaultValues() public view {
        // This is the only test that checks default values - if defaults change, only this test
        // should fail
        assertEq(proxy.minCommissionIncreaseInterval(), 7 days);
        assertEq(proxy.maxCommissionIncrease(), 500);
        assertEq(proxy.MAX_COMMISSION_BPS(), 10000);
    }

    function test_InitializeV2_RevertWhenInitialValidatorNotRegistered() public {
        StakeTableUpgradeV2Test upgradeTest = new StakeTableUpgradeV2Test();
        upgradeTest.setUp();
        S baseProxy = upgradeTest.getStakeTable();

        address validator = makeAddr("validator");
        // validator does not register

        StakeTableV2.InitialCommission[] memory wrongCommissions =
            new StakeTableV2.InitialCommission[](1);
        wrongCommissions[0] =
            StakeTableV2.InitialCommission({ validator: validator, commission: 500 });

        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, upgradeTest.admin(), 0, wrongCommissions
        );

        vm.startPrank(upgradeTest.admin());
        StakeTableV2 implV2 = new StakeTableV2();
        vm.expectRevert(abi.encodeWithSelector(S.ValidatorInactive.selector));
        baseProxy.upgradeToAndCall(address(implV2), initData);
        vm.stopPrank();
    }

    function test_InitializeV2_RevertWhenDuplicateValidator() public {
        StakeTableUpgradeV2Test upgradeTest = new StakeTableUpgradeV2Test();
        upgradeTest.setUp();
        S baseProxy = upgradeTest.getStakeTable();

        address validator = makeAddr("validator");
        upgradeTest.registerValidatorOnStakeTableV1(validator, "123", 500, baseProxy);

        StakeTableV2.InitialCommission[] memory duplicateCommissions =
            new StakeTableV2.InitialCommission[](2);
        duplicateCommissions[0] =
            StakeTableV2.InitialCommission({ validator: validator, commission: 500 });
        // Duplicate validator
        duplicateCommissions[1] =
            StakeTableV2.InitialCommission({ validator: validator, commission: 500 });

        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, upgradeTest.admin(), 0, duplicateCommissions
        );

        vm.startPrank(upgradeTest.admin());
        StakeTableV2 implV2 = new StakeTableV2();
        vm.expectRevert(
            abi.encodeWithSelector(StakeTableV2.CommissionAlreadyInitialized.selector, validator)
        );
        baseProxy.upgradeToAndCall(address(implV2), initData);
        vm.stopPrank();
    }

    function test_UndelegatedV2_EmitsEventWithUnlocksAt() public {
        address validator = makeAddr("validator");
        address delegator = makeAddr("delegator");
        uint256 delegateAmount = 1 ether;

        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);

        deal(address(stakeTableUpgradeTest.token()), delegator, delegateAmount);

        vm.startPrank(delegator);
        stakeTableUpgradeTest.token().approve(address(proxy), delegateAmount);
        proxy.delegate(validator, delegateAmount);

        uint256 expectedUnlocksAt = block.timestamp + proxy.exitEscrowPeriod();
        vm.expectEmit();
        emit StakeTableV2.UndelegatedV2(delegator, validator, 1, delegateAmount, expectedUnlocksAt);
        proxy.undelegate(validator, delegateAmount);
        vm.stopPrank();
    }

    function test_ValidatorExitV2_EmitsEventWithUnlocksAt() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);

        vm.startPrank(validator);
        uint256 expectedUnlocksAt = block.timestamp + proxy.exitEscrowPeriod();
        vm.expectEmit();
        emit StakeTableV2.ValidatorExitV2(validator, expectedUnlocksAt);
        proxy.deregisterValidator();
        vm.stopPrank();
    }

    // TEST:st-commission-over-max-fails - tries commission = 10001 (above MAX_COMMISSION_BPS)
    function test_CommissionUpdate_RevertsAboveMaxBps() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);
        vm.prank(validator);
        vm.expectRevert(S.InvalidCommission.selector);
        proxy.updateCommission(10001);
    }

    // TEST:st-commission-unchanged-fails - tries setting to same value
    function test_CommissionUpdate_RevertsWhenUnchanged() public {
        address validator = makeAddr("validator");
        uint16 commission = 500;
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", commission, proxy);
        vm.prank(validator);
        vm.expectRevert(StakeTableV2.CommissionUnchanged.selector);
        proxy.updateCommission(commission);
    }

    // TEST:st-commission-decrease-free-ok - decrease immediately after increase (no rate limit)
    function test_CommissionUpdate_DecreaseSkipsRateLimit() public {
        address validator = makeAddr("validator");
        uint16 initial = 500;
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", initial, proxy);
        vm.startPrank(validator);
        // increase
        proxy.updateCommission(initial + proxy.maxCommissionIncrease());
        // decrease immediately (no warp) - should succeed
        proxy.updateCommission(initial);
        vm.stopPrank();
    }

    // TEST:st-commission-timestamp-ok - assert stored timestamp after increase
    function test_CommissionUpdate_StoresTimestamp() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);
        vm.warp(1000);
        vm.prank(validator);
        proxy.updateCommission(600);
        (uint16 storedCommission, uint256 storedTime) = proxy.commissionTracking(validator);
        assertEq(storedCommission, 600);
        assertEq(storedTime, 1000);
    }

    // TEST:st-commission-first-increase-ok - first increase always allowed (lastIncreaseTime=0)
    function test_CommissionUpdate_FirstIncreaseAlwaysAllowed() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);
        // No warp - first increase should be allowed because lastIncreaseTime == 0
        vm.prank(validator);
        proxy.updateCommission(600);
        (uint16 storedCommission,) = proxy.commissionTracking(validator);
        assertEq(storedCommission, 600);
    }

    // TEST:st-commission-exact-10000-ok - commission exactly at MAX_COMMISSION_BPS is allowed
    function test_CommissionUpdate_ExactMaxBpsAllowed() public {
        address validator = makeAddr("validator");
        // Register at 9600, then increase by max 500 to 10000
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 9600, proxy);
        vm.prank(validator);
        proxy.updateCommission(10000);
        (uint16 storedCommission,) = proxy.commissionTracking(validator);
        assertEq(storedCommission, 10000);
    }

    // TEST:st-set-interval-zero-fails
    function test_SetMinCommissionUpdateInterval_RevertsWhenZero() public {
        vm.prank(stakeTableUpgradeTest.admin());
        vm.expectRevert(StakeTableV2.InvalidRateLimitParameters.selector);
        proxy.setMinCommissionUpdateInterval(0);
    }

    // TEST:st-set-interval-over365-fails
    function test_SetMinCommissionUpdateInterval_RevertsWhenAbove365Days() public {
        vm.prank(stakeTableUpgradeTest.admin());
        vm.expectRevert(StakeTableV2.InvalidRateLimitParameters.selector);
        proxy.setMinCommissionUpdateInterval(365 days + 1);
    }

    // TEST:st-interval-exact-365-ok - exact boundary
    function test_SetMinCommissionUpdateInterval_ExactBoundary() public {
        vm.prank(stakeTableUpgradeTest.admin());
        proxy.setMinCommissionUpdateInterval(365 days);
        assertEq(proxy.minCommissionIncreaseInterval(), 365 days);
    }

    // TEST:st-set-maxinc-zero-fails
    function test_SetMaxCommissionIncrease_RevertsWhenZero() public {
        vm.prank(stakeTableUpgradeTest.admin());
        vm.expectRevert(StakeTableV2.InvalidRateLimitParameters.selector);
        proxy.setMaxCommissionIncrease(0);
    }

    // TEST:st-set-maxinc-over10000-fails
    function test_SetMaxCommissionIncrease_RevertsWhenAboveMax() public {
        vm.prank(stakeTableUpgradeTest.admin());
        vm.expectRevert(StakeTableV2.InvalidRateLimitParameters.selector);
        proxy.setMaxCommissionIncrease(10001);
    }

    // TEST:st-set-maxinc-ok - exact boundary 10000
    function test_SetMaxCommissionIncrease_ExactBoundary() public {
        vm.prank(stakeTableUpgradeTest.admin());
        proxy.setMaxCommissionIncrease(10000);
        assertEq(proxy.maxCommissionIncrease(), 10000);
    }

    // TEST:st-init-commission-invalid-fails - commission > 10000 in initialCommissions array
    function test_InitializeV2_RevertsInvalidCommission() public {
        StakeTableUpgradeV2Test upgradeTest = new StakeTableUpgradeV2Test();
        upgradeTest.setUp();
        S baseProxy = upgradeTest.getStakeTable();

        address validator = makeAddr("validator");
        upgradeTest.registerValidatorOnStakeTableV1(validator, "123", 500, baseProxy);

        StakeTableV2.InitialCommission[] memory badCommissions =
            new StakeTableV2.InitialCommission[](1);
        badCommissions[0] =
            StakeTableV2.InitialCommission({ validator: validator, commission: 10001 });

        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, upgradeTest.admin(), 0, badCommissions
        );

        vm.startPrank(upgradeTest.admin());
        StakeTableV2 implV2 = new StakeTableV2();
        vm.expectRevert(S.InvalidCommission.selector);
        baseProxy.upgradeToAndCall(address(implV2), initData);
        vm.stopPrank();
    }

    // TEST:st-commission-ratelimit-exact-boundary - increase at exact boundary succeeds (kills >=
    // to > mutant)
    function test_CommissionUpdate_SucceedsAtExactInterval() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);
        vm.startPrank(validator);
        uint256 t0 = block.timestamp;
        proxy.updateCommission(600); // first increase at t0
        // warp to exactly lastIncreaseTime + interval (not +1)
        vm.warp(t0 + proxy.minCommissionIncreaseInterval());
        proxy.updateCommission(700); // should succeed at exact boundary
        (uint16 storedCommission,) = proxy.commissionTracking(validator);
        assertEq(storedCommission, 700);
        vm.stopPrank();
    }

    // TEST:st-commission-maxinc-fails - increase exceeds max in single step
    function test_CommissionUpdate_RevertsWhenExceedsMaxIncrease() public {
        address validator = makeAddr("validator");
        stakeTableUpgradeTest.registerValidatorOnStakeTableV2(validator, "123", 500, proxy);
        vm.startPrank(validator);
        // maxCommissionIncrease is 500. Try to increase by 501.
        vm.expectRevert(StakeTableV2.CommissionIncreaseExceedsMax.selector);
        proxy.updateCommission(500 + 501);
        vm.stopPrank();
    }
}
