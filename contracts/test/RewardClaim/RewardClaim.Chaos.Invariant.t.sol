// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

/* solhint-disable func-name-mixedcase, no-console */

import "./RewardClaim.Base.t.sol";
import "forge-std/StdInvariant.sol";
import "forge-std/console.sol";

contract RewardClaimHandler is RewardClaimTestBase {
    struct AccountState {
        address account;
        uint256 lifetimeRewards;
    }

    struct FuncStats {
        uint256 ok;
        uint256 reverts;
    }

    struct Stats {
        FuncStats evolveState;
        FuncStats advanceTime;
        FuncStats updateDailyLimit;
    }

    AccountState[] public rewardState;
    RewardClaimTestCase[] public lastFixtures;
    mapping(address => uint256) public totalClaimed;
    uint256 public totalDailyClaims;
    uint256 public totalClaimedAllAccounts;
    uint256 public initialSupply;
    uint256 public currentDay;
    uint256 public numIterations;
    uint256 public totalClaimAttempts;
    uint256 public successfulClaims;
    uint256 public dailyLimitHits;
    uint64 public seed;

    Stats public stats;

    function initialize() public {
        seed = 1;
        currentDay = block.timestamp / 1 days;
        initialSupply = espToken.totalSupply();

        vm.prank(rewardClaim.owner());
        rewardClaim.setDailyLimit(1000 ether);
    }

    function evolveState(uint256 addSeed, uint256 updateSeed) public {
        numIterations++;
        seed = uint64(uint256(keccak256(abi.encodePacked(seed, block.timestamp, numIterations))));

        if (rewardState.length > 0 && seed % 3 != 0) {
            stats.evolveState.ok++;
            return;
        }

        bytes memory encodedState = abi.encode(rewardState);
        string memory hexState = vm.toString(encodedState);

        uint256 numAccountsToAdd = _bound(addSeed, 0, 10);
        uint256 numAccountsToUpdate = _bound(updateSeed, 0, 10);

        string[] memory cmds = new string[](9);
        cmds[0] = "diff-test";
        cmds[1] = "evolve-reward-state";
        cmds[2] = hexState;
        cmds[3] = vm.toString(seed);
        cmds[4] = vm.toString(numAccountsToAdd);
        cmds[5] = vm.toString(numAccountsToUpdate);
        cmds[6] = vm.toString(uint256(10 ether));
        cmds[7] = vm.toString(uint256(100 ether));
        cmds[8] = "10";

        bytes memory result = vm.ffi(cmds);
        (uint256 authRoot, AccountState[] memory newState, RewardClaimTestCase[] memory fixtures) =
            abi.decode(result, (uint256, AccountState[], RewardClaimTestCase[]));

        lightClient.setAuthRoot(authRoot);

        delete rewardState;
        for (uint256 i = 0; i < newState.length; i++) {
            rewardState.push(newState[i]);
        }

        delete lastFixtures;
        for (uint256 i = 0; i < fixtures.length; i++) {
            lastFixtures.push(fixtures[i]);
        }

        uint256 today = block.timestamp / 1 days;
        if (today != currentDay) {
            currentDay = today;
            totalDailyClaims = 0;
        }

        for (uint256 i = 0; i < fixtures.length; i++) {
            RewardClaimTestCase memory testCase = fixtures[i];
            uint256 alreadyClaimed = totalClaimed[testCase.account];

            if (testCase.lifetimeRewards <= alreadyClaimed) {
                continue;
            }

            uint256 amountToClaim = testCase.lifetimeRewards - alreadyClaimed;

            totalClaimAttempts++;

            bool shouldExceedLimit = totalDailyClaims + amountToClaim > rewardClaim.dailyLimit();

            if (shouldExceedLimit) {
                dailyLimitHits++;
                vm.prank(testCase.account);
                vm.expectRevert(IRewardClaim.DailyLimitExceeded.selector);
                rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
            } else {
                vm.prank(testCase.account);
                rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

                totalClaimed[testCase.account] = testCase.lifetimeRewards;
                totalDailyClaims += amountToClaim;
                totalClaimedAllAccounts += amountToClaim;
                successfulClaims++;

                assertEq(rewardClaim.claimedRewards(testCase.account), testCase.lifetimeRewards);

                // Invariant: immediately after a successful claim, total claimed today must not
                // exceed the daily limit. We check here rather than in invariant_* because the
                // limit can be reduced below totalDailyClaims by admin (valid), but claims must
                // always respect the limit at the time they execute.
                assertLe(
                    totalDailyClaims,
                    rewardClaim.dailyLimit(),
                    "Daily limit exceeded immediately after claim"
                );

                vm.prank(testCase.account);
                vm.expectRevert(IRewardClaim.AlreadyClaimed.selector);
                rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
            }
        }

        stats.evolveState.ok++;
    }

    function advanceTime(uint256 hoursSeed) public {
        uint256 numHours = _bound(hoursSeed, 1, 48);
        vm.warp(block.timestamp + numHours * 1 hours);
        stats.advanceTime.ok++;
    }

    function updateDailyLimit(uint256 limitSeed) public {
        uint256 totalSupply = espToken.totalSupply();
        uint256 maxLimit = (totalSupply * rewardClaim.MAX_DAILY_LIMIT_PERCENTAGE()) / 100e18;
        uint256 newLimit = _bound(limitSeed, 100, maxLimit);
        vm.prank(rewardClaim.owner());
        rewardClaim.setDailyLimit(newLimit);
        stats.updateDailyLimit.ok++;
    }

    function getTotalCalls() external view returns (uint256) {
        return stats.evolveState.ok + stats.advanceTime.ok + stats.updateDailyLimit.ok;
    }

    function getRewardStateLength() external view returns (uint256) {
        return rewardState.length;
    }
}

