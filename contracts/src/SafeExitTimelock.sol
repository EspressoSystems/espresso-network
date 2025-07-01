//SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/governance/TimelockController.sol";

/// @title SafeExitTimelock
/// @notice A timelock controller for contracts that can have a long delay before updates are applied
/// @dev The delay on the contract is long enough for users to exit the system if they do not agree
/// with the update
contract SafeExitTimelock is TimelockController {
    constructor(
        uint256 minDelay,
        address[] memory proposers,
        address[] memory executors,
        address admin
    ) TimelockController(minDelay, proposers, executors, admin) { }
}
