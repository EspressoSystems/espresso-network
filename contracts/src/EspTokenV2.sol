// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./EspToken.sol";

/// @title EspTokenV2
/// @notice Upgrades EspToken to allow minting by the RewardClaim contract
/// @dev Upgradeability & storage layout (frozen-base pattern)
/// - V2 inherits from V1 (frozen base): never add, remove, or reorder storage in V1 to avoid
/// collisions.
/// - V2 adds `rewardClaim` (1 slot) and its own `__gap` (50 slots) for future versions.
/// - Future versions (V3, V4, …) should inherit from V2 and append new variables only.
/// - The `__gap` in V2 reserves space for V3+; it's not used by children and should stay untouched.
contract EspTokenV2 is EspToken {
    /// @notice Address of the RewardClaim contract authorized to mint tokens
    /// @notice Can only be set once, during initialization.
    address public rewardClaim;

    /// @dev Storage gap reserved for future versions (V3, V4, etc.)
    uint256[50] private __gap;

    /// @notice A non-RewardClaim address attempts to mint
    error OnlyRewardClaim();

    /// @notice RewardClaim address cannot be zero
    error ZeroRewardClaimAddress();

    constructor() {
        _disableInitializers();
    }

    /// @notice Initializes the V2 upgrade with the RewardClaim contract address
    /// @param _rewardClaim Address of the RewardClaim contract
    function initializeV2(address _rewardClaim) public onlyOwner reinitializer(2) {
        require(_rewardClaim != address(0), ZeroRewardClaimAddress());
        rewardClaim = _rewardClaim;
    }

    /// @notice Mints new tokens to a specified address
    /// @notice Only the RewardClaim contract can mint new tokens
    ///
    /// @param to Address to receive the minted tokens
    /// @param amount Number of tokens to mint
    function mint(address to, uint256 amount) public {
        require(msg.sender == rewardClaim, OnlyRewardClaim());
        _mint(to, amount);
    }

    /// @notice Returns the contract version
    /// @return majorVersion Major version number
    /// @return minorVersion Minor version number
    /// @return patchVersion Patch version number
    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (2, 0, 0);
    }
}
