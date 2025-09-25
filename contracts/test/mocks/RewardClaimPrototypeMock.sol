// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "../../src/libraries/RewardMerkleTreeVerifier.sol";
import "../../src/interfaces/IRewardClaim.sol";

/**
 * @title RewardClaimPrototypeMock
 * @dev Mock contract for testing reward merkle tree verification
 *
 * This contract currently only exists to make it possible to call the verifier
 * library.
 */
contract RewardClaimPrototypeMock {
    /**
     * @dev Verify a reward claim using merkle proof
     * @param commitment The reward commitment to verify against
     * @param account The account claiming the reward
     * @param amount The reward amount being claimed
     * @param proof The merkle proof for the claim
     * @return true if the claim is valid
     */
    function verifyRewardClaim(
        bytes32 commitment,
        address account,
        uint256 lifetimeRewards,
        bytes authData,
        IRewardClaim.LifetimeRewardsProof calldata proof
    ) external pure returns (bool) {
        return RewardMerkleTreeVerifier.computeRoot(account, amount, proof) == commitment;
    }
}
