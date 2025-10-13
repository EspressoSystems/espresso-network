// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import "./mocks/MockRewardClaim.sol";
import "../src/EspTokenV2.sol";

// Conventions:
// - Use claim() or claimAs() helpers for successful claims (validates events and balances)
contract RewardClaimTest is Test {
    MockRewardClaim public rewardClaim;
    EspTokenV2 public espToken;
    address public owner;
    address public pauser;
    address public claimer;

    uint256 constant DAILY_LIMIT = 1000 ether;

    function setUp() public {
        owner = makeAddr("owner");
        pauser = makeAddr("pauser");
        claimer = makeAddr("claimer");

        EspTokenV2 espTokenImpl = new EspTokenV2();
        bytes memory espTokenInitData = abi.encodeWithSignature(
            "initialize(address,address,uint256,string,string)",
            owner,
            owner,
            100_000,
            "Espresso",
            "ESP"
        );
        ERC1967Proxy espTokenProxy = new ERC1967Proxy(address(espTokenImpl), espTokenInitData);
        espToken = EspTokenV2(payable(address(espTokenProxy)));

        MockRewardClaim rewardClaimImpl = new MockRewardClaim();
        bytes memory rewardClaimInitData = abi.encodeWithSignature(
            "initialize(address,address,address,address)",
            owner,
            address(espToken),
            makeAddr("lightClient"),
            pauser
        );
        ERC1967Proxy rewardClaimProxy =
            new ERC1967Proxy(address(rewardClaimImpl), rewardClaimInitData);
        rewardClaim = MockRewardClaim(payable(address(rewardClaimProxy)));

        vm.prank(owner);
        espToken.initializeV2(address(rewardClaim));

        assertEq(rewardClaim.dailyLimit(), DAILY_LIMIT);
    }

    function claim(uint256 lifetimeRewards) internal {
        claimAs(claimer, lifetimeRewards);
    }

    function claimAs(address user, uint256 lifetimeRewards) internal {
        uint256 balanceBefore = espToken.balanceOf(user);
        uint256 alreadyClaimed = rewardClaim.claimedRewards(user);
        uint256 amountToClaim = lifetimeRewards - alreadyClaimed;

        vm.prank(user);
        vm.expectEmit();
        emit IRewardClaim.RewardsClaimed(user, amountToClaim);
        rewardClaim.claimRewards(lifetimeRewards, "");

        assertEq(espToken.balanceOf(user), balanceBefore + amountToClaim);
    }
}
