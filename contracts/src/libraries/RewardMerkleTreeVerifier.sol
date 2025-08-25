// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

/* solhint-disable no-inline-assembly */

/**
 * @title RewardMerkleTreeVerifier
 * @dev Solidity verifier for RewardMerkleTreeV2 compatible with Espresso's reward system
 * - Arity: 2 (binary tree)
 * - Depth: 160 levels
 * - Key length: 20 bytes (160 bits) - Ethereum addresses
 * - EVM native keccak hash
 * - Double hashing of leaves as cheap domain separator
 */
library RewardMerkleTreeVerifier {
    error InvalidProofLength();

    uint256 public constant TREE_DEPTH = 160;

    struct AccruedRewardsProof {
        bytes32[] siblings;
    }

    function _hashLeaf(uint256 value) internal pure returns (bytes32) {
        bytes32 firstHash = keccak256(abi.encodePacked(value));
        // Double hashing instead of domain separation
        return keccak256(abi.encodePacked(firstHash));
    }

    function _hashInternal(bytes32 left, bytes32 right) internal pure returns (bytes32) {
        // keccak256(abi.encodePacked(left, right)) in assembly saves about 10%
        // gas for the entire proof verification.
        bytes32 hash;
        assembly {
            let ptr := mload(0x40) // Get free memory pointer
            mstore(ptr, left) // Store left (32 bytes)
            mstore(add(ptr, 0x20), right) // Store right (32 bytes)
            hash := keccak256(ptr, 0x40) // Hash 64 bytes
        }
        return hash;
    }

    /**
     * @dev Compute reward commitment from a key-value pair and proof
     * @param key The key to prove - Ethereum address
     * @param value The value to prove - accrued rewards amount
     * @param proof The membership proof containing sibling hashes and numLeaves
     * @return The computed reward commitment
     */
    function computeRoot(address key, uint256 value, AccruedRewardsProof calldata proof)
        internal
        pure
        returns (bytes32)
    {
        // NOTE: using memory instead of calldata for proof or siblings
        //       increases gas cost by 20%
        // NOTE: *not* defining siblings here increases gas cost by 20%
        // TODO: unittest this function
        // TODO: fuzz test this function
        // TODO: benchmark gas cost by averaging gas cost over many different trees with
        //       realistic size.
        // TODO: optimize gas cost
        bytes32[] calldata siblings = proof.siblings;
        require(siblings.length == TREE_DEPTH, InvalidProofLength());

        bytes32 currentHash = _hashLeaf(value);

        // Traverse from leaf to root using the same pattern as RewardMerkleTreeV2
        for (uint256 level = 0; level < TREE_DEPTH; level++) {
            bytes32 sibling = siblings[level];

            // Extract bit using direct right shift
            bool branch;
            assembly {
                let shifted := shr(level, key)
                branch := and(shifted, 1)
            }

            if (branch) {
                currentHash = _hashInternal(sibling, currentHash);
            } else {
                currentHash = _hashInternal(currentHash, sibling);
            }
        }

        return currentHash;
    }
}
