// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { PausableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import { AccessControlUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import { StakeTable } from "./StakeTable.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";
import { SafeTransferLib, ERC20 } from "solmate/utils/SafeTransferLib.sol";

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
/// 2. The exit escrow period can be updated by the owner of the contract.
///
/// @notice The StakeTableV2 contract ABI is a superset of the original ABI. Consumers of the
/// contract can use the V2 ABI, even if they would like to maintain backwards compatibility.
contract StakeTableV2 is StakeTable, PausableUpgradeable, AccessControlUpgradeable {
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

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

    // === Errors ===

    /// The exit escrow period is invalid (either too short or too long)
    error ExitEscrowPeriodInvalid();

    /// The Schnorr signature is invalid (either the wrong length or the wrong key)
    error InvalidSchnorrSig();

    /// The function is deprecated as it was replaced by a new function
    error DeprecatedFunction();

    /// @notice Constructor
    /// @dev This function is overridden to disable initializers
    constructor() {
        _disableInitializers();
    }

    /// @notice Reinitialize the contract
    /// @dev This function is overridden to add pauser and admin roles
    function initializeV2(address pauser, address admin) public reinitializer(2) {
        __AccessControl_init();

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(PAUSER_ROLE, pauser);
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
        // deduct the amount from the validator's delegatedAmount
        validators[validator].delegatedAmount -= amount;

        SafeTransferLib.safeTransfer(token, delegator, amount);

        emit Withdrawal(delegator, amount);
    }

    /// @notice Withdraw previously delegated funds after a validator has exited
    /// @param validator The validator to withdraw from
    /// @dev This function is overridden to deduct the amount from the validator's delegatedAmount
    /// and to add pausable functionality
    /// @dev since the delegated Amount is no longer updated during undelegation
    /// @dev delegatedAmount represents the no. of tokens that have been delegated to a validator,
    /// even if it's not participating in consensus
    function claimWithdrawal(address validator) public virtual override whenNotPaused {
        address delegator = msg.sender;
        // If entries are missing at any of the levels of the mapping this will return zero
        uint256 amount = undelegations[validator][delegator].amount;
        if (amount == 0) {
            revert NothingToWithdraw();
        }

        if (block.timestamp < undelegations[validator][delegator].unlocksAt) {
            revert PrematureWithdrawal();
        }

        // Mark funds as spent
        delete undelegations[validator][delegator];
        // deduct the amount from the validator's delegatedAmount
        validators[validator].delegatedAmount -= amount;

        SafeTransferLib.safeTransfer(token, delegator, amount);

        emit Withdrawal(delegator, amount);
    }

    /// @notice Delegate funds to a validator
    /// @param validator The validator to delegate to
    /// @param amount The amount to delegate
    /// @dev This function is overridden to add pausable functionality
    function delegate(address validator, uint256 amount) public virtual override whenNotPaused {
        super.delegate(validator, amount);
    }

    /// @notice Undelegate funds from a validator
    /// @param validator The validator to undelegate from
    /// @param amount The amount to undelegate
    /// @dev This function is overridden to add pausable functionality
    /// @dev and to ensure that the validator's delegatedAmount is not updated until withdrawal
    /// @dev delegatedAmount represents the no. of tokens that have been delegated to a validator,
    /// even if it's not participating in consensus
    function undelegate(address validator, uint256 amount) public virtual override whenNotPaused {
        ensureValidatorActive(validator);
        address delegator = msg.sender;

        if (amount == 0) {
            revert ZeroAmount();
        }

        if (undelegations[validator][delegator].amount != 0) {
            revert UndelegationAlreadyExists();
        }

        uint256 balance = delegations[validator][delegator];
        if (balance < amount) {
            revert InsufficientBalance(balance);
        }

        delegations[validator][delegator] -= amount;
        undelegations[validator][delegator] =
            Undelegation({ amount: amount, unlocksAt: block.timestamp + exitEscrowPeriod });

        emit Undelegated(delegator, validator, amount);
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
        ensureNewKey(blsVK);

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
        validators[validator] = Validator({ status: ValidatorStatus.Active, delegatedAmount: 0 });

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
        ensureNewKey(blsVK);

        // Verify that the validator can sign for that blsVK. This prevents rogue public-key
        // attacks.
        bytes memory message = abi.encode(validator);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        blsKeys[_hashBlsKey(blsVK)] = true;

        emit ConsensusKeysUpdatedV2(validator, blsVK, schnorrVK, blsSig, schnorrSig);
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
