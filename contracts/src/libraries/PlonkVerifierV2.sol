// SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { PolynomialEvalV2 as Poly } from "./PolynomialEvalV2.sol";
import { IPlonkVerifier } from "../interfaces/IPlonkVerifier.sol";

/* solhint-disable no-inline-assembly */

/// @dev The TurboPlonk formula is:
/// qo * wo = pub_input + q_c +
///           q_mul0 * w0 * w1 + q_mul1 * w2 * w3 +
///           q_lc0 * w0 + q_lc1 * w1 + q_lc2 * w2 + q_lc3 * w3 +
///           q_hash0 * w0 + q_hash1 * w1 + q_hash2 * w2 + q_hash3 * w3 +
///           q_ecc * w0 * w1 * w2 * w3 * wo
library PlonkVerifierV2 {
    /// Plonk: invalid inputs, either mismatching lengths among input arguments
    /// or empty input.
    error InvalidPlonkArgs();
    /// Plonk: wrong verification key used.
    error WrongPlonkVK();

    // _COSET_K0 = 1, has no effect during multiplication, thus avoid declaring it here.
    uint256 public constant COSET_K1 =
        0x2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a;
    uint256 public constant COSET_K2 =
        0x1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb025;
    uint256 public constant COSET_K3 =
        0x2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a;
    uint256 public constant COSET_K4 =
        0x2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e881;

    // Parsed from Aztec's Ignition CRS,
    // `beta_h` \in G2 where \beta is the trapdoor, h is G2 generator `BN254.P2()`
    // See parsing code: https://github.com/alxiong/crs
    // @dev since library cannot have constant value of custom type, we break it
    // into individual field values.
    uint256 public constant BETA_H_X0 =
        0x260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c1;
    uint256 public constant BETA_H_X1 =
        0x0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b0;
    uint256 public constant BETA_H_Y0 =
        0x04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe4;
    uint256 public constant BETA_H_Y1 =
        0x22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e55;

    /// The number of wire types of the circuit, TurboPlonk has 5.
    uint256 internal constant NUM_WIRE_TYPES = 5;

    /// @dev Plonk IOP verifier challenges.
    struct Challenges {
        uint256 alpha; // 0x00
        uint256 alpha2; // 0x20
        uint256 alpha3; // 0x40
        uint256 beta; // 0x60
        uint256 gamma; // 0x80
        uint256 zeta; // 0xA0
        uint256 v; // 0xC0
        uint256 u; // 0xE0
    }

    /// @dev Verify a single TurboPlonk proofs.
    /// @param verifyingKey The Plonk verification key
    /// @param publicInput The public input fields
    /// @param proof The TurboPlonk proof
    /// @return _ A boolean indicating successful verification, false otherwise
    function verify(
        IPlonkVerifier.VerifyingKey memory verifyingKey,
        uint256[11] memory publicInput,
        IPlonkVerifier.PlonkProof memory proof
    ) external view returns (bool) {
        _validateProof(proof);

        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[0]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[1]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[2]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[3]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[4]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[5]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[6]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[7]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[8]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[9]));
        BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[10]));

        return _verify(verifyingKey, publicInput, proof);
    }

    /// @dev Validate all group points and scalar fields. Revert if
    /// any are invalid.
    /// @param proof A Plonk proof
    function _validateProof(IPlonkVerifier.PlonkProof memory proof) internal pure {
        BN254.validateG1Point(proof.wire0);
        BN254.validateG1Point(proof.wire1);
        BN254.validateG1Point(proof.wire2);
        BN254.validateG1Point(proof.wire3);
        BN254.validateG1Point(proof.wire4);
        BN254.validateG1Point(proof.prodPerm);
        BN254.validateG1Point(proof.split0);
        BN254.validateG1Point(proof.split1);
        BN254.validateG1Point(proof.split2);
        BN254.validateG1Point(proof.split3);
        BN254.validateG1Point(proof.split4);
        BN254.validateG1Point(proof.zeta);
        BN254.validateG1Point(proof.zetaOmega);
        BN254.validateScalarField(proof.wireEval0);
        BN254.validateScalarField(proof.wireEval1);
        BN254.validateScalarField(proof.wireEval2);
        BN254.validateScalarField(proof.wireEval3);
        BN254.validateScalarField(proof.wireEval4);
        BN254.validateScalarField(proof.sigmaEval0);
        BN254.validateScalarField(proof.sigmaEval1);
        BN254.validateScalarField(proof.sigmaEval2);
        BN254.validateScalarField(proof.sigmaEval3);
        BN254.validateScalarField(proof.prodPermZetaOmegaEval);
    }

    // core verifier logic, assuming all input arguments are validated
    function _verify(
        IPlonkVerifier.VerifyingKey memory verifyingKey,
        uint256[11] memory publicInput,
        IPlonkVerifier.PlonkProof memory proof
    ) private view returns (bool) {
        if (verifyingKey.numInputs != 11) revert WrongPlonkVK();

        Challenges memory chal = _computeChallenges(verifyingKey, publicInput, proof);

        Poly.EvalDomain memory domain = Poly.newEvalDomain(verifyingKey.domainSize);
        // pre-compute evaluation data
        Poly.EvalData memory evalData = Poly.evalDataGen(domain, chal.zeta, publicInput);

        // in the final pairing check: e(a, [x]_2) =?= e(b, [1]_2)
        BN254.G1Point memory a;
        BN254.G1Point memory b;

        // a = openingProof + shiftedOpeningProof^u
        // in Plonk paper: "[Wz]1 + u · [Wzω]1"
        a = BN254.add(proof.zeta, BN254.scalarMul(proof.zetaOmega, BN254.ScalarField.wrap(chal.u)));

        // computing b in Plonk paper: "z · [Wz]1 + uzω · [Wzω]1 + [F]1 − [E]1"
        (BN254.G1Point memory e1, BN254.G1Point memory f1) =
            _preparePolyCommitments(verifyingKey, chal, evalData, proof);
        b = BN254.add(f1, BN254.negate(e1)); // [F]1 − [E]1
        // b += proof.zeta^chal.zeta or "z · [Wz]1"
        b = BN254.add(b, BN254.scalarMul(proof.zeta, BN254.ScalarField.wrap(chal.zeta)));

        uint256 p = BN254.R_MOD;
        uint256 scalar;
        assembly {
            // chal.zeta
            scalar := mload(add(chal, 0xa0))
            // chal.zeta * groupGen or nextEvalPoint or zetaOmega
            scalar := mulmod(scalar, mload(add(mload(add(domain, 0x40)), 0x20)), p)
            // u * zetaOmega or "uzω"
            scalar := mulmod(scalar, mload(add(chal, 0xe0)), p)
        }
        // b += proof.zetaOmega^(u * chal.zeta * groupGen)
        b = BN254.add(b, BN254.scalarMul(proof.zetaOmega, BN254.ScalarField.wrap(scalar)));

        BN254.G2Point memory betaH = BN254.G2Point({
            x0: BN254.BaseField.wrap(BETA_H_X1),
            x1: BN254.BaseField.wrap(BETA_H_X0),
            y0: BN254.BaseField.wrap(BETA_H_Y1),
            y1: BN254.BaseField.wrap(BETA_H_Y0)
        });
        // Check e(A, [x]2) =?= e(B, [1]2)
        // Equivalently, e(A, [x]2) * e(-B, [1]2) =?= 1
        return BN254.pairingProd2(a, betaH, BN254.negate(b), BN254.P2());
    }

    function _computeChallenges(
        IPlonkVerifier.VerifyingKey memory vk,
        uint256[11] memory pi,
        IPlonkVerifier.PlonkProof memory proof
    ) internal pure returns (Challenges memory res) {
        uint256 p = BN254.R_MOD;

        assembly {
            // use free memory space for scratch pad, 0x40: free memory ptr
            let statePtr := mload(0x40)
            let dataPtr := add(statePtr, 0x20)

            // Start of transcript (unit: bytes)
            // sizeInBits (4)  | domainSize (8) | numInputs (8) | pad (12)
            mstore(dataPtr, 0) // initialize to 0 first
            mstore(dataPtr, shl(224, 254)) // sizeInBits
            mstore(add(dataPtr, 4), shl(192, mload(vk))) // domainSize
            mstore(add(dataPtr, 12), shl(192, mload(add(vk, 0x20)))) // numInputs

            // G2 from SRS
            mstore(add(dataPtr, 0x20), mload(add(vk, 0x280))) // g2LSB (32)
            mstore(add(dataPtr, 0x40), mload(add(vk, 0x2a0))) // g2MSB (32)

            // k0 ~ k4
            mstore(add(dataPtr, 0x60), 0x1)
            mstore(add(dataPtr, 0x80), COSET_K1)
            mstore(add(dataPtr, 0xa0), COSET_K2)
            mstore(add(dataPtr, 0xc0), COSET_K3)
            mstore(add(dataPtr, 0xe0), COSET_K4)

            // selectors
            let q1Ptr := mload(add(vk, 0xe0))
            mstore(add(dataPtr, 0x100), mload(q1Ptr)) // q1.x
            mstore(add(dataPtr, 0x120), mload(add(q1Ptr, 0x20))) // q1.y
            let q2Ptr := mload(add(vk, 0x100))
            mstore(add(dataPtr, 0x140), mload(q2Ptr)) // q2.x
            mstore(add(dataPtr, 0x160), mload(add(q2Ptr, 0x20))) // q2.y
            let q3Ptr := mload(add(vk, 0x120))
            mstore(add(dataPtr, 0x180), mload(q3Ptr)) // q3.x
            mstore(add(dataPtr, 0x1a0), mload(add(q3Ptr, 0x20))) // q3.y
            let q4Ptr := mload(add(vk, 0x140))
            mstore(add(dataPtr, 0x1c0), mload(q4Ptr)) // q4.x
            mstore(add(dataPtr, 0x1e0), mload(add(q4Ptr, 0x20))) // q4.y
            let qM12Ptr := mload(add(vk, 0x160))
            mstore(add(dataPtr, 0x200), mload(qM12Ptr)) // qM12.x
            mstore(add(dataPtr, 0x220), mload(add(qM12Ptr, 0x20))) // qM12.y
            let qM34Ptr := mload(add(vk, 0x180))
            mstore(add(dataPtr, 0x240), mload(qM34Ptr)) // qM34.x
            mstore(add(dataPtr, 0x260), mload(add(qM34Ptr, 0x20))) // qM34.y
            let qH1Ptr := mload(add(vk, 0x1e0))
            mstore(add(dataPtr, 0x280), mload(qH1Ptr)) // qH1.x
            mstore(add(dataPtr, 0x2a0), mload(add(qH1Ptr, 0x20))) // qH1.y
            let qH2Ptr := mload(add(vk, 0x200))
            mstore(add(dataPtr, 0x2c0), mload(qH2Ptr)) // qH2.x
            mstore(add(dataPtr, 0x2e0), mload(add(qH2Ptr, 0x20))) // qH2.y
            let qH3Ptr := mload(add(vk, 0x220))
            mstore(add(dataPtr, 0x300), mload(qH3Ptr)) // qH3.x
            mstore(add(dataPtr, 0x320), mload(add(qH3Ptr, 0x20))) // qH3.y
            let qH4Ptr := mload(add(vk, 0x240))
            mstore(add(dataPtr, 0x340), mload(qH4Ptr)) // qH4.x
            mstore(add(dataPtr, 0x360), mload(add(qH4Ptr, 0x20))) // qH4.y
            let qOPtr := mload(add(vk, 0x1a0))
            mstore(add(dataPtr, 0x380), mload(qOPtr)) // qO.x
            mstore(add(dataPtr, 0x3a0), mload(add(qOPtr, 0x20))) // qO.y
            let qCPtr := mload(add(vk, 0x1c0))
            mstore(add(dataPtr, 0x3c0), mload(qCPtr)) // qC.x
            mstore(add(dataPtr, 0x3e0), mload(add(qCPtr, 0x20))) // qC.y
            let qECCPtr := mload(add(vk, 0x260))
            mstore(add(dataPtr, 0x400), mload(qECCPtr)) // qECC.x
            mstore(add(dataPtr, 0x420), mload(add(qECCPtr, 0x20))) // qECC.y

            // sigmas
            let sigma0Ptr := mload(add(vk, 0x40))
            mstore(add(dataPtr, 0x440), mload(sigma0Ptr)) // sigma0.x
            mstore(add(dataPtr, 0x460), mload(add(sigma0Ptr, 0x20))) // sigma0.y
            let sigma1Ptr := mload(add(vk, 0x60))
            mstore(add(dataPtr, 0x480), mload(sigma1Ptr)) // sigma1.x
            mstore(add(dataPtr, 0x4a0), mload(add(sigma1Ptr, 0x20))) // sigma1.y
            let sigma2Ptr := mload(add(vk, 0x80))
            mstore(add(dataPtr, 0x4c0), mload(sigma2Ptr)) // sigma2.x
            mstore(add(dataPtr, 0x4e0), mload(add(sigma2Ptr, 0x20))) // sigma2.y
            let sigma3Ptr := mload(add(vk, 0xa0))
            mstore(add(dataPtr, 0x500), mload(sigma3Ptr)) // sigma3.x
            mstore(add(dataPtr, 0x520), mload(add(sigma3Ptr, 0x20))) // sigma3.y
            let sigma4Ptr := mload(add(vk, 0xc0))
            mstore(add(dataPtr, 0x540), mload(sigma4Ptr)) // sigma4.x
            mstore(add(dataPtr, 0x560), mload(add(sigma4Ptr, 0x20))) // sigma4.y

            // public inputs
            mstore(add(dataPtr, 0x580), mload(pi)) // PI[0]
            mstore(add(dataPtr, 0x5a0), mload(add(pi, 0x20))) // PI[1]
            mstore(add(dataPtr, 0x5c0), mload(add(pi, 0x40))) // PI[2]
            mstore(add(dataPtr, 0x5e0), mload(add(pi, 0x60))) // PI[3]
            mstore(add(dataPtr, 0x600), mload(add(pi, 0x80))) // PI[4]
            mstore(add(dataPtr, 0x620), mload(add(pi, 0xa0))) // PI[5]
            mstore(add(dataPtr, 0x640), mload(add(pi, 0xc0))) // PI[6]
            mstore(add(dataPtr, 0x660), mload(add(pi, 0xe0))) // PI[7]
            mstore(add(dataPtr, 0x680), mload(add(pi, 0x100))) // PI[8]
            mstore(add(dataPtr, 0x6a0), mload(add(pi, 0x120))) // PI[9]
            mstore(add(dataPtr, 0x6c0), mload(add(pi, 0x140))) // PI[10]

            // proof
            let wire0Ptr := mload(proof)
            mstore(add(dataPtr, 0x6e0), mload(wire0Ptr)) // wire0.x
            mstore(add(dataPtr, 0x700), mload(add(wire0Ptr, 0x20))) // wire0.y
            let wire1Ptr := mload(add(proof, 0x20))
            mstore(add(dataPtr, 0x720), mload(wire1Ptr)) // wire1.x
            mstore(add(dataPtr, 0x740), mload(add(wire1Ptr, 0x20))) // wire1.y
            let wire2Ptr := mload(add(proof, 0x40))
            mstore(add(dataPtr, 0x760), mload(wire2Ptr)) // wire2.x
            mstore(add(dataPtr, 0x780), mload(add(wire2Ptr, 0x20))) // wire2.y
            let wire3Ptr := mload(add(proof, 0x60))
            mstore(add(dataPtr, 0x7a0), mload(wire3Ptr)) // wire3.x
            mstore(add(dataPtr, 0x7c0), mload(add(wire3Ptr, 0x20))) // wire3.y
            let wire4Ptr := mload(add(proof, 0x80))
            mstore(add(dataPtr, 0x7e0), mload(wire4Ptr)) // wire4.x
            mstore(add(dataPtr, 0x800), mload(add(wire4Ptr, 0x20))) // wire4.y

            // challenge: beta
            {
                mstore(statePtr, 0x0) // init state
                // preimage len: state(0x20) + transcript(0x820)
                // overwrite previous state at freePtr
                mstore(statePtr, keccak256(statePtr, 0x840))
                // (mod p) to get beta
                mstore(add(res, 0x60), mod(mload(statePtr), p))
            }

            // challenge: gamma
            {
                // preimage len: state(0x20) + transcript(0x0)
                // overwrite previous state at freePtr
                mstore(statePtr, keccak256(statePtr, 0x20))
                // (mod p) to get gamma
                mstore(add(res, 0x80), mod(mload(statePtr), p))
            }

            let prodPermPtr := mload(add(proof, 0xa0))
            mstore(dataPtr, mload(prodPermPtr)) // prodPerm.x
            mstore(add(dataPtr, 0x20), mload(add(prodPermPtr, 0x20))) // prodPerm.y

            // challenge: alpha, alpha2, alpha3
            {
                // preimage len: state(0x20) + transcript(0x40)
                let alpha := keccak256(statePtr, 0x60)
                mstore(statePtr, alpha)
                // (mod p) to get challenge
                mstore(res, mod(alpha, p))

                let alpha2 := mulmod(alpha, alpha, p)
                let alpha3 := mulmod(alpha2, alpha, p)
                mstore(add(res, 0x20), alpha2)
                mstore(add(res, 0x40), alpha3)
            }

            let split0Ptr := mload(add(proof, 0xc0))
            mstore(dataPtr, mload(split0Ptr)) // split0.x
            mstore(add(dataPtr, 0x20), mload(add(split0Ptr, 0x20))) // split0.y
            let split1Ptr := mload(add(proof, 0xe0))
            mstore(add(dataPtr, 0x40), mload(split1Ptr)) // split1.x
            mstore(add(dataPtr, 0x60), mload(add(split1Ptr, 0x20))) // split1.y
            let split2Ptr := mload(add(proof, 0x100))
            mstore(add(dataPtr, 0x80), mload(split2Ptr)) // split2.x
            mstore(add(dataPtr, 0xa0), mload(add(split2Ptr, 0x20))) // split2.y
            let split3Ptr := mload(add(proof, 0x120))
            mstore(add(dataPtr, 0xc0), mload(split3Ptr)) // split3.x
            mstore(add(dataPtr, 0xe0), mload(add(split3Ptr, 0x20))) // split3.y
            let split4Ptr := mload(add(proof, 0x140))
            mstore(add(dataPtr, 0x100), mload(split4Ptr)) // split4.x
            mstore(add(dataPtr, 0x120), mload(add(split4Ptr, 0x20))) // split4.y

            // challenge: zeta
            {
                // preimage len: state(0x20) + transcript(0x140)
                // overwrite previous state at freePtr
                mstore(statePtr, keccak256(statePtr, 0x160))
                // (mod p) to get challenge
                mstore(add(res, 0xa0), mod(mload(statePtr), p))
            }

            mstore(dataPtr, mload(add(proof, 0x1a0))) // wireEval0
            mstore(add(dataPtr, 0x20), mload(add(proof, 0x1c0))) // wireEval1
            mstore(add(dataPtr, 0x40), mload(add(proof, 0x1e0))) // wireEval2
            mstore(add(dataPtr, 0x60), mload(add(proof, 0x200))) // wireEval3
            mstore(add(dataPtr, 0x80), mload(add(proof, 0x220))) // wireEval4
            mstore(add(dataPtr, 0xa0), mload(add(proof, 0x240))) // sigmaEval0
            mstore(add(dataPtr, 0xc0), mload(add(proof, 0x260))) // sigmaEval1
            mstore(add(dataPtr, 0xe0), mload(add(proof, 0x280))) // sigmaEval2
            mstore(add(dataPtr, 0x100), mload(add(proof, 0x2a0))) // sigmaEval3
            mstore(add(dataPtr, 0x120), mload(add(proof, 0x2c0))) // prodPermZetaOmegaEval

            // challenge: v
            {
                // preimage len: state(0x20) + transcript(0x140)
                // overwrite previous state at freePtr
                mstore(statePtr, keccak256(statePtr, 0x160))
                // (mod p) to get challenge
                mstore(add(res, 0xc0), mod(mload(statePtr), p))
            }

            let zetaPtr := mload(add(proof, 0x160))
            mstore(dataPtr, mload(zetaPtr)) // zeta.x
            mstore(add(dataPtr, 0x20), mload(add(zetaPtr, 0x20))) // zeta.y
            let zetaOmegaPtr := mload(add(proof, 0x180))
            mstore(add(dataPtr, 0x40), mload(zetaOmegaPtr)) // zetaOmega.x
            mstore(add(dataPtr, 0x60), mload(add(zetaOmegaPtr, 0x20))) // zetaOmega.y

            // challenge: u
            {
                // preimage len: state(0x20) + transcript(0x80)
                let hash := keccak256(statePtr, 0xa0)
                // (mod p) to get challenge
                mstore(add(res, 0xe0), mod(hash, p))
            }
        }
    }

    /// @dev Compute the constant term of the linearization polynomial.
    /// ```
    /// r_plonk = PI - L1(x) * alpha^2 - alpha * \prod_i=1..m-1 (w_i + beta * sigma_i + gamma) *
    /// (w_m + gamma) * z(xw)
    /// ```
    /// where m is the number of wire types.
    function _computeLinPolyConstantTerm(
        Challenges memory chal,
        IPlonkVerifier.PlonkProof memory proof,
        Poly.EvalData memory evalData
    ) internal pure returns (uint256 res) {
        uint256 p = BN254.R_MOD;
        uint256 lagrangeOneEval = BN254.ScalarField.unwrap(evalData.lagrangeOne);
        uint256 piEval = BN254.ScalarField.unwrap(evalData.piEval);
        uint256 perm = 1;

        assembly {
            let beta := mload(add(chal, 0x60))
            let gamma := mload(add(chal, 0x80))

            // \prod_i=1..m-1 (w_i + beta * sigma_i + gamma)
            {
                let w0 := mload(add(proof, 0x1a0))
                let sigma0 := mload(add(proof, 0x240))
                perm := mulmod(perm, addmod(addmod(w0, gamma, p), mulmod(beta, sigma0, p), p), p)
            }
            {
                let w1 := mload(add(proof, 0x1c0))
                let sigma1 := mload(add(proof, 0x260))
                perm := mulmod(perm, addmod(addmod(w1, gamma, p), mulmod(beta, sigma1, p), p), p)
            }
            {
                let w2 := mload(add(proof, 0x1e0))
                let sigma2 := mload(add(proof, 0x280))
                perm := mulmod(perm, addmod(addmod(w2, gamma, p), mulmod(beta, sigma2, p), p), p)
            }
            {
                let w3 := mload(add(proof, 0x200))
                let sigma3 := mload(add(proof, 0x2a0))
                perm := mulmod(perm, addmod(addmod(w3, gamma, p), mulmod(beta, sigma3, p), p), p)
            }

            // \prod_i=1..m-1 (w_i + beta * sigma_i + gamma) * (w_m + gamma) * z(xw)
            {
                let w4 := mload(add(proof, 0x220))
                let permNextEval := mload(add(proof, 0x2c0))
                perm := mulmod(perm, mulmod(addmod(w4, gamma, p), permNextEval, p), p)
            }

            let alpha := mload(chal)
            let alpha2 := mload(add(chal, 0x20))
            // PI - L1(x) * alpha^2 - alpha * \prod_i=1..m-1 (w_i + beta * sigma_i + gamma) * (w_m +
            // gamma) * z(xw)
            res := addmod(piEval, sub(p, mulmod(alpha2, lagrangeOneEval, p)), p)
            res := addmod(res, sub(p, mulmod(alpha, perm, p)), p)
        }
    }

    /// @return e1 The [E]1 in Sec 8.4, step 11 of Plonk
    /// @return f1 The [F]1 in Sec 8.4, step 10 of Plonk
    function _preparePolyCommitments(
        IPlonkVerifier.VerifyingKey memory verifyingKey,
        Challenges memory chal,
        Poly.EvalData memory evalData,
        IPlonkVerifier.PlonkProof memory proof
    ) internal view returns (BN254.G1Point memory e1, BN254.G1Point memory f1) {
        // Compute first part of batched polynomial commitment [D]1
        BN254.G1Point memory d1 = _linearizationPolyComm(verifyingKey, chal, evalData, proof);

        // ============================================
        // Add wire witness poly commitments
        // Meanwhile, _prepareEvaluation (`[E]1` in Sec 8.4, step 11 of Plonk)
        // ============================================
        uint256 p = BN254.R_MOD;

        // r0 in step 8 of Plonk
        uint256 eval = _computeLinPolyConstantTerm(chal, proof, evalData);
        eval = p - eval; // -r0

        uint256 v = chal.v;
        uint256 vPow = v;

        assembly {
            // eval += v * proof.wireEval0
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x1a0)), p), p)
        }
        f1 = BN254.add(d1, BN254.scalarMul(proof.wire0, BN254.ScalarField.wrap(vPow)));

        assembly {
            // vPow = v^2
            vPow := mulmod(vPow, v, p)
            // eval += v^2 * proof.wireEval1
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x1c0)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(proof.wire1, BN254.ScalarField.wrap(vPow)));

        assembly {
            // vPow = v^3
            vPow := mulmod(vPow, v, p)
            // eval += v^3 * proof.wireEval2
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x1e0)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(proof.wire2, BN254.ScalarField.wrap(vPow)));

        assembly {
            // vPow = v^4
            vPow := mulmod(vPow, v, p)
            // eval += v^4 * proof.wireEval3
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x200)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(proof.wire3, BN254.ScalarField.wrap(vPow)));

        assembly {
            // vPow = v^5
            vPow := mulmod(vPow, v, p)
            // eval += v^5 * proof.wireEval4
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x220)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(proof.wire4, BN254.ScalarField.wrap(vPow)));

        // Add wire sigma polynomial commitments. The last sigma commitment is excluded.
        assembly {
            // vPow = v^6
            vPow := mulmod(vPow, v, p)
            // eval += v^6 * proof.sigmaEval0
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x240)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(verifyingKey.sigma0, BN254.ScalarField.wrap(vPow)));

        assembly {
            // vPow = v^7
            vPow := mulmod(vPow, v, p)
            // eval += v^7 * proof.sigmaEval1
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x260)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(verifyingKey.sigma1, BN254.ScalarField.wrap(vPow)));

        assembly {
            // vPow = v^8
            vPow := mulmod(vPow, v, p)
            // eval += v^8 * proof.sigmaEval2
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x280)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(verifyingKey.sigma2, BN254.ScalarField.wrap(vPow)));

        assembly {
            // vPow = v^9
            vPow := mulmod(vPow, v, p)
            // eval += v^9 * proof.sigmaEval3
            eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x2a0)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(verifyingKey.sigma3, BN254.ScalarField.wrap(vPow)));

        // Add poly commitments to be evaluated at point `zeta * g`.
        uint256 u = chal.u;
        assembly {
            // eval += u * proof.prodPermZetaOmegaEval
            eval := addmod(eval, mulmod(u, mload(add(proof, 0x2c0)), p), p)
        }
        f1 = BN254.add(f1, BN254.scalarMul(proof.prodPerm, BN254.ScalarField.wrap(u)));

        // e1 = eval * [1]1
        e1 = BN254.scalarMul(BN254.P1(), BN254.ScalarField.wrap(eval));
    }

    /// @dev Compute the linearization poly commitment
    /// @param verifyingKey The verifying key
    /// @param challenge A set of challenges
    /// @param evalData Polynomial evaluation data
    /// @param proof A Plonk proof
    /// @return d1 The [D]1 in Step 9 of Plonk
    function _linearizationPolyComm(
        IPlonkVerifier.VerifyingKey memory verifyingKey,
        Challenges memory challenge,
        Poly.EvalData memory evalData,
        IPlonkVerifier.PlonkProof memory proof
    ) private view returns (BN254.G1Point memory d1) {
        uint256 tmpScalar;
        uint256 rhs;
        uint256 tmp;
        uint256 tmp2;
        uint256 p = BN254.R_MOD;

        // ============================================
        // Compute coefficient for the permutation product polynomial commitment.
        // firstScalar =
        //          L1(zeta) * alpha^2
        //          + alpha
        //              * (beta * zeta      + wireEval0 + gamma)
        //              * (beta * k1 * zeta + wireEval1 + gamma)
        //              * (beta * k2 * zeta + wireEval2 + gamma)
        //              * ...
        // where wireEval0, wireEval1, wireEval2, ... are in w_evals
        // ============================================
        // first base and scala:
        // - proof.prodPerm
        // - firstScalar
        assembly {
            let gamma := mload(add(challenge, 0x80))
            // firstScalar = alpha^2 * L1(zeta)
            tmpScalar := mulmod(mload(add(challenge, 0x20)), mload(add(evalData, 0x20)), p)

            // rhs = alpha
            rhs := mload(challenge)

            // tmp = beta * zeta
            tmp := mulmod(mload(add(challenge, 0x60)), mload(add(challenge, 0xA0)), p)

            // =================================
            // k0 (which is 1) component
            // (beta * zeta + wireEval0 + gamma)
            // =================================
            tmp2 := addmod(tmp, mload(add(proof, 0x1A0)), p)
            tmp2 := addmod(tmp2, gamma, p)
            rhs := mulmod(tmp2, rhs, p)

            // =================================
            // k1 component
            // (beta * zeta * k1 + wireEval1 + gamma)
            // =================================
            tmp2 := mulmod(tmp, COSET_K1, p)
            tmp2 := addmod(tmp2, mload(add(proof, 0x1C0)), p)
            tmp2 := addmod(tmp2, gamma, p)
            rhs := mulmod(tmp2, rhs, p)

            // =================================
            // k2 component
            // (beta * zeta * k2 + wireEval2 + gamma)
            // =================================
            tmp2 := mulmod(tmp, COSET_K2, p)
            tmp2 := addmod(tmp2, mload(add(proof, 0x1E0)), p)
            tmp2 := addmod(tmp2, gamma, p)
            rhs := mulmod(tmp2, rhs, p)

            // =================================
            // k3 component
            // (beta * zeta * k3 + wireEval3 + gamma)
            // =================================
            tmp2 := mulmod(tmp, COSET_K3, p)
            tmp2 := addmod(tmp2, mload(add(proof, 0x200)), p)
            tmp2 := addmod(tmp2, gamma, p)
            rhs := mulmod(tmp2, rhs, p)

            // =================================
            // k4 component
            // (beta * zeta * k4 + wireEval4 + gamma)
            // =================================
            tmp2 := mulmod(tmp, COSET_K4, p)
            tmp2 := addmod(tmp2, mload(add(proof, 0x220)), p)
            tmp2 := addmod(tmp2, gamma, p)
            rhs := mulmod(tmp2, rhs, p)

            tmpScalar := addmod(tmpScalar, rhs, p)
        }
        d1 = BN254.scalarMul(proof.prodPerm, BN254.ScalarField.wrap(tmpScalar));

        // ============================================
        // Compute coefficient for the last wire sigma polynomial commitment.
        // secondScalar = - alpha * beta * z_w
        //              * (wireEval0 + gamma + beta * sigmaEval0)
        //              * (wireEval1 + gamma + beta * sigmaEval1)
        //              * ...
        // ============================================
        // second base and scala:
        // - verifyingKey.sigma4
        // - secondScalar
        assembly {
            let alpha := mload(challenge)
            let beta := mload(add(challenge, 0x60))
            let gamma := mload(add(challenge, 0x80))
            // secondScalar = alpha * beta * z_w
            tmpScalar := mulmod(alpha, beta, p)
            tmpScalar := mulmod(tmpScalar, mload(add(proof, 0x2C0)), p)

            // (wireEval0 + gamma + beta * sigmaEval0)
            tmp := mulmod(beta, mload(add(proof, 0x240)), p)
            tmp := addmod(tmp, mload(add(proof, 0x1A0)), p)
            tmp := addmod(tmp, gamma, p)

            tmpScalar := mulmod(tmpScalar, tmp, p)

            // (wireEval1 + gamma + beta * sigmaEval1)
            tmp := mulmod(beta, mload(add(proof, 0x260)), p)
            tmp := addmod(tmp, mload(add(proof, 0x1C0)), p)
            tmp := addmod(tmp, gamma, p)

            tmpScalar := mulmod(tmpScalar, tmp, p)

            // (wireEval2 + gamma + beta * sigmaEval2)
            tmp := mulmod(beta, mload(add(proof, 0x280)), p)
            tmp := addmod(tmp, mload(add(proof, 0x1E0)), p)
            tmp := addmod(tmp, gamma, p)

            tmpScalar := mulmod(tmpScalar, tmp, p)

            // (wireEval3 + gamma + beta * sigmaEval3)
            tmp := mulmod(beta, mload(add(proof, 0x2A0)), p)
            tmp := addmod(tmp, mload(add(proof, 0x200)), p)
            tmp := addmod(tmp, gamma, p)

            tmpScalar := mulmod(tmpScalar, tmp, p)
        }
        d1 = BN254.add(
            d1, BN254.scalarMul(verifyingKey.sigma4, BN254.ScalarField.wrap(p - tmpScalar))
        );

        // ============================================
        // next 13 are for selectors:
        //
        // the selectors are organized as
        // - q_lc
        // - q_mul
        // - q_hash
        // - q_o
        // - q_c
        // - q_ecc
        // ============================================

        // ============
        // q_lc
        // ============
        // q_1...q_4
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.q1, proof.wireEval0));
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.q2, proof.wireEval1));
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.q3, proof.wireEval2));
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.q4, proof.wireEval3));

        // ============
        // q_M
        // ============
        // q_M12 and q_M34
        // q_M12 = w_evals[0] * w_evals[1];
        assembly {
            tmp := mulmod(mload(add(proof, 0x1A0)), mload(add(proof, 0x1C0)), p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qM12, BN254.ScalarField.wrap(tmp)));

        assembly {
            tmp := mulmod(mload(add(proof, 0x1E0)), mload(add(proof, 0x200)), p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qM34, BN254.ScalarField.wrap(tmp)));

        // ============
        // q_H
        // ============
        // w_evals[0].pow([5]);
        assembly {
            tmp := mload(add(proof, 0x1A0))
            tmp2 := mulmod(tmp, tmp, p)
            tmp2 := mulmod(tmp2, tmp2, p)
            tmp := mulmod(tmp, tmp2, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qH1, BN254.ScalarField.wrap(tmp)));

        // w_evals[1].pow([5]);
        assembly {
            tmp := mload(add(proof, 0x1C0))
            tmp2 := mulmod(tmp, tmp, p)
            tmp2 := mulmod(tmp2, tmp2, p)
            tmp := mulmod(tmp, tmp2, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qH2, BN254.ScalarField.wrap(tmp)));

        // w_evals[2].pow([5]);
        assembly {
            tmp := mload(add(proof, 0x1E0))
            tmp2 := mulmod(tmp, tmp, p)
            tmp2 := mulmod(tmp2, tmp2, p)
            tmp := mulmod(tmp, tmp2, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qH3, BN254.ScalarField.wrap(tmp)));

        // w_evals[3].pow([5]);
        assembly {
            tmp := mload(add(proof, 0x200))
            tmp2 := mulmod(tmp, tmp, p)
            tmp2 := mulmod(tmp2, tmp2, p)
            tmp := mulmod(tmp, tmp2, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qH4, BN254.ScalarField.wrap(tmp)));

        // ============
        // q_o and q_c
        // ============
        // q_o
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qO, BN254.negate(proof.wireEval4)));
        // q_c
        d1 = BN254.add(d1, verifyingKey.qC);

        // ============
        // q_Ecc
        // ============
        // q_Ecc = w_evals[0] * w_evals[1] * w_evals[2] * w_evals[3] * w_evals[4];
        assembly {
            tmp := mulmod(mload(add(proof, 0x1A0)), mload(add(proof, 0x1C0)), p)
            tmp := mulmod(tmp, mload(add(proof, 0x1E0)), p)
            tmp := mulmod(tmp, mload(add(proof, 0x200)), p)
            tmp := mulmod(tmp, mload(add(proof, 0x220)), p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(verifyingKey.qEcc, BN254.ScalarField.wrap(tmp)));

        // ============================================
        // the last 5 are for splitting quotient commitments
        // ============================================

        // first one is 1-zeta^n
        tmpScalar = BN254.ScalarField.unwrap(BN254.negate(evalData.vanishEval)); // reused next
        d1 = BN254.add(d1, BN254.scalarMul(proof.split0, BN254.ScalarField.wrap(tmpScalar)));

        // second one is (1-zeta^n) zeta^(n+2)
        assembly {
            // zeta^n
            tmp := addmod(mload(evalData), 1, p)
            // todo: use pre-computed zeta^2
            tmp2 := mulmod(mload(add(challenge, 0xA0)), mload(add(challenge, 0xA0)), p)
            // tmp = zeta^{n+2}
            tmp := mulmod(tmp, tmp2, p)
            tmpScalar := mulmod(tmpScalar, tmp, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(proof.split1, BN254.ScalarField.wrap(tmpScalar)));

        // third one is (1-zeta^n) zeta^2(n+2)
        assembly {
            tmpScalar := mulmod(tmpScalar, tmp, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(proof.split2, BN254.ScalarField.wrap(tmpScalar)));

        // forth one is (1-zeta^n) zeta^3(n+2)
        assembly {
            tmpScalar := mulmod(tmpScalar, tmp, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(proof.split3, BN254.ScalarField.wrap(tmpScalar)));

        // fifth one is (1-zeta^n) zeta^4(n+2)
        assembly {
            tmpScalar := mulmod(tmpScalar, tmp, p)
        }
        d1 = BN254.add(d1, BN254.scalarMul(proof.split4, BN254.ScalarField.wrap(tmpScalar)));
    }
}
