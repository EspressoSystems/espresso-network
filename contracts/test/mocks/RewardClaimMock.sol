// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "../../src/RewardClaim.sol";

contract RewardClaimMock is RewardClaim {
    function _verifyAuthRoot(
        uint256,
        RewardMerkleTreeVerifier.AccruedRewardsProof calldata,
        bytes32[7] calldata
    ) internal pure override returns (bool) {
        return true;
    }
}
