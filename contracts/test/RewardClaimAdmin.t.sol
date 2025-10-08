// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";

contract RewardClaimAdminTest is RewardClaimTest {
    function test_SetDailyLimit_Success() public {
        uint256 newLimit = DAILY_LIMIT * 2;

        vm.prank(owner);
        vm.expectEmit(true, true, true, true);
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
}
