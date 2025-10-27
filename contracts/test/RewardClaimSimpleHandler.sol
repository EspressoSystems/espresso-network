// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import "./RewardClaimTestBase.sol";

contract RewardClaimSimpleHandler is RewardClaimTestBase {
    address public user;
    uint256 public currentLifetimeRewards;
    RewardClaimTestCase public currentFixture;

    mapping(uint256 => uint256) public claimsByDay;

    uint256 public numValidClaims;
    uint256 public numDoubleClaims;
    uint256 public numDailyLimitHits;
    uint256 public totalClaimed;
    uint256 public initialSupply;

    uint256 public constant DAILY_LIMIT = 300;

    constructor() {
        super.setUp();

        user = vm.addr(1);
        currentLifetimeRewards = 0;
        initialSupply = espToken.totalSupply();

        vm.prank(owner);
        rewardClaim.setDailyLimit(DAILY_LIMIT);

        _updateRootInternal(block.timestamp, 0);
    }

    function _updateRootInternal(uint256 seed, uint256 increment) private {
        currentLifetimeRewards += increment;

        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "gen-reward-fixture-with-account-and-amount";
        cmds[2] = vm.toString(user);
        cmds[3] = vm.toString(currentLifetimeRewards);
        bytes memory result = vm.ffi(cmds);

        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) =
            abi.decode(result, (uint256, RewardClaimTestCase[]));

        lightClient.setAuthRoot(authRoot);
        currentFixture = fixtures[0];
    }

    function claimRewards() public {
        uint256 previousClaimed = rewardClaim.claimedRewards(user);
        uint256 lifetimeRewards = currentFixture.lifetimeRewards;

        if (lifetimeRewards == previousClaimed) {
            numDoubleClaims++;
            return;
        }

        uint256 amountToClaim = lifetimeRewards - previousClaimed;
        uint256 currentDay = block.timestamp / 1 days;
        uint256 claimedToday = claimsByDay[currentDay];

        if (claimedToday + amountToClaim > DAILY_LIMIT) {
            vm.prank(user);
            vm.expectRevert(IRewardClaim.DailyLimitExceeded.selector);
            rewardClaim.claimRewards(lifetimeRewards, currentFixture.authData);
            numDailyLimitHits++;
        } else {
            vm.prank(user);
            rewardClaim.claimRewards(lifetimeRewards, currentFixture.authData);

            claimsByDay[currentDay] += amountToClaim;
            totalClaimed += amountToClaim;
            numValidClaims++;
        }
    }

    function updateRoot(uint256 seed) public {
        uint256 increment = seed % 101;
        _updateRootInternal(seed, increment);
    }

    function advanceTime(uint256 hoursSeed) public {
        uint256 numHours = _bound(hoursSeed, 1, 48);
        vm.warp(block.timestamp + numHours * 1 hours);
    }

    function getTotalSupply() external view returns (uint256) {
        return espToken.totalSupply();
    }
}
