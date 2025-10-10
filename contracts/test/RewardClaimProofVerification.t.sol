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

    function test_AuthDataByteCorruption_CriticalBytes() public {
        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) = getFixtures(1);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];
        validateTestCase(testCase, authRoot);

        // authData structure (ABI-encoded):
        // Offset 0-31:    ABI offset to data start
        // Offset 32-63:   Data length
        // Offset 64-5183: Merkle proof siblings (160 × 32 bytes)
        // Offset 5184-5407: Auth root inputs (7 × 32 bytes)
        //
        // Test corruption at critical boundaries to ensure:
        // 1. First/last bytes of proof siblings are validated
        // 2. Transitions between array elements are checked
        // 3. Both proof and authRootInputs components are verified
        // 4. Random positions within each section are also validated
        uint256[] memory criticalPositions = new uint256[](8);
        criticalPositions[0] = 64;
        criticalPositions[1] = 95;
        criticalPositions[2] = 96;
        criticalPositions[3] = 5183;
        criticalPositions[4] = 5184;
        criticalPositions[5] = 5407;
        criticalPositions[6] = 1000;
        criticalPositions[7] = 5300;

        for (uint256 i = 0; i < criticalPositions.length; i++) {
            bytes memory corruptedAuthData = testCase.authData;
            uint256 pos = criticalPositions[i];

            if (pos < corruptedAuthData.length) {
                corruptedAuthData[pos] ^= 0xFF;

                vm.prank(testCase.account);
                vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
                rewardClaim.claimRewards(testCase.lifetimeRewards, corruptedAuthData);
            }
        }
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

    function test_AccumulatedRewards_PartialThenFull_Succeeds() public {
        (uint256 authRoot1, RewardClaimTestCase[] memory fixtures1) =
            getFixturesWithAmount(1, 100 ether);
        lightClient.setAuthRoot(authRoot1);

        RewardClaimTestCase memory testCase1 = fixtures1[0];
        address account = testCase1.account;

        vm.prank(account);
        rewardClaim.claimRewards(100 ether, testCase1.authData);
        assertEq(espToken.balanceOf(account), 100 ether);
        assertEq(rewardClaim.claimedRewards(account), 100 ether);

        (uint256 authRoot2, RewardClaimTestCase[] memory fixtures2) =
            getFixturesWithAccountAndAmount(account, 250 ether);
        lightClient.setAuthRoot(authRoot2);

        RewardClaimTestCase memory testCase2 = fixtures2[0];

        vm.prank(account);
        vm.expectEmit(true, true, true, true);
        emit IRewardClaim.RewardsClaimed(account, 150 ether);
        rewardClaim.claimRewards(250 ether, testCase2.authData);

        assertEq(espToken.balanceOf(account), 250 ether);
        assertEq(rewardClaim.claimedRewards(account), 250 ether);
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
