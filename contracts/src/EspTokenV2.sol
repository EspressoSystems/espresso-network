// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./EspToken.sol";

/// @title EspTokenV2
/// @notice Upgrades EspToken to allow minting by the RewardClaim contract
/// @dev Upgradeability & storage layout (frozen-inheritance pattern)
/// We intentionally do not use `__gap` slots. Once a version is deployed,
/// its storage layout is frozen and never modified. New state variables are
/// added only in a new child contract (V2, V3, …) that inherits from the
/// previous version and appends fields at the end. This preserves slot order
/// across upgrades without relying on gaps. (Note: upstream OZ parents may
/// include their own gaps—those remain untouched.)
contract EspTokenV2 is EspToken {
    /// @notice Address of the RewardClaim contract authorized to mint tokens
    /// @notice Can only be set once, during initialization.
    address public rewardClaim;

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
