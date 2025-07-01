//SPDX-License-Identifier: Unlicense
pragma solidity ^0.8.0;

import "@openzeppelin/contracts/governance/TimelockController.sol";

/// @title OpsTimelock
/// @notice A timelock controller for contracts that require faster updates
/// @dev Timelock used for operational control during early protocol phases.
/// Grants privileged access to core team for upgrades or config changes
/// with a short delay.
contract OpsTimelock is TimelockController {
    constructor(
        uint256 minDelay,
        address[] memory proposers,
        address[] memory executors,
        address admin
    ) TimelockController(minDelay, proposers, executors, admin) { }
}
