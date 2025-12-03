// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import "../../src/RewardClaim.sol";

contract MockRewardClaim is RewardClaim {
    function _verifyAuthRoot(uint256, bytes calldata) internal pure override returns (bool) {
        return true;
    }
}
