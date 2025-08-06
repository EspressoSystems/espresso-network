// SPDX-License-Identifier: MIT
pragma solidity ^0.8.28;

import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { UUPSUpgradeable } from
    "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import "./LightClientV3.sol";
import "./EspTokenV2.sol";
import "./libraries/RewardMerkleTreeVerifier.sol";

contract RewardClaim is Initializable, OwnableUpgradeable, UUPSUpgradeable {
    using RewardMerkleTreeVerifier for bytes32;

    EspTokenV2 public espToken;
    LightClientV3 public lightClient;

    mapping(address => uint256) public claimedRewards;

    event RewardClaimed(address indexed user, uint256 amount);

    error InvalidProof();
    error AlreadyClaimed();
    error InvalidRewardAmount();

    constructor() {
        _disableInitializers();
    }

    function initialize(address owner, address _espToken, address _lightClient)
        public
        initializer
    {
        __Ownable_init(owner);
        __UUPSUpgradeable_init();
        espToken = EspTokenV2(_espToken);
        lightClient = LightClientV3(_lightClient);
    }

    function _authorizeUpgrade(address newImplementation) internal override onlyOwner { }

    function claimRewards(
        uint256 rewardAmount,
        RewardMerkleTreeVerifier.AccruedRewardsProof calldata proof,
        bytes32 rewardMerkleRoot, // TODO authenticate against LC contract
        bytes calldata remainingAuthRootData
    ) external {
        // TODO: Implement function body
        // 1. Verify the user hasn't already claimed this amount
        // 2. Verify the merkle proof against the reward merkle root
        // 3. Reconstruct the authRoot from rewardMerkleRoot + remainingAuthRootData
        // 4. Verify the reconstructed authRoot matches lightClient.authRoot()
        // 5. Mint tokens to the user
        // 6. Update claimedRewards mapping
        revert("Not implemented");
    }

    function getVersion()
        public
        pure
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (1, 0, 0);
    }
}
