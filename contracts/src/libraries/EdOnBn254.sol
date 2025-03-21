// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { Utils } from "bn254/BN254.sol";

/// @notice Edward curve on BN254.
/// This library only implements a serialization function that is consistent with
/// Arkworks' format. It does not support any group operations.
library EdOnBN254 {
    uint256 public constant P_MOD =
        21888242871839275222246405745257275088548364400416034343698204186575808495617;

    struct EdOnBN254Point {
        uint256 x;
        uint256 y;
    }

    /// @dev Check if y-coordinate of G1 point is negative.
    function isYNegative(EdOnBN254Point memory point) internal pure returns (bool) {
        return (point.y << 1) < P_MOD;
    }

    /// @dev Serialize a point into bytes
    function serialize(EdOnBN254Point memory point) internal pure returns (bytes memory res) {
        uint256 mask = 0;
        // Edward curve does not have an infinity flag.
        // Set the 255-th bit to 1 for positive Y
        // See:
        // https://github.com/arkworks-rs/algebra/blob/d6365c3a0724e5d71322fe19cbdb30f979b064c8/serialize/src/flags.rs#L148
        if (!EdOnBN254.isYNegative(point)) {
            mask = 0x8000000000000000000000000000000000000000000000000000000000000000;
        }

        return abi.encodePacked(Utils.reverseEndianness(point.x | mask));
    }

    /// @dev Check if two points are equal
    function isEqual(EdOnBN254Point memory a, EdOnBN254Point memory b)
        internal
        pure
        returns (bool)
    {
        return a.x == b.x && a.y == b.y;
    }

    // TODO: (alex) add `validatePoint` methods and tests
}
