// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import "forge-std/StdInvariant.sol";
import { console } from "forge-std/console.sol";
import "./RewardClaimSimpleHandler.sol";

/// forge-config: quick.invariant.runs = 1
contract RewardClaimSimpleInvariantTest is StdInvariant, Test {
    RewardClaimSimpleHandler public handler;

    function setUp() public {
        handler = new RewardClaimSimpleHandler();
        targetContract(address(handler));

        targetSelector(FuzzSelector({ addr: address(handler), selectors: _getTargetSelectors() }));
    }

    function _getTargetSelectors() internal pure returns (bytes4[] memory) {
        bytes4[] memory selectors = new bytes4[](3);
        selectors[0] = RewardClaimSimpleHandler.claimRewards.selector;
        selectors[1] = RewardClaimSimpleHandler.updateRoot.selector;
        selectors[2] = RewardClaimSimpleHandler.advanceTime.selector;
        return selectors;
    }

    function afterInvariant() external view {
        console.log("\n=== Simple Reward Claim Invariant Test Stats ===");
        console.log("Valid claims:         ", handler.numValidClaims());
        console.log("Double claims:        ", handler.numDoubleClaims());
        console.log("Daily limit hits:     ", handler.numDailyLimitHits());
        console.log("Total claimed:        ", handler.totalClaimed());
        console.log("Lifetime rewards:     ", handler.currentLifetimeRewards());
        console.log("Daily limit:          ", handler.rewardClaim().dailyLimit());
    }

    function invariant_TokenConservation() public view {
        uint256 totalMinted = handler.getTotalSupply() - handler.initialSupply();
        assertEq(totalMinted, handler.totalClaimed(), "Token conservation violated");
    }

    function invariant_DailyLimit() public view {
        uint256 currentDay = block.timestamp / 1 days;
        uint256 claimedToday = handler.claimsByDay(currentDay);
        assertLe(claimedToday, handler.rewardClaim().dailyLimit(), "Daily limit exceeded");
    }
}
