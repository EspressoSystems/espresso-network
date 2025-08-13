// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import "../../src/libraries/RewardMerkleTreeVerifier.sol";

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
    function verifyAuthRootCommitment(
        bytes32 commitment,
        address account,
        uint256 amount,
        RewardMerkleTreeVerifier.AccruedRewardsProof calldata proof
    ) external pure returns (bool) {
        return
            RewardMerkleTreeVerifier.computeAuthRootCommitment(account, amount, proof) == commitment;
    }
}
