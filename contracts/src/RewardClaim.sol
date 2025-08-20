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

    mapping(address claimer => uint256 claimed) public claimedRewards;

    /// @notice upgrade event when the proxy updates the implementation it's pointing to
    event Upgrade(address implementation);

    /// @notice User claimed rewards
    event RewardClaimed(address indexed user, uint256 amount);

    error InvalidAuthRoot();
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

    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {
        emit Upgrade(newImplementation);
    }

    function claimRewards(
        uint256 accruedReward,
        RewardMerkleTreeVerifier.AccruedRewardsProof calldata proof,
        bytes32[7] calldata authRootInputs
    ) external {
        require(accruedReward != 0, InvalidRewardAmount());
        require(claimedRewards[msg.sender] < accruedReward, AlreadyClaimed());

        bytes32 rewardCommitment =
            RewardMerkleTreeVerifier.computeAuthRootCommitment(msg.sender, accruedReward, proof);
        bytes32 authRoot = keccak256(
            abi.encodePacked(
                rewardCommitment,
                authRootInputs[0],
                authRootInputs[1],
                authRootInputs[2],
                authRootInputs[3],
                authRootInputs[4],
                authRootInputs[5],
                authRootInputs[6]
            )
        );

        require(uint256(authRoot) == lightClient.authRoot(), InvalidAuthRoot());

        uint256 newClaimAmount = accruedReward - claimedRewards[msg.sender];
        claimedRewards[msg.sender] = accruedReward;

        espToken.mint(msg.sender, newClaimAmount);

        emit RewardClaimed(msg.sender, newClaimAmount);
    }

    function getVersion()
        public
        pure
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (1, 0, 0);
    }
}
