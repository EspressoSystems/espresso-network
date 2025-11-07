// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";

contract RewardClaimAdminTest is RewardClaimTest {
    function test_SetDailyLimit_Success() public {
        uint256 currentLimit = rewardClaim.dailyLimitWei();
        uint256 basisPoints = 200; // 2%
        uint256 expectedLimit = (espToken.totalSupply() * basisPoints) / 10000;

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
        uint256 expectedLimit = (espToken.totalSupply() * maxBasisPoints) / 10000;

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
}
