// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";

contract RewardClaimAdminTest is RewardClaimTest {
    function test_SetDailyLimit_Success() public {
        uint256 currentLimit = rewardClaim.dailyLimitWei();
        uint256 basisPoints = 200; // 2%
        uint256 expectedLimit =
            (espToken.totalSupply() * basisPoints) / rewardClaim.BPS_DENOMINATOR();

        vm.prank(owner);
        vm.expectEmit();
        emit RewardClaim.DailyLimitUpdated(currentLimit, expectedLimit);
        rewardClaim.setDailyLimit(basisPoints);

        assertEq(rewardClaim.dailyLimitWei(), expectedLimit);
    }

    function test_SetDailyLimit_RevertsNonAdmin() public {
        address attacker = makeAddr("attacker");
        uint256 basisPoints = 200; // 2%
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();
        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, attacker, adminRole
            )
        );
        rewardClaim.setDailyLimit(basisPoints);
    }

    function test_SetDailyLimit_RevertsZero() public {
        vm.prank(owner);
        vm.expectRevert(RewardClaim.ZeroDailyLimit.selector);
        rewardClaim.setDailyLimit(0);
    }

    function test_SetDailyLimit_RevertsNoChangeRequired() public {
        uint256 basisPoints = 100; // 1% - same as initial value
        vm.prank(owner);
        vm.expectRevert(RewardClaim.NoChangeRequired.selector);
        rewardClaim.setDailyLimit(basisPoints);
    }

    function test_SetDailyLimit_SuccessAtMaxBound() public {
        uint256 currentLimit = rewardClaim.dailyLimitWei();
        uint256 maxBasisPoints = rewardClaim.MAX_DAILY_LIMIT_BASIS_POINTS();
        uint256 expectedLimit =
            (espToken.totalSupply() * maxBasisPoints) / rewardClaim.BPS_DENOMINATOR();

        vm.prank(owner);
        vm.expectEmit();
        emit RewardClaim.DailyLimitUpdated(currentLimit, expectedLimit);
        rewardClaim.setDailyLimit(maxBasisPoints);

        assertEq(rewardClaim.dailyLimitWei(), expectedLimit);
    }

    function test_SetDailyLimit_RevertsAboveMaxBound() public {
        uint256 maxBasisPoints = rewardClaim.MAX_DAILY_LIMIT_BASIS_POINTS();
        uint256 tooHigh = maxBasisPoints + 1;

        vm.prank(owner);
        vm.expectRevert(RewardClaim.DailyLimitTooHigh.selector);
        rewardClaim.setDailyLimit(tooHigh);
    }

    function test_SetDailyLimit_MaxPercentageIs5Percent() public view {
        assertEq(rewardClaim.MAX_DAILY_LIMIT_BASIS_POINTS(), 500);
    }

    function test_RenounceRole_RevertsForDefaultAdminRole() public {
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();
        vm.prank(owner);
        vm.expectRevert(RewardClaim.DefaultAdminCannotBeRenounced.selector);
        rewardClaim.renounceRole(adminRole, owner);
    }

    function test_RenounceRole_SucceedsForOtherRoles() public {
        bytes32 pauserRole = rewardClaim.PAUSER_ROLE();
        address pauser = makeAddr("pauser");

        // Grant pauser role to a new address
        vm.prank(owner);
        rewardClaim.grantRole(pauserRole, pauser);

        // Pauser can renounce their own role
        vm.prank(pauser);
        rewardClaim.renounceRole(pauserRole, pauser);

        // Verify role was renounced
        assertFalse(rewardClaim.hasRole(pauserRole, pauser));
    }

    function test_RenouncePauserRole_ByNonRoleHolderReverts() public {
        bytes32 pauserRole = rewardClaim.PAUSER_ROLE();
        address attacker = makeAddr("attacker");
        // pauser has PAUSER_ROLE (from setup), attacker does not
        vm.prank(attacker);
        vm.expectRevert(IAccessControl.AccessControlBadConfirmation.selector);
        // Try to renounce pauser's role (attacker doesn't have this role)
        rewardClaim.renounceRole(pauserRole, pauser);
    }

    function test_RenounceRole_DefaultAdminRoleRevertsEvenForNonOwner() public {
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();
        address attacker = makeAddr("attacker");
        vm.prank(attacker);
        // Even non-owners get DefaultAdminCannotBeRenounced, not AccessControlUnauthorizedAccount
        // because the DEFAULT_ADMIN_ROLE check happens before authorization check
        vm.expectRevert(RewardClaim.DefaultAdminCannotBeRenounced.selector);
        rewardClaim.renounceRole(adminRole, owner);
    }

    function test_RevokeRole_RevertsForDefaultAdminRole() public {
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();
        vm.prank(owner);
        vm.expectRevert(RewardClaim.DefaultAdminCannotBeRevoked.selector);
        rewardClaim.revokeRole(adminRole, owner);
    }

    function test_RevokeRole_SucceedsForOtherRoles() public {
        bytes32 pauserRole = rewardClaim.PAUSER_ROLE();
        address pauserAddress = makeAddr("pauserAddress");
        vm.startPrank(owner);
        rewardClaim.grantRole(pauserRole, pauserAddress);
        assertTrue(rewardClaim.hasRole(pauserRole, pauserAddress));
        rewardClaim.revokeRole(pauserRole, pauserAddress);
        vm.stopPrank();
        assertFalse(rewardClaim.hasRole(pauserRole, pauserAddress));
    }

    function test_GrantRole_DefaultAdminRole_TransfersAdmin() public {
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();
        address newAdmin = makeAddr("newAdmin");

        vm.startPrank(owner);
        vm.expectEmit(true, true, true, true, address(rewardClaim));
        emit IAccessControl.RoleGranted(adminRole, newAdmin, owner);
        vm.expectEmit(true, true, true, true, address(rewardClaim));
        emit IAccessControl.RoleRevoked(adminRole, owner, owner);
        rewardClaim.grantRole(adminRole, newAdmin);
        vm.stopPrank();

        assertTrue(rewardClaim.hasRole(adminRole, newAdmin), "new admin should hold role");
        assertFalse(rewardClaim.hasRole(adminRole, owner), "old admin should lose role");
        assertEq(rewardClaim.currentAdmin(), newAdmin);
    }

    function test_GrantRole_DefaultAdminRole_SelfGrantNoOp() public {
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();

        vm.prank(owner);
        rewardClaim.grantRole(adminRole, owner);

        assertTrue(rewardClaim.hasRole(adminRole, owner), "owner should still have role");
        assertEq(rewardClaim.currentAdmin(), owner);
    }
}
