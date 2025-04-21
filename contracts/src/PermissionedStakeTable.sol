// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { InitializedAt } from "./InitializedAt.sol";

/**
 * @title SimpleStakeTable
 * @dev An stake table mapping with owner-only access control.
 */
contract PermissionedStakeTable is Ownable, InitializedAt {
    event StakersUpdated(BN254.G2Point[] removed, NodeInfo[] added);

    error StakerAlreadyExists(BN254.G2Point);
    error StakerNotFound(BN254.G2Point);

    struct NodeInfo {
        /// The consensus signing key
        BN254.G2Point blsVK;
        /// The consensus signing key. Only used for storage in this contract.
        EdOnBN254.EdOnBN254Point schnorrVK;
        /// Is the Node DA Node? Only used for storage in this contract.
        bool isDA;
    }

    // State mapping from staker IDs to their staking status
    mapping(bytes32 nodeID => bool isStaker) private stakers;

    constructor(NodeInfo[] memory initialStakers) Ownable(msg.sender) InitializedAt() {
        // This contract isn't deployed behind a proxy, so we manually initialize the block number.
        initializedAtBlock = block.number;
        insert(initialStakers);
    }

    // public methods

    function update(BN254.G2Point[] memory stakersToRemove, NodeInfo[] memory newStakers)
        public
        onlyOwner
    {
        remove(stakersToRemove);
        insert(newStakers);
        emit StakersUpdated(stakersToRemove, newStakers);
    }

    // internal methods

    function insert(NodeInfo[] memory newStakers) internal {
        require(newStakers.length > 0, "PermissionedStakeTable: no stakers to insert");

        for (uint256 i = 0; i < newStakers.length; i++) {
            bytes32 stakerID = _hashBlsKey(newStakers[i].blsVK);
            if (stakers[stakerID]) {
                revert StakerAlreadyExists(newStakers[i].blsVK);
            }
            stakers[stakerID] = true;
        }
    }

    function remove(BN254.G2Point[] memory stakersToRemove) internal {
        require(stakersToRemove.length > 0, "PermissionedStakeTable: no stakers to remove");

        for (uint256 i = 0; i < stakersToRemove.length; i++) {
            bytes32 stakerID = _hashBlsKey(stakersToRemove[i]);
            if (!stakers[stakerID]) {
                revert StakerNotFound(stakersToRemove[i]);
            }
            stakers[stakerID] = false;
        }
    }

    // view methods

    function isStaker(BN254.G2Point memory staker) external view returns (bool) {
        return stakers[_hashBlsKey(staker)];
    }

    function _hashBlsKey(BN254.G2Point memory blsVK) public pure returns (bytes32) {
        return keccak256(abi.encode(blsVK.x0, blsVK.x1, blsVK.y0, blsVK.y1));
    }
}
