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
     * @param root The merkle root to verify against
     * @param account The account claiming the reward
     * @param amount The reward amount being claimed
     * @param proof The merkle proof for the claim
     * @return true if the claim is valid
     */
    function verifyRewardClaim(
        bytes32 root,
        address account,
        uint256 amount,
        bytes32[160] calldata proof
    ) external pure returns (bool) {
        return RewardMerkleTreeVerifier.verifyMembership(root, account, amount, proof);
    }

    // Ensure we test the abi.decoding until we have the full reward claim contract.
    function verifyRewardClaimAuthData(
        bytes32 root,
        address account,
        uint256 amount,
        bytes calldata authData
    ) external view returns (bool) {
        (bytes32[160] memory proof,) = abi.decode(authData, (bytes32[160], bytes32[7]));
        return this.verifyRewardClaim(root, account, amount, proof);
    }
}
