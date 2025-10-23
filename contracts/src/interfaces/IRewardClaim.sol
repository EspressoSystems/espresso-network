// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

interface IRewardClaim {
    /// @notice User claimed rewards
    event RewardsClaimed(address indexed user, uint256 amount);

    /// @notice Unable to authenticate rewards against Light Client contract
    error InvalidAuthRoot();

    /// @notice All available rewards already claimed
    error AlreadyClaimed();

    /// @notice Reward amount must be greater than zero
    error InvalidRewardAmount();

    /// @notice A claim would exceed the remaining daily capacity
    error DailyLimitExceeded();

    /// @notice Claim staking rewards
    ///
    /// @param lifetimeRewards Total earned lifetime rewards for the user @param
    /// @param authData inputs required for authentication of lifetime rewards amount.
    ///
    /// @notice Obtain authData from the Espresso query service API.
    function claimRewards(uint256 lifetimeRewards, bytes calldata authData) external;

    /// @notice Check amount of rewards claimed by a user
    function claimedRewards(address claimer) external view returns (uint256);
}
