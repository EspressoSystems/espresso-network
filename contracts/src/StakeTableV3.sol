// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { StakeTableV2 } from "./StakeTableV2.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";

/// @title StakeTableV3
/// @notice V3 hardens Schnorr key validation by rejecting the Edwards identity point `(0, 1)`.
contract StakeTableV3 is StakeTableV2 {
    /// @notice Constructor
    /// @dev This function is overridden to disable initializers
    constructor() {
        _disableInitializers();
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
        return (3, 0, 0);
    }

    /// @notice Register a validator in the stake table
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key
    /// @param blsSig The BLS signature that authenticates the BLS VK
    /// @param schnorrSig The Schnorr signature that authenticates the Schnorr VK
    /// @param commission in % with 2 decimals, from 0.00% (value 0) to 100% (value 10_000)
    /// @param metadataUri The metadata URI for the validator
    function registerValidatorV2(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission,
        string memory metadataUri
    ) external virtual override whenNotPaused {
        address validator = msg.sender;

        ensureValidatorNotRegistered(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        _ensureSchnorrKeyNotIdentity(schnorrVK);
        ensureNewKeys(blsVK, schnorrVK);

        bytes memory message = abi.encode(validator);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        if (schnorrSig.length != 64) {
            revert InvalidSchnorrSig();
        }

        if (commission > MAX_COMMISSION_BPS) {
            revert InvalidCommission();
        }

        validateMetadataUri(metadataUri);

        blsKeys[_hashBlsKey(blsVK)] = true;
        schnorrKeys[_hashSchnorrKey(schnorrVK)] = true;
        validators[validator] = Validator({ status: ValidatorStatus.Active, delegatedAmount: 0 });

        commissionTracking[validator] =
            CommissionTracking({ commission: commission, lastIncreaseTime: 0 });

        emit ValidatorRegisteredV2(
            validator, blsVK, schnorrVK, commission, blsSig, schnorrSig, metadataUri
        );
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
    ) public virtual override whenNotPaused {
        _ensureSchnorrKeyNotIdentity(schnorrVK);
        super.updateConsensusKeysV2(blsVK, schnorrVK, blsSig, schnorrSig);
    }

    /// @dev (0, 1) is the identity (neutral element) on the twisted Edwards curve over BN254.
    /// @dev Schnorr signatures are trivially forgeable when the public key is the identity point so
    /// we reject it.
    function _ensureSchnorrKeyNotIdentity(EdOnBN254.EdOnBN254Point memory schnorrVK) internal pure {
        if (schnorrVK.x == 0 && schnorrVK.y == 1) {
            revert InvalidSchnorrVK();
        }
    }
}
