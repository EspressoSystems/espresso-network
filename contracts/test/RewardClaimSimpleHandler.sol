// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import "./RewardClaimTestBase.sol";

contract RewardClaimSimpleHandler is RewardClaimTestBase {
    struct AccountState {
        address account;
        uint256 lifetimeRewards;
    }

    AccountState[] public rewardState;
    RewardClaimTestCase public currentFixture;

    mapping(uint256 => uint256) public claimsByDay;

    uint256 public numValidClaims;
    uint256 public numDoubleClaims;
    uint256 public numDailyLimitHits;
    uint256 public totalClaimed;
    uint256 public initialSupply;

    uint256 public constant DAILY_LIMIT = 150;

    constructor() {
        super.setUp();

        initialSupply = espToken.totalSupply();

        vm.prank(owner);
        rewardClaim.setDailyLimit(DAILY_LIMIT);

        address user = vm.addr(1);
        rewardState.push(AccountState(user, 0));

        _updateRootInternal(block.timestamp);
    }

    function _updateRootInternal(uint256 seed) private {
        bytes memory encodedState = abi.encode(rewardState);
        string memory hexState = vm.toString(encodedState);

        string[] memory cmds = new string[](9);
        cmds[0] = "diff-test";
        cmds[1] = "evolve-reward-state";
        cmds[2] = hexState;
        cmds[3] = vm.toString(seed);
        cmds[4] = "0";
        cmds[5] = "1";
        cmds[6] = "0";
        cmds[7] = "100";
        cmds[8] = "1";

        bytes memory result = vm.ffi(cmds);
        (uint256 authRoot, AccountState[] memory newState, RewardClaimTestCase[] memory fixtures) =
            abi.decode(result, (uint256, AccountState[], RewardClaimTestCase[]));

        lightClient.setAuthRoot(authRoot);

        delete rewardState;
        for (uint256 i = 0; i < newState.length; i++) {
            rewardState.push(newState[i]);
        }

        if (fixtures.length > 0) {
            currentFixture = fixtures[0];
        }
    }

    function claimRewards() public {
        address user = rewardState[0].account;
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
        _updateRootInternal(seed);
    }

    function advanceTime(uint256 hoursSeed) public {
        uint256 numHours = _bound(hoursSeed, 1, 48);
        vm.warp(block.timestamp + numHours * 1 hours);
    }

    function getTotalSupply() external view returns (uint256) {
        return espToken.totalSupply();
    }
}
