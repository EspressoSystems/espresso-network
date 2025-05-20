// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { StakeTable } from "./StakeTable.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";

contract StakeTableV2 is StakeTable {
    // === Events ===

    /// @notice A validator is registered in the stake table
    /// @notice the blsSig and schnorrSig are validated by the Espresso Network
    event ValidatorRegisteredV2(
        address indexed account,
        BN254.G2Point blsVk,
        EdOnBN254.EdOnBN254Point schnorrVk,
        uint16 commission,
        BN254.G1Point blsSig,
        EdOnBN254.EdOnBN254Point schnorrSig
    );

    /// @notice A validator updates their consensus keys
    /// @notice the blsSig and schnorrSig are validated by the Espresso Network
    event ConsensusKeysUpdatedV2(
        address indexed account,
        BN254.G2Point blsVK,
        EdOnBN254.EdOnBN254Point schnorrVK,
        BN254.G1Point blsSig,
        EdOnBN254.EdOnBN254Point schnorrSig
    );

    /// @notice The exit escrow period is updated
    event ExitEscrowPeriodUpdated(uint64 newExitEscrowPeriod);

    // === Errors ===

    /// The exit escrow period is invalid (either too short or too long)
    error ExitEscrowPeriodInvalid();

    constructor() {
        _disableInitializers();
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

    /// @notice Register a validator in the stake table
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key
    /// @param blsSig The BLS signature that authenticates the BLS VK
    /// @param schnorrSig The Schnorr signature that authenticates the Schnorr VK
    /// @param commission in % with 2 decimals, from 0.00% (value 0) to 100% (value 10_000)
    function registerValidator(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        EdOnBN254.EdOnBN254Point memory schnorrSig,
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
    function updateConsensusKeys(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        EdOnBN254.EdOnBN254Point memory schnorrSig
    ) external virtual {
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
}
