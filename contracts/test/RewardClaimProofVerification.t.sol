// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

/* solhint-disable func-name-mixedcase */

import "./RewardClaimTestBase.sol";

contract RewardClaimProofVerificationTest is RewardClaimTestBase {
    function test_ValidProof_SingleAccount_Succeeds() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];

        vm.prank(testCase.account);
        vm.expectEmit(true, true, true, true);
        emit IRewardClaim.RewardsClaimed(testCase.account, testCase.lifetimeRewards);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

        assertEq(espToken.balanceOf(testCase.account), testCase.lifetimeRewards);
        assertEq(rewardClaim.claimedRewards(testCase.account), testCase.lifetimeRewards);
    }

    function test_ValidProof_MultipleAccounts_Succeeds() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(10);
        lightClient.setAuthRoot(authRoot);

        for (uint256 i = 0; i < fixtures.length; i++) {
            RewardClaimTestCase memory testCase = fixtures[i];

            vm.prank(testCase.account);
            rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

            assertEq(espToken.balanceOf(testCase.account), testCase.lifetimeRewards);
        }
    }

    function test_WrongAddress_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(2);
        lightClient.setAuthRoot(authRoot);

        address attacker = fixtures[1].account;
        RewardClaimTestCase memory victimProof = fixtures[0];

        vm.prank(attacker);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(victimProof.lifetimeRewards, victimProof.authData);
    }

    function test_WrongAddress_Random_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        address randomAttacker = makeAddr("attacker");

        vm.prank(randomAttacker);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(fixtures[0].lifetimeRewards, fixtures[0].authData);
    }

    function test_WrongAmount_Higher_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];
        validateTestCase(testCase, authRoot);

        uint256 inflatedAmount = testCase.lifetimeRewards + 1;

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(inflatedAmount, testCase.authData);
    }

    function test_WrongAmount_Lower_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];
        vm.assume(testCase.lifetimeRewards > 1);
        validateTestCase(testCase, authRoot);

        uint256 lowerAmount = testCase.lifetimeRewards - 1;

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(lowerAmount, testCase.authData);
    }

    function test_AlreadyClaimed_Full_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];

        vm.prank(testCase.account);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.AlreadyClaimed.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
    }

    function test_WrongAuthRoot_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);

        lightClient.setAuthRoot(authRoot + 1);

        RewardClaimTestCase memory testCase = fixtures[0];

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
    }

    function test_NoAuthRoot_Fails() public {
        (, RewardClaimTestCase[] memory fixtures) = getFixtures(1);

        RewardClaimTestCase memory testCase = fixtures[0];

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
    }

    function test_ClaimZeroAmount_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidRewardAmount.selector);
        rewardClaim.claimRewards(0, testCase.authData);
    }

    function test_AccountInTreeWithZeroRewards_Fails() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixturesWithAmount(1, 0);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidRewardAmount.selector);
        rewardClaim.claimRewards(0, testCase.authData);
    }

    function test_AddressZero_CannotClaim() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];

        vm.prank(address(0));
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
    }
}