/// forge-config: default.invariant.runs = 1
/// forge-config: default.invariant.depth = 200
contract RewardClaimInvariantTest is StdInvariant, Test {
    RewardClaimHandler public handler;

    function setUp() public {
        handler = new RewardClaimHandler();
        handler.setUp();
        handler.initialize();

        targetContract(address(handler));

        bytes4[] memory selectors = new bytes4[](3);
        selectors[0] = RewardClaimHandler.evolveState.selector;
        selectors[1] = RewardClaimHandler.advanceTime.selector;
        selectors[2] = RewardClaimHandler.updateDailyLimit.selector;

        targetSelector(FuzzSelector({ addr: address(handler), selectors: selectors }));
    }

    function invariant_contractHoldsNoTokens() public view {
        assertEq(
            handler.espToken().balanceOf(address(handler.rewardClaim())),
            0,
            "RewardClaim should not hold tokens"
        );
    }

    function invariant_totalMintedMatchesClaimed() public view {
        uint256 currentSupply = handler.espToken().totalSupply();
        uint256 totalMinted = currentSupply - handler.initialSupply();

        assertEq(totalMinted, handler.totalClaimedAllAccounts(), "Total minted != total claimed");
    }

    function afterInvariant() external view {
        console.log("\n=== Reward Claim Invariant Test Stats ===");
        console.log("Total calls:", handler.getTotalCalls());
        console.log("Iterations:", handler.numIterations());
        console.log("Total accounts in state:", handler.getRewardStateLength());
        console.log("Total claim attempts:", handler.totalClaimAttempts());
        console.log("Successful claims:", handler.successfulClaims());
        console.log("Daily limit hits:", handler.dailyLimitHits());
        console.log("Current daily claims:", handler.totalDailyClaims());
        console.log("Daily limit:", handler.rewardClaim().dailyLimit());
        uint256 utilization =
            (handler.totalDailyClaims() * 100) / handler.rewardClaim().dailyLimit();
        console.log("Daily limit utilization:", utilization, "%");
    }
}
