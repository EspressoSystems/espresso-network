// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { StakeTableV3 } from "../src/StakeTableV3.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";

// Removes the BLS/Schnorr sig verification for easier fuzzing
contract MockStakeTableV3 is StakeTableV3 {
    function registerValidatorV3(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission,
        string memory metadataUri,
        bytes32 x25519Key,
        string memory p2pAddr
    ) external override whenNotPaused {
        address validator = msg.sender;

        ensureValidatorNotRegistered(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        ensureNewKeys(blsVK, schnorrVK);

        require(commission <= MAX_COMMISSION_BPS, InvalidCommission());
        validateMetadataUri(metadataUri);

        require(x25519Key != bytes32(0), InvalidX25519Key());
        require(!x25519Keys[x25519Key], X25519KeyAlreadyUsed());
        validateP2pAddr(p2pAddr);

        blsKeys[_hashBlsKey(blsVK)] = true;
        schnorrKeys[_hashSchnorrKey(schnorrVK)] = true;
        validators[validator] = Validator({ status: ValidatorStatus.Active, delegatedAmount: 0 });
        commissionTracking[validator] =
            CommissionTracking({ commission: commission, lastIncreaseTime: 0 });
        x25519Keys[x25519Key] = true;

        emit ValidatorRegisteredV3(
            validator,
            blsVK,
            schnorrVK,
            commission,
            blsSig,
            schnorrSig,
            metadataUri,
            x25519Key,
            p2pAddr
        );
    }
}
