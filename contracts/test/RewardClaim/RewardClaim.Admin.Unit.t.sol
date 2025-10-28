// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract RewardClaimAdminTest is RewardClaimTest {
    function test_SetDailyLimit_Success() public {
        uint256 currentLimit = rewardClaim.dailyLimit();
        uint256 newLimit = currentLimit * 2;

        vm.prank(owner);
        vm.expectEmit();
        emit RewardClaim.DailyLimitUpdated(currentLimit, newLimit);
        rewardClaim.setDailyLimit(newLimit);

        assertEq(rewardClaim.dailyLimit(), newLimit);
    }

    function test_SetDailyLimit_RevertsNonOwner() public {
        address attacker = makeAddr("attacker");
        uint256 newLimit = rewardClaim.dailyLimit() + 1;
        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, attacker)
        );
        rewardClaim.setDailyLimit(newLimit);
    }

    function test_SetDailyLimit_RevertsZero() public {
        vm.prank(owner);
        vm.expectRevert(RewardClaim.ZeroDailyLimit.selector);
        rewardClaim.setDailyLimit(0);
    }

    function test_SetDailyLimit_RevertsNoChangeRequired() public {
        uint256 currentLimit = rewardClaim.dailyLimit();
        vm.prank(owner);
        vm.expectRevert(RewardClaim.NoChangeRequired.selector);
        rewardClaim.setDailyLimit(currentLimit);
    }

    function test_SetDailyLimit_SuccessAtMaxBound() public {
        uint256 currentLimit = rewardClaim.dailyLimit();
        uint256 totalSupply = espToken.totalSupply();
        uint256 maxLimit = (totalSupply * rewardClaim.MAX_DAILY_LIMIT_PERCENTAGE()) / 100e18;

        vm.prank(owner);
        vm.expectEmit();
        emit RewardClaim.DailyLimitUpdated(currentLimit, maxLimit);
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
