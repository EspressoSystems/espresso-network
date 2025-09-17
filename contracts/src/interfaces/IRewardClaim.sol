// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

interface IRewardClaim {
    /// @notice Proof for rewards merkle tree. Obtained from Espresso query service API.
    struct LifetimeRewardsProof {
        bytes32[] siblings;
    }

    /// @notice User claimed rewards
    event RewardClaimed(address indexed user, uint256 amount);

    /// @notice Unable to authenticate rewards against Light Client contract
    error InvalidAuthRoot();

    /// @notice All available rewards already claimed
    error AlreadyClaimed();

    /// @notice Reward amount must be greater than zero
    error InvalidRewardAmount();

    /// @notice Claim accrued rewards
    ///
    /// @param totalEarnedRewards Total earned rewards for the user
    /// @param proof Merkle proof attesting to earned rewards for the user
    /// @param authRootInputs The authRootInputs must all be zero at the moment,
    ///        this may change in the future with Espresso protocol upgrades.
    ///
    /// @notice Obtain rewards and proof from the Espresso query service API.
    function claimRewards(
        uint256 totalEarnedRewards,
        LifetimeRewardsProof calldata proof,
        bytes32[7] calldata authRootInputs
    ) external;

    /// @notice Check amount of rewards claimed by a user
    function claimedRewards(address claimer) external view returns (uint256);
}
