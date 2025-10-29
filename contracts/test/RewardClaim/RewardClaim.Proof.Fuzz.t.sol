// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

/* solhint-disable func-name-mixedcase, no-console */

import "./RewardClaim.t.sol";

/// forge-config: quick.fuzz.runs = 1
contract RewardClaimProofFuzzTest is RewardClaimTest {
    function testFuzz_ValidProofs_AlwaysSucceed(uint256 numAccounts, uint64 seed) public {
        numAccounts = bound(numAccounts, 1, 1000);

        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) =
            getFixturesWithSeed(numAccounts, seed);
        lightClient.setAuthRoot(authRoot);

        for (uint256 i = 0; i < fixtures.length; i++) {
            RewardClaimTestCase memory testCase = fixtures[i];

            uint256 balanceBefore = espToken.balanceOf(testCase.account);

            vm.prank(testCase.account);
            rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);

            assertEq(espToken.balanceOf(testCase.account), balanceBefore + testCase.lifetimeRewards);
        }
    }

    function testFuzz_RandomAuthData_AlwaysFails(bytes memory randomAuthData, uint64 seed) public {
        (uint256 authRoot, RewardClaimTestCase memory validCase) = getFixture(seed);
        lightClient.setAuthRoot(authRoot);

        validateTestCase(validCase, authRoot);

        vm.prank(validCase.account);
        vm.expectRevert();
        rewardClaim.claimRewards(validCase.lifetimeRewards, randomAuthData);
    }

    function testFuzz_RandomProof_ValidAuthRootInputs_AlwaysFails(
        bytes32[160] memory randomProof,
        uint64 seed
    ) public {
        (uint256 authRoot, RewardClaimTestCase memory validCase) = getFixture(seed);
        lightClient.setAuthRoot(authRoot);

        validateTestCase(validCase, authRoot);

        (, bytes32[7] memory validAuthRootInputs) =
            abi.decode(validCase.authData, (bytes32[160], bytes32[7]));

        bytes memory invalidAuthData = abi.encode(randomProof, validAuthRootInputs);

        vm.prank(validCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(validCase.lifetimeRewards, invalidAuthData);
    }

    function testFuzz_ValidProof_RandomAuthRootInputs_AlwaysFails(
        bytes32[7] memory randomAuthRootInputs,
        uint64 seed
    ) public {
        (uint256 authRoot, RewardClaimTestCase memory validCase) = getFixture(seed);
        lightClient.setAuthRoot(authRoot);

        validateTestCase(validCase, authRoot);

        (bytes32[160] memory validProof,) =
            abi.decode(validCase.authData, (bytes32[160], bytes32[7]));

        bytes memory invalidAuthData = abi.encode(validProof, randomAuthRootInputs);

        vm.prank(validCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(validCase.lifetimeRewards, invalidAuthData);
    }

    function testFuzz_TruncatedAuthData_AlwaysReverts(uint256 truncateAt, uint64 seed) public {
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getFixture(seed);
        truncateAt %= testCase.authData.length;
        lightClient.setAuthRoot(authRoot);

        validateTestCase(testCase, authRoot);

        bytes memory truncated = new bytes(truncateAt);
        for (uint256 i = 0; i < truncateAt; i++) {
            truncated[i] = testCase.authData[i];
        }

        vm.prank(testCase.account);
        vm.expectRevert();
        rewardClaim.claimRewards(testCase.lifetimeRewards, truncated);
    }

    function testFuzz_ValidProof_WrongAmount_Fails(uint256 wrongAmount, uint64 seed) public {
        vm.assume(wrongAmount > 0);
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getFixture(seed);
        vm.assume(wrongAmount != testCase.lifetimeRewards);
        lightClient.setAuthRoot(authRoot);

        validateTestCase(testCase, authRoot);

        vm.prank(testCase.account);
        vm.expectRevert();
        rewardClaim.claimRewards(wrongAmount, testCase.authData);
    }

    function testFuzz_ValidProof_WrongSender_Fails(address wrongSender, uint64 seed) public {
        vm.assume(wrongSender != address(0));
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getFixture(seed);
        vm.assume(wrongSender != testCase.account);
        lightClient.setAuthRoot(authRoot);

        validateTestCase(testCase, authRoot);

        vm.prank(wrongSender);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
    }

    function testFuzz_ByteManipulation_AlwaysFails(uint256 byteIndex, uint8 xorMask, uint64 seed)
        public
    {
        vm.assume(xorMask != 0);
        (uint256 authRoot, RewardClaimTestCase memory testCase) = getFixture(seed);
        byteIndex %= testCase.authData.length;
        lightClient.setAuthRoot(authRoot);

        validateTestCase(testCase, authRoot);

        bytes memory corruptedAuthData = testCase.authData;
        corruptedAuthData[byteIndex] ^= bytes1(xorMask);

        vm.prank(testCase.account);
        vm.expectRevert(IRewardClaim.InvalidAuthRoot.selector);
        rewardClaim.claimRewards(testCase.lifetimeRewards, corruptedAuthData);
    }

    function testFuzz_EveryBitFlip_AlwaysFails(uint256 numAccounts, uint64 seed) public {
        numAccounts = bound(numAccounts, 1, 50);

        (uint256 authRoot, RewardClaimTestCase[] memory fixtures) =
            getFixturesWithSeed(numAccounts, seed);
        lightClient.setAuthRoot(authRoot);

        RewardClaimTestCase memory testCase = fixtures[0];

        validateTestCase(testCase, authRoot);

        // This is a reference, but we don't need the original anymore
        bytes memory corruptAuthData = testCase.authData;

        vm.pauseGasMetering();
        for (uint256 byteIndex = 0; byteIndex < corruptAuthData.length; byteIndex++) {
            for (uint256 bitIndex = 0; bitIndex < 8; bitIndex++) {
                bytes1 mask = bytes1(uint8(1 << bitIndex));
                corruptAuthData[byteIndex] ^= mask;

                vm.prank(testCase.account);
                vm.expectRevert();
                rewardClaim.claimRewards(testCase.lifetimeRewards, corruptAuthData);

                // Reuse same memory for corruptions to avoid expensive copies in the loop
                corruptAuthData[byteIndex] ^= mask;
            }
        }
        vm.resumeGasMetering();
    }
}
