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
import "./interfaces/IRewardClaim.sol";

contract RewardClaim is IRewardClaim, Initializable, OwnableUpgradeable, UUPSUpgradeable {
    using RewardMerkleTreeVerifier for bytes32;

    EspTokenV2 public espToken;
    LightClientV3 public lightClient;

    mapping(address claimer => uint256 claimed) public claimedRewards;

    /// @notice upgrade event when the proxy updates the implementation it's pointing to
    event Upgrade(address implementation);

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

    function claimRewards(uint256 lifetimeRewards, bytes memory authData) external {
        require(lifetimeRewards != 0, InvalidRewardAmount());
        require(claimedRewards[msg.sender] < lifetimeRewards, AlreadyClaimed());

        (bytes32[160] memory proof, bytes32[7] memory authRootInputs) =
            abi.decode(authData, (bytes32[160], bytes32[7]));
        require(_verifyAuthRoot(lifetimeRewards, proof, authRootInputs), InvalidAuthRoot());

        uint256 availableToClaim = lifetimeRewards - claimedRewards[msg.sender];
        claimedRewards[msg.sender] = lifetimeRewards;

        espToken.mint(msg.sender, availableToClaim);

        emit RewardsClaimed(msg.sender, availableToClaim);
    }

    function getVersion()
        public
        pure
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (1, 0, 0);
    }

    function _verifyAuthRoot(
        uint256 lifetimeRewards,
        bytes32[160] memory proof,
        bytes32[7] memory authRootInputs
    ) internal view virtual returns (bool) {
        bytes32 rewardCommitment =
            RewardMerkleTreeVerifier.computeRoot(msg.sender, lifetimeRewards, proof);
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

        return uint256(authRoot) == lightClient.authRoot();
    }
}
