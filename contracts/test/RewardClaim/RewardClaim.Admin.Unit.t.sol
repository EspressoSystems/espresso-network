// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";

contract RewardClaimAdminTest is RewardClaimTest {
    function test_SetDailyLimit_Success() public {
        uint256 newLimit = DAILY_LIMIT * 2;

        vm.prank(owner);
        vm.expectEmit();
        emit RewardClaim.DailyLimitUpdated(DAILY_LIMIT, newLimit);
        rewardClaim.setDailyLimit(newLimit);

        assertEq(rewardClaim.dailyLimit(), newLimit);
    }

    function test_SetDailyLimit_RevertsNonOwner() public {
        vm.prank(claimer);
        vm.expectRevert();
        rewardClaim.setDailyLimit(DAILY_LIMIT * 2);
    }

    function test_SetDailyLimit_RevertsZero() public {
        vm.prank(owner);
        vm.expectRevert(RewardClaim.ZeroDailyLimit.selector);
        rewardClaim.setDailyLimit(0);
    }

    function test_SetDailyLimit_SuccessAtMaxBound() public {
        uint256 totalSupply = espToken.totalSupply();
        uint256 maxLimit = (totalSupply * rewardClaim.MAX_DAILY_LIMIT_PERCENTAGE()) / 100e18;

        vm.prank(owner);
        vm.expectEmit();
        emit RewardClaim.DailyLimitUpdated(DAILY_LIMIT, maxLimit);
        rewardClaim.setDailyLimit(maxLimit);

        assertEq(rewardClaim.dailyLimit(), maxLimit);
    }

    function test_SetDailyLimit_RevertsAboveMaxBound() public {
        uint256 totalSupply = espToken.totalSupply();
        uint256 maxLimit = (totalSupply * rewardClaim.MAX_DAILY_LIMIT_PERCENTAGE()) / 100e18;
        uint256 tooHigh = maxLimit + 1;

        vm.prank(owner);
        vm.expectRevert(RewardClaim.DailyLimitTooHigh.selector);
        rewardClaim.setDailyLimit(tooHigh);
    }

    function test_SetDailyLimit_MaxPercentageIs5Percent() public view {
        assertEq(rewardClaim.MAX_DAILY_LIMIT_PERCENTAGE(), 5e18);
    }
}
