// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";

// Removes the BLS sig verification for easier fuzzing
contract MockStakeTableV2 is StakeTableV2 {
    function registerValidatorV2(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission
    ) external override {
        address validator = msg.sender;

        ensureValidatorNotRegistered(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        ensureNewKey(blsVK);

        if (commission > 10000) {
            revert InvalidCommission();
        }

        blsKeys[_hashBlsKey(blsVK)] = true;
        validators[validator] = Validator({ status: ValidatorStatus.Active, delegatedAmount: 0 });

        // Store the initial commission for this validator
        commissionTracking[validator] =
            CommissionTracking({ commission: commission, lastIncreaseTime: block.timestamp });

        emit ValidatorRegisteredV2(validator, blsVK, schnorrVK, commission, blsSig, schnorrSig);
    }
}
