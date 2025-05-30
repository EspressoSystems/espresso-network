// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { PausableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/utils/PausableUpgradeable.sol";
import { AccessControlUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";
import { StakeTableV2 } from "./StakeTableV2.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";

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
contract StakeTableV3 is StakeTableV2, PausableUpgradeable, AccessControlUpgradeable {
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");

    constructor() {
        _disableInitializers();
    }

    function initializeV3(address pauser, address admin) public reinitializer(3) {
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
        return (3, 0, 0);
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

    function updateConsensusKeysV2(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig
    ) public virtual override whenNotPaused {
        super.updateConsensusKeysV2(blsVK, schnorrVK, blsSig, schnorrSig);
    }
}
