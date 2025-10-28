// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";

contract RewardClaimPauseTest is RewardClaimTest {
    function test_claim_FailsIfPaused() public {
        vm.prank(pauser);
        rewardClaim.pause();

        vm.prank(claimer);
        vm.expectRevert(abi.encodeWithSignature("EnforcedPause()"));
        rewardClaim.claimRewards(1, "");
    }

    function test_claim_SucceedsIfUnpaused() public {
        vm.prank(pauser);
        rewardClaim.pause();

        vm.prank(pauser);
        rewardClaim.unpause();

        claim(1);
    }

    function test_pause_RevertsNonPauser() public {
        bytes32 pauserRole = rewardClaim.PAUSER_ROLE();
        vm.prank(claimer);
        vm.expectRevert(
            abi.encodeWithSignature(
                "AccessControlUnauthorizedAccount(address,bytes32)", claimer, pauserRole
            )
        );
        rewardClaim.pause();
    }

    function test_unpause_RevertsNonPauser() public {
        vm.prank(pauser);
        rewardClaim.pause();

        bytes32 pauserRole = rewardClaim.PAUSER_ROLE();
        vm.prank(claimer);
        vm.expectRevert(
            abi.encodeWithSignature(
                "AccessControlUnauthorizedAccount(address,bytes32)", claimer, pauserRole
            )
        );
        rewardClaim.unpause();
    }
}
