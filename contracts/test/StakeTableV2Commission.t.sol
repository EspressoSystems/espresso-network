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
            StakeTableV2.initializeV2.selector, pauser, admin, emptyCommissions
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
        uint16 maxCommission = 10000;
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

    function test_SetMinCommissionUpdateInterval_RevertWhenNotOwner() public {
        address notAdmin = makeAddr("notAdmin");
        uint256 newInterval = 14 days;

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, notAdmin)
        );
        proxy.setMinCommissionUpdateInterval(newInterval);
        vm.stopPrank();
    }

    function test_SetMaxCommissionIncrease_RevertWhenNotOwner() public {
        address notAdmin = makeAddr("notAdmin");
        uint16 newMaxIncrease = 1000;

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, notAdmin)
        );
        proxy.setMaxCommissionIncrease(newMaxIncrease);
        vm.stopPrank();
    }

    function test_DefaultValues() public view {
        // This is the only test that checks default values - if defaults change, only this test
        // should fail
        assertEq(proxy.minCommissionIncreaseInterval(), 7 days);
        assertEq(proxy.maxCommissionIncrease(), 500);
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
            StakeTableV2.initializeV2.selector, pauser, upgradeTest.admin(), wrongCommissions
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
            StakeTableV2.initializeV2.selector, pauser, upgradeTest.admin(), duplicateCommissions
        );

        vm.startPrank(upgradeTest.admin());
        StakeTableV2 implV2 = new StakeTableV2();
        vm.expectRevert(
            abi.encodeWithSelector(StakeTableV2.CommissionAlreadyInitialized.selector, validator)
        );
        baseProxy.upgradeToAndCall(address(implV2), initData);
        vm.stopPrank();
    }
}
