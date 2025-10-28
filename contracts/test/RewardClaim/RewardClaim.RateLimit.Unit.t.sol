// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";
import "../../src/interfaces/IRewardClaim.sol";

// Conventions:
// - Use checkLimitEnforced() helper for verifying limit exceeded
// - Tests verifying limit exceeded should exceed by exactly 1 wei for precision
contract RewardClaimRateLimitTest is RewardClaimTest {
    function checkLimitEnforced(address user, uint256 lifetimeRewards) internal {
        vm.prank(user);
        vm.expectRevert(IRewardClaim.DailyLimitExceeded.selector);
        rewardClaim.claimRewards(lifetimeRewards, "");
    }

    function test_Claim_WithinLimit() public {
        claim(1);
    }

    function test_Claim_ExactLimit() public {
        claim(DAILY_LIMIT);
    }

    function test_Claim_Multiple() public {
        uint256 halfLimit = DAILY_LIMIT / 2;

        claim(halfLimit);
        claim(DAILY_LIMIT);
    }

    function test_Claim_ExceedsLimit() public {
        checkLimitEnforced(claimer, DAILY_LIMIT + 1);
    }

    function test_Claim_ExceedsAfterPartial() public {
        claim(DAILY_LIMIT);
        checkLimitEnforced(claimer, DAILY_LIMIT + 1);
    }

    function test_Claim_AfterDailyReset() public {
        claim(DAILY_LIMIT);

        vm.warp(block.timestamp + 1 days);

        claim(DAILY_LIMIT * 2);
    }

    function test_Claim_ExactDayBoundary() public {
        uint256 startTime = block.timestamp;
        uint256 startDay = startTime / 1 days;
        uint256 nextDayStart = (startDay + 1) * 1 days;

        claim(DAILY_LIMIT);

        vm.warp(nextDayStart - 1);
        checkLimitEnforced(claimer, DAILY_LIMIT + 1);

        vm.warp(nextDayStart);
        claim(DAILY_LIMIT * 2);
    }

    function test_SetDailyLimit_IncreasesCapacity() public {
        claim(DAILY_LIMIT);
        checkLimitEnforced(claimer, DAILY_LIMIT + 1);

        uint256 newLimit = DAILY_LIMIT * 2;
        vm.prank(owner);
        rewardClaim.setDailyLimit(newLimit);

        claim(newLimit);
        checkLimitEnforced(claimer, newLimit + 1);
    }

    function test_MultipleUsers_SharedLimit() public {
        address user1 = makeAddr("user1");
        address user2 = makeAddr("user2");
        uint256 halfLimit = DAILY_LIMIT / 2;

        claimAs(user1, halfLimit);
        claimAs(user2, halfLimit);

        checkLimitEnforced(user1, halfLimit + 1);
        checkLimitEnforced(user2, halfLimit + 1);
    }

    function test_DecreasedLimit_AppliesNextDay() public {
        claim(DAILY_LIMIT);

        uint256 newLimit = DAILY_LIMIT / 2;
        vm.prank(owner);
        rewardClaim.setDailyLimit(newLimit);

        checkLimitEnforced(claimer, DAILY_LIMIT + 1);

        vm.warp(block.timestamp + 1 days);

        claim(DAILY_LIMIT + newLimit);

        checkLimitEnforced(claimer, DAILY_LIMIT + newLimit + 1);
    }

    function testFuzz_ClaimWithinLimit(uint256 amount) public {
        amount = bound(amount, 1, DAILY_LIMIT);

        claim(amount);
    }

    function testFuzz_ExceedsLimit(uint256 amount) public {
        amount = bound(amount, DAILY_LIMIT + 1, type(uint256).max / 2);

        vm.prank(claimer);
        vm.expectRevert();
        rewardClaim.claimRewards(amount, "");
    }
}
