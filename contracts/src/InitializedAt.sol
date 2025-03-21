// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

// Store the block number when a contract was deployed, or initialized (for upgradable contracts).
//
// Clients can read the member variable `initializedAtBlock` to know at what L1 block they need to
// start processing events.

import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";

contract InitializedAt is Initializable {
    // @notice The block number the contract was initialized at.
    uint256 public initializedAtBlock;

    constructor() {
        _disableInitializers();
    }

    // @dev The `initializeAtBlock` function must be called during initialization.
    function initializeAtBlock() internal initializer {
        initializedAtBlock = block.number;
    }
}
