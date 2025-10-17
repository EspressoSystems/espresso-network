// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { PausableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import { AccessControlUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import { StakeTable } from "./StakeTable.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";
import { SafeTransferLib } from "solmate/utils/SafeTransferLib.sol";

/// @title Ethereum L1 component of the Espresso Global Confirmation Layer (GCL) stake table.
///
/// @dev All functions are marked as virtual so that future upgrades can override them.
///
/// @notice This contract is an upgrade to the original StakeTable contract. On Espresso mainnet we
/// will only use the V2 contract. On decaf the V2 is used to upgrade the V1 that was first deployed
/// with the original proof of stake release.
///
/// @notice The V2 contract contains the following changes:
///
/// 1. The functions to register validators and update consensus keys are updated to require both a
/// BLS signature and a Schnorr signature and emit the signatures via events so that the GCL can
/// verify them. The new functions and events have a V2 postfix. After the upgrade components that
/// support registration and key updates must use the V2 functions and listen to the V2 events. The
/// original functions revert with a `DeprecatedFunction` error in V2.
///
/// 2. The exit escrow period can be updated by the owner of the contract
/// within valid bounds (15 blocks to 14 days).
///
/// 3. The following functions can be paused by the PAUSER_ROLE:
/// - `claimWithdrawal(...)`
/// - `claimValidatorExit(...)`
/// - `delegate(...)`
/// - `undelegate(...)`
/// - `deregisterValidator(...)`
/// - `registerValidatorV2(...)`
/// - `updateConsensusKeysV2(...)`
/// When paused, these functions revert with a standard pausable error, `EnforcedPause()`.
/// Only the PAUSER_ROLE can pause/unpause the contract.
///
/// Note: `updateExitEscrowPeriod` is NOT pausable for emergency governance access.
///
/// 4. The `claimValidatorExit` function is overridden to ensure
/// that the validator's delegatedAmount is updated during this method
/// The update is deferred until the funds are actually withdrawn.
///
/// 5. The `deregisterValidator` function is overridden to ensure
/// that the validator's delegatedAmount is not updated during this method as it was in v1.
///
/// 6. The `updateExitEscrowPeriod` function is added to allow governance to update
/// the exit escrow period within valid bounds (15 blocks to 14 days).
///
/// 7. The `pause` and `unpause` functions are added for emergency control.
///
/// 8. The commission rate for validators can be updated with the `updateCommission` function.
///
/// 9. The `activeStake` variable is added to allow governance to
/// track the total stake in the contract. The activeStake is the
/// total stake that is not awaiting exit or in exited state.
///
/// @notice The StakeTableV2 contract ABI is a superset of the original ABI. Consumers of the
/// contract can use the V2 ABI, even if they would like to maintain backwards compatibility.
contract StakeTableV2 is StakeTable, PausableUpgradeable, AccessControlUpgradeable {
    // === Types ===

    /// @notice Struct for tracking validator commission and last increase time
    struct CommissionTracking {
        uint16 commission;
        uint256 lastIncreaseTime;
    }

    /// @notice Struct for initializing validator commissions during migration
    struct InitialCommission {
        address validator;
        uint16 commission;
    }

    // === Storage ===

    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

    /// @notice Minimum time interval between commission increases (in seconds)
    uint256 public minCommissionIncreaseInterval;

    /// @notice Maximum commission increase allowed per increase (in basis points)
    uint16 public maxCommissionIncrease;

    /// @notice Total stake in active (not marked for exit) validators in the contract
    uint256 public activeStake;

    /// @notice Commission tracking for each validator
    mapping(address validator => CommissionTracking tracking) public commissionTracking;

    /// Schnorr keys that have been seen by the contract
    ///
    /// @dev ensures a bijective mapping between schnorr key and ethereum account and prevents some
    /// errors due to
    /// misconfigurations of validators the contract currently marks keys as used and only allow
    /// them to be used once. This for example prevents callers from accidentally registering the
    /// same Schnorr key twice.
    mapping(bytes32 schnorrKey => bool used) public schnorrKeys;

    // === Events ===

    /// @notice A validator is registered in the stake table
    /// @notice the blsSig and schnorrSig are validated by the Espresso Network
    event ValidatorRegisteredV2(
        address indexed account,
        BN254.G2Point blsVK,
        EdOnBN254.EdOnBN254Point schnorrVK,
        uint16 commission,
        BN254.G1Point blsSig,
        bytes schnorrSig
    );

    /// @notice A validator updates their consensus keys
    /// @notice the blsSig and schnorrSig are validated by the Espresso Network
    event ConsensusKeysUpdatedV2(
        address indexed account,
        BN254.G2Point blsVK,
        EdOnBN254.EdOnBN254Point schnorrVK,
        BN254.G1Point blsSig,
        bytes schnorrSig
    );

    /// @notice The exit escrow period is updated
    event ExitEscrowPeriodUpdated(uint64 newExitEscrowPeriod);

    /// @notice A validator updates their commission rate
    /// @param validator The address of the validator
    /// @param timestamp The timestamp of the update
    /// @param newCommission The new commission rate
    ///
    /// @dev the timestamp is emitted to simplify processing in the GCL
    event CommissionUpdated(
        address indexed validator, uint256 timestamp, uint16 oldCommission, uint16 newCommission
    );

    /// @notice The minimum commission update interval is updated
    /// @param newInterval The new minimum update interval in seconds
    event MinCommissionUpdateIntervalUpdated(uint256 newInterval);

    /// @notice The maximum commission increase is updated
    /// @param newMaxIncrease The new maximum commission increase in basis points
    event MaxCommissionIncreaseUpdated(uint16 newMaxIncrease);

    // === Errors ===

    /// The Schnorr signature is invalid (either the wrong length or the wrong key)
    error InvalidSchnorrSig();

    /// The function is deprecated as it was replaced by a new function
    error DeprecatedFunction();

    /// The commission update is too soon after the last update
    error CommissionUpdateTooSoon();

    /// The commission increase exceeds the maximum allowed increase
    error CommissionIncreaseExceedsMax();

    /// The commission value is unchanged
    error CommissionUnchanged();

    /// The rate limit parameters are invalid
    error InvalidRateLimitParameters();

    /// The validator commission has already been initialized
    error CommissionAlreadyInitialized(address validator);

    /// The initial active stake exceeds the balance of the contract
    error InitialActiveStakeExceedsBalance();

    /// The Schnorr key has been previously registered in the contract.
    error SchnorrKeyAlreadyUsed();

    /// @notice Constructor
    /// @dev This function is overridden to disable initializers
    constructor() {
        _disableInitializers();
    }

    /// @notice Reinitialize the contract
    ///
    /// @param admin The address to be granted the default admin role
    /// @param pauser The address to be granted the pauser role
    /// @param initialActiveStake The initial active stake in the contract
    /// @param initialCommissions commissions of validators
    ///
    /// @notice initialCommissions must be an empty array if the contract we're
    /// upgrading has not been used before (e.g. on mainnet). On decaf (sepolia),
    /// this must be called with the current commissions of pre-existing
    /// validators read from L1 events.
    ///
    /// @dev This function is overridden to add pauser and admin roles
    function initializeV2(
        address pauser,
        address admin,
        uint256 initialActiveStake,
        InitialCommission[] calldata initialCommissions
    ) public onlyOwner reinitializer(2) {
        __AccessControl_init();

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(PAUSER_ROLE, pauser);

        // Default values found to be reasonable in internal discussion, may be
        // adjusted before release and updated after release.
        minCommissionIncreaseInterval = 7 days;
        maxCommissionIncrease = 500; // 5%

        // initialize commissions (if the contract under upgrade has existing state)
        _initializeCommissions(initialCommissions);
        _initializeActiveStake(initialActiveStake);
    }

    /// @notice Get the version of the contract
    /// @dev This function is overridden to return the version of the contract
    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (2, 0, 0);
    }

    /// @notice Pause the contract
    /// @dev This function is only callable by the PAUSER_ROLE
    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    /// @notice Unpause the contract
    /// @dev This function is only callable by the PAUSER_ROLE
    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }

    /// @notice Withdraw previously delegated funds after a validator has exited
    /// @param validator The validator to withdraw from
    /// @dev This function is overridden to deduct the amount from the validator's delegatedAmount
    /// @dev and to add pausable functionality
    /// @dev since the delegated Amount is no longer updated during validator exit
    function claimValidatorExit(address validator) public virtual override whenNotPaused {
        address delegator = msg.sender;
        uint256 unlocksAt = validatorExits[validator];
        if (unlocksAt == 0) {
            revert ValidatorNotExited();
        }

        if (block.timestamp < unlocksAt) {
            revert PrematureWithdrawal();
        }

        uint256 amount = delegations[validator][delegator];
        if (amount == 0) {
            revert NothingToWithdraw();
        }

        // Mark funds as spent
        delegations[validator][delegator] = 0;
        // the delegatedAmount is updated here (instead of during deregistration) in v2,
        // it's only decremented during withdrawal
        validators[validator].delegatedAmount -= amount;

        SafeTransferLib.safeTransfer(token, delegator, amount);

        emit Withdrawal(delegator, amount);
    }

    /// @notice Withdraw previously delegated funds after a validator has exited
    /// @param validator The validator to withdraw from
    /// @dev This function is overridden to add pausable functionality
    function claimWithdrawal(address validator) public virtual override whenNotPaused {
        super.claimWithdrawal(validator);
    }

    /// @notice Delegate funds to a validator
    /// @param validator The validator to delegate to
    /// @param amount The amount to delegate
    /// @dev This function is overridden to add pausable functionality
    function delegate(address validator, uint256 amount) public virtual override whenNotPaused {
        super.delegate(validator, amount);
        activeStake += amount;
    }

    /// @notice Undelegate funds from a validator
    /// @param validator The validator to undelegate from
    /// @param amount The amount to undelegate
    /// @dev This function is overridden to add pausable functionality
    function undelegate(address validator, uint256 amount) public virtual override whenNotPaused {
        super.undelegate(validator, amount);
        activeStake -= amount;
    }

    /// @notice Deregister a validator
    /// @dev This function is overridden to add pausable functionality
    /// @dev and to ensure that the validator's delegatedAmount is not updated until withdrawal
    /// @dev delegatedAmount represents the no. of tokens that have been delegated to a validator,
    /// even if it's not participating in consensus
    function deregisterValidator() public virtual override whenNotPaused {
        address validator = msg.sender;
        ensureValidatorActive(validator);

        validators[validator].status = ValidatorStatus.Exited;
        validatorExits[validator] = block.timestamp + exitEscrowPeriod;
        // in v2, the delegatedAmount is not updated until withdrawal

        activeStake -= validators[validator].delegatedAmount;
        emit ValidatorExit(validator);
    }

    /// @notice Register a validator in the stake table
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key
    /// @param blsSig The BLS signature that authenticates the BLS VK
    /// @param schnorrSig The Schnorr signature that authenticates the Schnorr VK
    /// @param commission in % with 2 decimals, from 0.00% (value 0) to 100% (value 10_000)
    /// @dev This function is overridden to add pausable functionality
    /// @dev and to add schnorrSig validation
    function registerValidatorV2(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission
    ) external virtual whenNotPaused {
        address validator = msg.sender;

        ensureValidatorNotRegistered(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        ensureNewKeys(blsVK, schnorrVK);

        // Verify that the validator can sign for that blsVK. This prevents rogue public-key
        // attacks.
        bytes memory message = abi.encode(validator);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        // ensure that the schnorrSig is the correct length
        if (schnorrSig.length != 64) {
            revert InvalidSchnorrSig();
        }

        if (commission > 10000) {
            revert InvalidCommission();
        }

        blsKeys[_hashBlsKey(blsVK)] = true;
        schnorrKeys[_hashSchnorrKey(schnorrVK)] = true;
        validators[validator] = Validator({ status: ValidatorStatus.Active, delegatedAmount: 0 });

        // Store the initial commission for this validator
        commissionTracking[validator] =
            CommissionTracking({ commission: commission, lastIncreaseTime: 0 });

        emit ValidatorRegisteredV2(validator, blsVK, schnorrVK, commission, blsSig, schnorrSig);
    }

    /// @notice Update the consensus keys of a validator
    ///
    /// @param blsVK The new BLS verification key
    /// @param schnorrVK The new Schnorr verification key
    /// @param blsSig The BLS signature that authenticates the blsVK
    /// @param schnorrSig The Schnorr signature that authenticates the schnorrVK
    /// @dev This function is overridden to add pausable functionality
    /// @dev and to add schnorrSig validation
    function updateConsensusKeysV2(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig
    ) public virtual whenNotPaused {
        address validator = msg.sender;

        ensureValidatorActive(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        ensureNewKeys(blsVK, schnorrVK);

        // Verify that the validator can sign for that blsVK. This prevents rogue public-key
        // attacks.
        bytes memory message = abi.encode(validator);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        blsKeys[_hashBlsKey(blsVK)] = true;

        emit ConsensusKeysUpdatedV2(validator, blsVK, schnorrVK, blsSig, schnorrSig);
    }

    /// @notice Update the commission rate for a validator
    /// @param newCommission The new commission rate in % with 2 decimals (0 to 10_000)
    /// @notice
    ///
    ///   1. Only one commission *increase* per minCommissionIncreaseInterval is allowed.
    ///   2. The commission increase cannot exceed maxCommissionIncrease.
    ///
    /// These limits protect stakers from sudden large commission increases,
    /// particularly by exiting validators.
    function updateCommission(uint16 newCommission) external virtual whenNotPaused {
        address validator = msg.sender;
        ensureValidatorActive(validator);
        require(newCommission <= 10000, InvalidCommission());

        CommissionTracking storage tracking = commissionTracking[validator];
        uint16 currentCommission = tracking.commission;
        require(newCommission != currentCommission, CommissionUnchanged());

        // NOTE: Limits exist to protect stakers from sudden loss of revenue due
        // to commission increases.
        //
        // 1. Limits only enforced for commission *increase*.
        // 2. Time of last change only tracked for commission *increase*.
        if (newCommission > currentCommission) {
            // Allow immediate first increase or after interval has passed
            uint256 lastIncreaseTime = tracking.lastIncreaseTime;
            require(
                lastIncreaseTime == 0
                    || block.timestamp >= lastIncreaseTime + minCommissionIncreaseInterval,
                CommissionUpdateTooSoon()
            );

            // both maxCommissionIncrease and newCommission are <= 10_000
            require(
                newCommission <= currentCommission + maxCommissionIncrease,
                CommissionIncreaseExceedsMax()
            );
            tracking.lastIncreaseTime = block.timestamp;
        }

        tracking.commission = newCommission;

        emit CommissionUpdated(validator, block.timestamp, currentCommission, newCommission);
    }

    /// @notice Set the minimum interval between commission updates
    /// @param newInterval The new minimum interval in seconds
    function setMinCommissionUpdateInterval(uint256 newInterval) external virtual onlyOwner {
        require(newInterval > 0 && newInterval <= 365 days, InvalidRateLimitParameters());
        minCommissionIncreaseInterval = newInterval;
        emit MinCommissionUpdateIntervalUpdated(newInterval);
    }

    /// @notice Set the maximum commission increase allowed per update
    /// @param newMaxIncrease The new maximum increase in basis points (e.g., 500 = 5%)
    function setMaxCommissionIncrease(uint16 newMaxIncrease) external virtual onlyOwner {
        require(newMaxIncrease > 0 && newMaxIncrease <= 10000, InvalidRateLimitParameters());
        maxCommissionIncrease = newMaxIncrease;
        emit MaxCommissionIncreaseUpdated(newMaxIncrease);
    }

    /// @notice Initialize validator commissions during V2 migration
    /// @dev This function is used to retroactively initialize commission storage for validators
    /// that were registered before the V2 upgrade. On decaf, this will be called with current
    /// commission values read from L1 events. On mainnet, this will be called with an empty array
    /// since there are no pre-existing validators.
    /// @param initialCommissions Array of InitialCommission structs containing validator addresses
    /// and their commissions
    function _initializeCommissions(InitialCommission[] calldata initialCommissions) private {
        for (uint256 i = 0; i < initialCommissions.length; i++) {
            address validator = initialCommissions[i].validator;
            uint16 commission = initialCommissions[i].commission;

            require(commission <= 10000, InvalidCommission());

            ValidatorStatus status = validators[validator].status;
            require(status != ValidatorStatus.Unknown, ValidatorInactive());

            require(
                commissionTracking[validator].lastIncreaseTime == 0
                    && commissionTracking[validator].commission == 0,
                CommissionAlreadyInitialized(validator)
            );

            commissionTracking[validator] =
                CommissionTracking({ commission: commission, lastIncreaseTime: 0 });
        }
    }

    /// @notice Initialize the active stake in the contract
    /// @param initialActiveStake The initial active stake in the contract
    function _initializeActiveStake(uint256 initialActiveStake) private {
        require(
            initialActiveStake <= token.balanceOf(address(this)), InitialActiveStakeExceedsBalance()
        );

        activeStake = initialActiveStake;
    }

    /// @notice Update the exit escrow period
    /// @param newExitEscrowPeriod The new exit escrow period
    /// @dev This function ensures that the exit escrow period is within the valid range
    /// @dev This function is not pausable so that governance can perform emergency updates in the
    /// presence of system
    function updateExitEscrowPeriod(uint64 newExitEscrowPeriod) external virtual onlyOwner {
        uint64 minExitEscrowPeriod = lightClient.blocksPerEpoch() * 15; // assuming 15 seconds per
            // block
        uint64 maxExitEscrowPeriod = 86400 * 14; // 14 days

        if (newExitEscrowPeriod < minExitEscrowPeriod || newExitEscrowPeriod > maxExitEscrowPeriod)
        {
            revert ExitEscrowPeriodInvalid();
        }
        exitEscrowPeriod = newExitEscrowPeriod;
        emit ExitEscrowPeriodUpdated(newExitEscrowPeriod);
    }

    function _hashSchnorrKey(EdOnBN254.EdOnBN254Point memory schnorrVK)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(abi.encode(schnorrVK.x, schnorrVK.y));
    }

    /// @notice Ensure that the BLS and Schnorr keys are not already used
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key
    function ensureNewKeys(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK)
        internal
        view
    {
        if (blsKeys[_hashBlsKey(blsVK)]) {
            revert BlsKeyAlreadyUsed();
        }

        if (schnorrKeys[_hashSchnorrKey(schnorrVK)]) {
            revert SchnorrKeyAlreadyUsed();
        }
    }

    // deprecate previous registration function
    /// @notice Deprecate previous registration function
    /// @dev This function is overridden to revert with a DeprecatedFunction error
    /// @dev users must call registerValidatorV2 instead
    function registerValidator(
        BN254.G2Point memory,
        EdOnBN254.EdOnBN254Point memory,
        BN254.G1Point memory,
        uint16
    ) external pure override {
        revert DeprecatedFunction();
    }

    /// @notice Deprecate previous updateConsensusKeys function
    /// @dev This function is overridden to revert with a DeprecatedFunction error
    /// @dev users must call updateConsensusKeysV2 instead
    function updateConsensusKeys(
        BN254.G2Point memory,
        EdOnBN254.EdOnBN254Point memory,
        BN254.G1Point memory
    ) external pure override {
        revert DeprecatedFunction();
    }
}
