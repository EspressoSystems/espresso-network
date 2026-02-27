// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

/* solhint-disable func-name-mixedcase */

import "./RewardClaim.t.sol";

contract RewardClaimProofVerificationTest is RewardClaimTest {
    function test_ValidProof_SingleAccount_Succeeds() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        vm.prank(testCase.account);
        vm.expectEmit(true, true, true, true);
        emit IRewardClaim.RewardsClaimed(testCase.account, testCase.lifetimeRewards);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

        assertEq(espToken.balanceOf(testCase.account), testCase.lifetimeRewards);
        assertEq(rewardClaim.claimedRewards(testCase.account), testCase.lifetimeRewards);
        assertEq(rewardClaim.totalClaimed(), testCase.lifetimeRewards);
    }

    function test_ValidProof_MultipleAccounts_Succeeds() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getRewardFixtures(10);
        lightClient.setAuthRoot(authRoot);

        uint256 expectedTotalClaimed = 0;
        for (uint256 i = 0; i < fixtures.length; i++) {
            RewardClaimTestCase memory testCase = fixtures[i];

            vm.prank(testCase.account);
            rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

            assertEq(espToken.balanceOf(testCase.account), testCase.lifetimeRewards);
            expectedTotalClaimed += testCase.lifetimeRewards;
        }
        assertEq(rewardClaim.totalClaimed(), expectedTotalClaimed);
    }

    function test_WrongAddress_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getRewardFixtures(2, 0);
        lightClient.setAuthRoot(authRoot);

        address attacker = fixtures[1].account;
        RewardClaimTestCase memory victimProof = fixtures[0];

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(attacker);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(victimProof.lifetimeRewards, victimProof.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_WrongAddress_Random_Fails() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        address randomAttacker = makeAddr("attacker");

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(randomAttacker);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_WrongAmount_Higher_Fails() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        validateTestCase(testCase, authRoot);

        uint256 inflatedAmount = testCase.lifetimeRewards + 1;

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(inflatedAmount, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_WrongAmount_Lower_Fails() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        vm.assume(testCase.lifetimeRewards > 1);
        validateTestCase(testCase, authRoot);

        uint256 lowerAmount = testCase.lifetimeRewards - 1;

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(lowerAmount, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_AlreadyClaimed_Full_Fails() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        vm.prank(testCase.account);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

        uint256 totalClaimedAfterFirstClaim = rewardClaim.totalClaimed();
        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.AlreadyClaimed.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedAfterFirstClaim);
    }

    function test_WrongAuthRoot_Fails() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);

        lightClient.setAuthRoot(authRoot + 1);

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_NoAuthRoot_Fails() public {
        (, RewardClaimTestCase memory testCase) = getRewardFixture(0);

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_ClaimZeroAmount_Fails() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidRewardAmount.selector);
        rewardClaim.claimRewards(0, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_ClaimingZeroRewards_Fails() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidRewardAmount.selector);
        // The amount is checked first, so we don't need the correct authData here.
        rewardClaim.claimRewards(0, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }

    function test_AddressZero_CannotClaim() public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getRewardFixture(0);
        lightClient.setAuthRoot(authRoot);

        uint256 totalClaimedBefore = rewardClaim.totalClaimed();
        vm.prank(address(0));
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
        assertEq(rewardClaim.totalClaimed(), totalClaimedBefore);
    }
}
