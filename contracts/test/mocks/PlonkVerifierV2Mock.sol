// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { IPlonkVerifier } from "../../src/interfaces/IPlonkVerifier.sol";
import { PlonkVerifierV2 } from "../../src/libraries/PlonkVerifierV2.sol";

contract PlonkVerifierV2Mock is PlonkVerifierV2 {
    function validateProof(IPlonkVerifier.PlonkProof memory proof) public pure {
        _validateProof(proof);
    }

    function computeChallenges(
        IPlonkVerifier.VerifyingKey memory vk,
        uint256[11] memory pi,
        IPlonkVerifier.PlonkProof memory proof
    ) public pure returns (Challenges memory) {
        return _computeChallenges(vk, pi, proof);
    }
}
