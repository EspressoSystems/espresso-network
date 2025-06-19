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

    /// The function is deprecated as it was replaced by a new function
    error DeprecatedFunction();

    constructor() {
        _disableInitializers();
    }

    function initializeV2(address pauser, address admin) public reinitializer(2) {
        __AccessControl_init();

        _grantRole(DEFAULT_ADMIN_ROLE, admin);
        _grantRole(PAUSER_ROLE, pauser);
    }

    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (2, 0, 0);
    }

    function pause() external onlyRole(PAUSER_ROLE) {
        _pause();
    }

    function unpause() external onlyRole(PAUSER_ROLE) {
        _unpause();
    }

    function claimValidatorExit(address validator) public virtual override whenNotPaused {
        super.claimValidatorExit(validator);
    }

    function claimWithdrawal(address validator) public virtual override whenNotPaused {
        super.claimWithdrawal(validator);
    }

    function delegate(address validator, uint256 amount) public virtual override whenNotPaused {
        super.delegate(validator, amount);
    }

    function undelegate(address validator, uint256 amount) public virtual override whenNotPaused {
        super.undelegate(validator, amount);
    }

    function deregisterValidator() public virtual override whenNotPaused {
        super.deregisterValidator();
    }

    /// @notice Register a validator in the stake table
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key
    /// @param blsSig The BLS signature that authenticates the BLS VK
    /// @param schnorrSig The Schnorr signature that authenticates the Schnorr VK
    /// @param commission in % with 2 decimals, from 0.00% (value 0) to 100% (value 10_000)
    function registerValidatorV2(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission
    ) external virtual {
        address validator = msg.sender;

        ensureValidatorNotRegistered(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        ensureNewKey(blsVK);

        // Verify that the validator can sign for that blsVK. This prevents rogue public-key
        // attacks.
        bytes memory message = abi.encode(validator);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

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

    // deprecate previous registration function
    function registerValidator(
        BN254.G2Point memory,
        EdOnBN254.EdOnBN254Point memory,
        BN254.G1Point memory,
        uint16
    ) external pure override {
        revert DeprecatedFunction();
    }

    // deprecate previous updateConsensusKeys function
    function updateConsensusKeys(
        BN254.G2Point memory,
        EdOnBN254.EdOnBN254Point memory,
        BN254.G1Point memory
    ) external pure override {
        revert DeprecatedFunction();
    }
}
