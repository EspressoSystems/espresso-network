// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { UUPSUpgradeable } from
    "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";
import { PausableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import { AccessControlUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import { ReentrancyGuardUpgradeable } from
    "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import "./LightClientV3.sol";
import "./EspTokenV2.sol";
import "./libraries/RewardMerkleTreeVerifier.sol";
import "./interfaces/IRewardClaim.sol";

contract RewardClaim is
    IRewardClaim,
    Initializable,
    OwnableUpgradeable,
    UUPSUpgradeable,
    PausableUpgradeable,
    AccessControlUpgradeable,
    ReentrancyGuardUpgradeable
{
    /// @notice The ESP token contract
    EspTokenV2 public espToken;

    /// @notice The light client contract
    LightClientV3 public lightClient;

    /// @notice Tracks total lifetime rewards claimed by each address
    mapping(address claimer => uint256 claimed) public claimedRewards;

    /// @notice Maximum amount (in Wei) that can be claimed per day across all claimers
    ///
    /// @dev Daily limits provide defense-in-depth security: in the unlikely event an exploit for
    /// the merkle proof verification is discovered, at most the daily limit can be minted before
    /// the contract is paused by the PAUSER_ROLE. This offers a second layer of protection beyond
    /// cryptographic verification.
    ///
    /// @dev This parameter is intentionally kept non-dynamic such that inflating the token
    /// `totalSupply` will not inflate the value of this limiting parameter.
    uint256 public dailyLimitWei;

    /// @notice Basis points used when daily limit was last set (for reference only)
    /// @dev This is a snapshot of the basis points parameter from the last setDailyLimit call.
    /// As total supply changes, this value becomes outdated and no longer represents the actual
    /// percentage that dailyLimitWei represents relative to current supply.
    uint256 public lastSetDailyLimitBasisPoints;

    /// @notice Maximum daily limit as percentage of total supply in basis points (500 = 5%)
    /// @dev Hardcoded to prevent setting dangerously high limits without a contract upgrade.
    /// Increasing this value further would require upgrading the contract, which is
    /// intentional to ensure careful consideration and governance of security parameters.
    uint256 public constant MAX_DAILY_LIMIT_BASIS_POINTS = 500; // 5%

    /// @notice Current day number (days since epoch)
    uint256 private _currentDay;

    /// @notice Amount claimed today across all claimers
    ///
    /// @dev No view functions provided for _currentDay or _claimedToday to avoid race
    /// conditions. Clients should use call/estimateGas on claimRewards() to check if a
    /// claim would succeed. Honest claims should never hit rate limits under normal
    /// operation.
    ///
    /// @dev It may be potentially useful to add a getter for when the daily limit will
    /// reset. We don't expect to hit the daily limits, therefore implementation in the
    /// contract and in clients is not part of the initial release.
    uint256 private _claimedToday;

    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

    /// @notice The proxy updates the implementation address
    event Upgrade(address implementation);

    /// @notice The daily limit is updated
    event DailyLimitUpdated(uint256 oldLimit, uint256 newLimit);

    /// @notice Attempting to set daily limit to zero
    error ZeroDailyLimit();

    /// @notice Attempting to set daily limit above the maximum allowed percentage
    error DailyLimitTooHigh();

    /// @notice Attempting to set daily limit to the current value
    error NoChangeRequired();

    /// @notice Total ESP token supply is zero during initialization
    error ZeroTotalSupply();

    /// @notice Pauser address is zero during initialization
    error ZeroPauserAddress();

    /// @notice Light client address is zero during initialization
    error ZeroLightClientAddress();

    /// @notice ESP token address is zero during initialization
    error ZeroTokenAddress();

    constructor() {
        _disableInitializers();
    }

    /// @notice Initializes the RewardClaim contract
    /// @param _owner Address that will own the contract
    /// @param _espToken Address of the ESP token contract
    /// @param _lightClient Address of the light client contract
    /// @param _pauser Address to be granted the pauser role
    /// @dev Sets daily limit to 1% of total ESP token supply
    function initialize(address _owner, address _espToken, address _lightClient, address _pauser)
        external
        virtual
        initializer
    {
        // NOTE: __Ownable_init checks _owner != address(0)
        require(_lightClient != address(0), ZeroLightClientAddress());
        require(_pauser != address(0), ZeroPauserAddress());
        require(_espToken != address(0), ZeroTokenAddress());

        // NOTE: external call
        uint256 totalSupply = EspTokenV2(_espToken).totalSupply();
        require(totalSupply > 0, ZeroTotalSupply());

        // Set initial daily limit to 1% (100 basis points) of total supply
        uint256 initialBps = 100; // 1%
        uint256 _dailyLimit = (totalSupply * initialBps) / 10000;
        require(_dailyLimit > 0, ZeroDailyLimit());

        __Ownable_init(_owner);
        __UUPSUpgradeable_init();
        __Pausable_init();
        __AccessControl_init();
        __ReentrancyGuard_init();

        _grantRole(DEFAULT_ADMIN_ROLE, _owner);
        _grantRole(PAUSER_ROLE, _pauser);

        espToken = EspTokenV2(_espToken);
        lightClient = LightClientV3(_lightClient);

        dailyLimitWei = _dailyLimit;
        lastSetDailyLimitBasisPoints = initialBps;
        _currentDay = block.timestamp / 1 days;
    }

    function pause() external virtual onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external virtual onlyRole(PAUSER_ROLE) {
        _unpause();
    }

    /// @notice Updates the daily limit
    ///
    /// @notice This function computes an absolute daily limit in Wei by multiplying the supplied
    /// basis points with the current total supply of ESP tokens.
    ///
    /// @param basisPoints Daily limit as basis points of current total supply (1-500 for 0.01%-5%)
    ///
    /// @dev nonReentrant protects against reentrancy during the external call to `totalSupply`.
    /// @dev Unlikely to be exploited: we are calling our token, but the token is upgradable.
    /// @dev DO NOT REMOVE: Added for defense-in-depth.
    function setDailyLimit(uint256 basisPoints) external virtual onlyOwner nonReentrant {
        require(basisPoints > 0, ZeroDailyLimit());
        require(basisPoints <= MAX_DAILY_LIMIT_BASIS_POINTS, DailyLimitTooHigh());
        uint256 newLimit = (espToken.totalSupply() * basisPoints) / 10000;
        require(newLimit > 0, ZeroDailyLimit());

        // Due to computation based on current total supply, the new limit is very unlikely to be
        // equal to the old limit. Likely an operator error, therefore revert.
        require(newLimit != dailyLimitWei, NoChangeRequired());

        emit DailyLimitUpdated(dailyLimitWei, newLimit);
        dailyLimitWei = newLimit;
        lastSetDailyLimitBasisPoints = basisPoints;
    }

    /// @notice Claim all unclaimed staking rewards
    /// @param lifetimeRewards Total earned lifetime rewards for the user
    /// @param authData Authentication data from Espresso query service
    ///
    /// @dev nonReentrant is not strictly necessary:
    ///
    /// - claimedRewards updated before external call
    /// - re-entrancy would change msg.sender making proof verification fail
    /// - we are calling _our_ token
    ///
    /// @dev The token is upgradable, the modifier makes re-entrancy simpler to reason about.
    /// @dev DO NOT REMOVE: added for defense-in-depth and clarity.
    /// @dev See RewardClaim.Reentrancy.Unit.t.sol for regression test.
    function claimRewards(uint256 lifetimeRewards, bytes calldata authData)
        external
        virtual
        whenNotPaused
        nonReentrant
    {
        require(lifetimeRewards != 0, InvalidRewardAmount());
        address claimer = msg.sender;
        require(lifetimeRewards > claimedRewards[claimer], AlreadyClaimed());

        uint256 amountToClaim = lifetimeRewards - claimedRewards[claimer];
        _enforceDailyLimit(amountToClaim);

        require(_verifyAuthRoot(lifetimeRewards, authData), InvalidAuthRoot());

        claimedRewards[claimer] = lifetimeRewards;

        espToken.mint(claimer, amountToClaim);

        emit RewardsClaimed(claimer, amountToClaim);
    }

    function getVersion()
        external
        pure
        virtual
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (1, 0, 0);
    }

    function _enforceDailyLimit(uint256 amount) internal virtual {
        uint256 today = block.timestamp / 1 days;
        if (today != _currentDay) {
            _currentDay = today;
            _claimedToday = 0;
        }
        _claimedToday += amount;
        if (_claimedToday > dailyLimitWei) {
            revert DailyLimitExceeded();
        }
    }

    function _authorizeUpgrade(address newImplementation) internal virtual override onlyOwner {
        emit Upgrade(newImplementation);
    }

    function _verifyAuthRoot(uint256 lifetimeRewards, bytes calldata authData)
        internal
        view
        virtual
        returns (bool)
    {
        (bytes32[160] memory proof, bytes32[7] memory authRootInputs) =
            abi.decode(authData, (bytes32[160], bytes32[7]));

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
