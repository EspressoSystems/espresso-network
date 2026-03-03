// SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { PolynomialEvalV3 as Poly } from "./PolynomialEvalV3.sol";
import { IPlonkVerifier } from "../interfaces/IPlonkVerifier.sol";

/* solhint-disable no-inline-assembly */

/// @dev The TurboPlonk formula is:
/// qo * wo = pub_input + q_c +
///           q_mul0 * w0 * w1 + q_mul1 * w2 * w3 +
///           q_lc0 * w0 + q_lc1 * w1 + q_lc2 * w2 + q_lc3 * w3 +
///           q_hash0 * w0 + q_hash1 * w1 + q_hash2 * w2 + q_hash3 * w3 +
///           q_ecc * w0 * w1 * w2 * w3 * wo
library PlonkVerifierV4 {
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
        uint256 beta; // 0x40
        uint256 gamma; // 0x60
        uint256 zeta; // 0x80
        uint256 v; // 0xA0
        uint256 u; // 0xC0
    }

    /// @dev Verify a single TurboPlonk proofs.
    /// @param verifyingKey The Plonk verification key
    /// @param publicInput The public input fields
    /// @param proof The TurboPlonk proof
    /// @return _ A boolean indicating successful verification, false otherwise
    function verify(
        IPlonkVerifier.VerifyingKey memory verifyingKey,
        uint256[5] memory publicInput,
        IPlonkVerifier.PlonkProof memory proof
    ) external view returns (bool) {
        _validateProof(proof);

        // Validate publicInput scalars inline (saves 5 function-call JUMPs)
        assembly {
            let R := 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
            for { let i := 0 } lt(i, 5) { i := add(i, 1) } {
                if iszero(lt(mload(add(publicInput, mul(i, 0x20))), R)) {
                    mstore(0x00, 0x05b05ccc00000000000000000000000000000000000000000000000000000000)
                    revert(0x00, 0x04)
                }
            }
        }

        return _verify(verifyingKey, publicInput, proof);
    }

    /// @dev Validate all group points and scalar fields. Revert if any are invalid.
    /// Single assembly block to eliminate 41+ function-call JUMPs from the original.
    /// @param proof A Plonk proof
    function _validateProof(IPlonkVerifier.PlonkProof memory proof) internal pure {
        assembly {
            // P_MOD (BN254 base field prime) and R_MOD (scalar field order)
            let P := 0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47
            let R := 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
            // 4-byte custom error selectors (left-aligned in 32-byte slot)
            let INVALID_G1 := 0x9e78d14c00000000000000000000000000000000000000000000000000000000
            let INVALID_SCALAR := 0x05b05ccc00000000000000000000000000000000000000000000000000000000

            // Validate 13 G1Points: proof offsets 0x00..0x180 are pointers to G1Point structs.
            // Check each point: infinity (0,0) is valid; otherwise verify x,y < P and curve eq.
            for { let i := 0 } lt(i, 13) { i := add(i, 1) } {
                let ptPtr := mload(add(proof, mul(i, 0x20)))
                let px := mload(ptPtr)
                let py := mload(add(ptPtr, 0x20))
                if iszero(and(iszero(px), iszero(py))) {
                    if iszero(
                        and(
                            and(lt(px, P), lt(py, P)),
                            eq(mulmod(py, py, P), addmod(mulmod(px, mulmod(px, px, P), P), 3, P))
                        )
                    ) {
                        mstore(0x00, INVALID_G1)
                        revert(0x00, 0x04)
                    }
                }
            }

            // Validate 10 ScalarField values at proof offsets 0x1A0..0x2C0 (inline uint256, not pointers).
            for { let i := 0 } lt(i, 10) { i := add(i, 1) } {
                if iszero(lt(mload(add(add(proof, 0x1a0), mul(i, 0x20))), R)) {
                    mstore(0x00, INVALID_SCALAR)
                    revert(0x00, 0x04)
                }
            }
        }
    }

    // core verifier logic, assuming all input arguments are validated
    function _verify(
        IPlonkVerifier.VerifyingKey memory verifyingKey,
        uint256[5] memory publicInput,
        IPlonkVerifier.PlonkProof memory proof
    ) private view returns (bool) {
        if (verifyingKey.numInputs != 5) revert WrongPlonkVK();

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
            scalar := mload(add(chal, 0x80))
            // chal.zeta * groupGen or nextEvalPoint or zetaOmega
            scalar := mulmod(scalar, mload(add(mload(add(domain, 0x40)), 0x20)), p)
            // u * zetaOmega or "uzω"
            scalar := mulmod(scalar, mload(add(chal, 0xc0)), p)
        }
        // b += proof.zetaOmega^(u * chal.zeta * groupGen)
        b = BN254.add(b, BN254.scalarMul(proof.zetaOmega, BN254.ScalarField.wrap(scalar)));

        // Check e(A, [x]2) =?= e(B, [1]2)
        // Equivalently, e(A, [x]2) * e(-B, [1]2) =?= 1
        return _pairingCheck(a, b);
    }

    /// @dev Pairing check e(a, betaH) · e(-b, P2) == 1 with hardcoded G2 constants.
    /// Avoids allocating G2Point structs and calling BN254.pairingProd2 / BN254.negate.
    function _pairingCheck(BN254.G1Point memory a, BN254.G1Point memory b)
        private
        view
        returns (bool res)
    {
        // Precompile 0x08 input layout for 2 pairs (0x180 bytes):
        //   [a.x, a.y, betaH.x1, betaH.x0, betaH.y1, betaH.y0,
        //    (-b).x, (-b).y, P2.x1, P2.x0, P2.y1, P2.y0]
        // G2 precompile format: [x1, x0, y1, y0]
        // betaH: x1=BETA_H_X0, x0=BETA_H_X1, y1=BETA_H_Y0, y0=BETA_H_Y1
        assembly {
            let sc := mload(0x40) // borrow scratch, never advance
            // Pair 1: a and betaH
            mstore(sc, mload(a))
            mstore(add(sc, 0x20), mload(add(a, 0x20)))
            mstore(add(sc, 0x40), BETA_H_X0)
            mstore(add(sc, 0x60), BETA_H_X1)
            mstore(add(sc, 0x80), BETA_H_Y0)
            mstore(add(sc, 0xa0), BETA_H_Y1)
            // Pair 2: -b and P2
            mstore(add(sc, 0xc0), mload(b))
            mstore(
                add(sc, 0xe0),
                sub(0x30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47, mload(add(b, 0x20)))
            )
            mstore(add(sc, 0x100), 0x198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c2)
            mstore(add(sc, 0x120), 0x1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed)
            mstore(add(sc, 0x140), 0x090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b)
            mstore(add(sc, 0x160), 0x12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa)
            if iszero(staticcall(gas(), 8, sc, 0x180, 0x00, 0x20)) { revert(0, 0) }
            res := mload(0x00)
        }
    }

    function _computeChallenges(
        IPlonkVerifier.VerifyingKey memory vk,
        uint256[5] memory pi,
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

            // proof
            let wire0Ptr := mload(proof)
            mstore(add(dataPtr, 0x620), mload(wire0Ptr)) // wire0.x
            mstore(add(dataPtr, 0x640), mload(add(wire0Ptr, 0x20))) // wire0.y
            let wire1Ptr := mload(add(proof, 0x20))
            mstore(add(dataPtr, 0x660), mload(wire1Ptr)) // wire1.x
            mstore(add(dataPtr, 0x680), mload(add(wire1Ptr, 0x20))) // wire1.y
            let wire2Ptr := mload(add(proof, 0x40))
            mstore(add(dataPtr, 0x6a0), mload(wire2Ptr)) // wire2.x
            mstore(add(dataPtr, 0x6c0), mload(add(wire2Ptr, 0x20))) // wire2.y
            let wire3Ptr := mload(add(proof, 0x60))
            mstore(add(dataPtr, 0x6e0), mload(wire3Ptr)) // wire3.x
            mstore(add(dataPtr, 0x700), mload(add(wire3Ptr, 0x20))) // wire3.y
            let wire4Ptr := mload(add(proof, 0x80))
            mstore(add(dataPtr, 0x720), mload(wire4Ptr)) // wire4.x
            mstore(add(dataPtr, 0x740), mload(add(wire4Ptr, 0x20))) // wire4.y

            // challenge: beta
            {
                mstore(statePtr, 0x0) // init state
                // preimage len: state(0x20) + transcript(0x760)
                // overwrite previous state at freePtr
                mstore(statePtr, keccak256(statePtr, 0x780))
                // (mod p) to get beta
                mstore(add(res, 0x40), mod(mload(statePtr), p))
            }

            // challenge: gamma
            {
                // preimage len: state(0x20) + transcript(0x0)
                // overwrite previous state at freePtr
                mstore(statePtr, keccak256(statePtr, 0x20))
                // (mod p) to get gamma
                mstore(add(res, 0x60), mod(mload(statePtr), p))
            }

            let prodPermPtr := mload(add(proof, 0xa0))
            mstore(dataPtr, mload(prodPermPtr)) // prodPerm.x
            mstore(add(dataPtr, 0x20), mload(add(prodPermPtr, 0x20))) // prodPerm.y

            // challenge: alpha, alpha2
            {
                // preimage len: state(0x20) + transcript(0x40)
                let alpha := keccak256(statePtr, 0x60)
                mstore(statePtr, alpha)
                // (mod p) to get challenge
                mstore(res, mod(alpha, p))

                let alpha2 := mulmod(alpha, alpha, p)
                mstore(add(res, 0x20), alpha2)
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
                mstore(add(res, 0x80), mod(mload(statePtr), p))
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
                mstore(add(res, 0xa0), mod(mload(statePtr), p))
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
                mstore(add(res, 0xc0), mod(hash, p))
            }
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
        uint256 p = BN254.R_MOD;

        // ============================================
        // Pre-compute sigmaWireProd = ∏_{i=0..3}(w_i + beta*sigma_i + gamma)
        // and inline _computeLinPolyConstantTerm to reuse this product,
        // avoiding a second pass over the same proof fields.
        // ============================================
        uint256 sigmaWireProd;
        uint256 eval;
        assembly {
            let beta := mload(add(chal, 0x40))
            let gamma := mload(add(chal, 0x60))
            sigmaWireProd := 1
            {
                let w := mload(add(proof, 0x1a0))
                let s := mload(add(proof, 0x240))
                sigmaWireProd :=
                    mulmod(sigmaWireProd, addmod(addmod(w, gamma, p), mulmod(beta, s, p), p), p)
            }
            {
                let w := mload(add(proof, 0x1c0))
                let s := mload(add(proof, 0x260))
                sigmaWireProd :=
                    mulmod(sigmaWireProd, addmod(addmod(w, gamma, p), mulmod(beta, s, p), p), p)
            }
            {
                let w := mload(add(proof, 0x1e0))
                let s := mload(add(proof, 0x280))
                sigmaWireProd :=
                    mulmod(sigmaWireProd, addmod(addmod(w, gamma, p), mulmod(beta, s, p), p), p)
            }
            {
                let w := mload(add(proof, 0x200))
                let s := mload(add(proof, 0x2a0))
                sigmaWireProd :=
                    mulmod(sigmaWireProd, addmod(addmod(w, gamma, p), mulmod(beta, s, p), p), p)
            }
            // Inline constant term: eval = -(piEval - alpha2 * L1(zeta) - alpha * perm)
            // perm = sigmaWireProd * (w4 + gamma) * z_w
            let alpha := mload(chal)
            let alpha2 := mload(add(chal, 0x20))
            let perm :=
                mulmod(
                    sigmaWireProd,
                    mulmod(addmod(mload(add(proof, 0x220)), gamma, p), mload(add(proof, 0x2c0)), p),
                    p
                )
            let r0 := addmod(mload(add(evalData, 0x40)), sub(p, mulmod(alpha2, mload(add(evalData, 0x20)), p)), p)
            r0 := addmod(r0, sub(p, mulmod(alpha, perm, p)), p)
            eval := sub(p, r0) // -r0
        }

        // Compute first part of batched polynomial commitment [D]1.
        // Pass sigmaWireProd so _linearizationPolyComm can reuse it for secondScalar.
        BN254.G1Point memory d1 =
            _linearizationPolyComm(verifyingKey, chal, evalData, proof, sigmaWireProd);

        // ============================================
        // Add wire witness poly commitments + compute eval accumulation.
        // Scratch-buffer pattern: borrow mload(0x40) without advancing it.
        // ============================================
        uint256 v = chal.v;
        uint256 vPow = v;
        assembly {
            let sc := mload(0x40) // borrow scratch, NEVER advance

            // ------------------------------------------------------------------
            // f1 = d1 + wire0 * v   (v^1)
            // ------------------------------------------------------------------
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x1a0)), p), p) // eval += v*wireEval0
                let ptr := mload(proof) // wire0 pointer
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40)) // ecMul → sc
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40)) // ecAdd(d1,sc) → f1
            }
            // ------------------------------------------------------------------
            // f1 += wire1 * v^2
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x1c0)), p), p)
                let ptr := mload(add(proof, 0x20)) // wire1
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += wire2 * v^3
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x1e0)), p), p)
                let ptr := mload(add(proof, 0x40)) // wire2
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += wire3 * v^4
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x200)), p), p)
                let ptr := mload(add(proof, 0x60)) // wire3
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += wire4 * v^5
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x220)), p), p)
                let ptr := mload(add(proof, 0x80)) // wire4
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += sigma0 * v^6
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x240)), p), p)
                let ptr := mload(add(verifyingKey, 0x40)) // sigma0
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += sigma1 * v^7
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x260)), p), p)
                let ptr := mload(add(verifyingKey, 0x60)) // sigma1
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += sigma2 * v^8
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x280)), p), p)
                let ptr := mload(add(verifyingKey, 0x80)) // sigma2
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += sigma3 * v^9
            // ------------------------------------------------------------------
            vPow := mulmod(vPow, v, p)
            {
                eval := addmod(eval, mulmod(vPow, mload(add(proof, 0x2a0)), p), p)
                let ptr := mload(add(verifyingKey, 0xa0)) // sigma3
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), vPow)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // f1 += prodPerm * u   (zeta*omega evaluation point)
            // eval += u * prodPermZetaOmegaEval
            // ------------------------------------------------------------------
            {
                let u := mload(add(chal, 0xc0)) // chal.u
                eval := addmod(eval, mulmod(u, mload(add(proof, 0x2c0)), p), p)
                let ptr := mload(add(proof, 0xa0)) // prodPerm
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), u)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(f1))
                mstore(add(sc, 0x60), mload(add(f1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, f1, 0x40))
            }
            // ------------------------------------------------------------------
            // e1 = eval * [1]1  (P1 = BN254 generator = (1, 2))
            // ------------------------------------------------------------------
            mstore(sc, 1)
            mstore(add(sc, 0x20), 2)
            mstore(add(sc, 0x40), eval)
            pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, e1, 0x40))
        }
    }

    /// @dev Compute the linearization poly commitment
    /// @param verifyingKey The verifying key
    /// @param challenge A set of challenges
    /// @param evalData Polynomial evaluation data
    /// @param proof A Plonk proof
    /// @param sigmaWireProd Pre-computed ∏_{i=0..3}(w_i + beta*sigma_i + gamma), shared with caller
    /// @return d1 The [D]1 in Step 9 of Plonk
    function _linearizationPolyComm(
        IPlonkVerifier.VerifyingKey memory verifyingKey,
        Challenges memory challenge,
        Poly.EvalData memory evalData,
        IPlonkVerifier.PlonkProof memory proof,
        uint256 sigmaWireProd
    ) private view returns (BN254.G1Point memory d1) {
        uint256 tmpScalar;
        uint256 tmp;
        uint256 p = BN254.R_MOD;

        // Pre-compute zeta^2 (resolves todo on split quotient section)
        uint256 zeta2;
        assembly {
            zeta2 := mulmod(mload(add(challenge, 0x80)), mload(add(challenge, 0x80)), p)
        }

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
            let gamma := mload(add(challenge, 0x60))
            // firstScalar = alpha^2 * L1(zeta)
            tmpScalar := mulmod(mload(add(challenge, 0x20)), mload(add(evalData, 0x20)), p)

            // rhs = alpha  (rhs is assembly-local to free a Solidity stack slot)
            let rhs := mload(challenge)

            // tmp = beta * zeta
            tmp := mulmod(mload(add(challenge, 0x40)), mload(add(challenge, 0x80)), p)

            // =================================
            // k0 (which is 1) component
            // (beta * zeta + wireEval0 + gamma)
            // =================================
            let tmp2 := addmod(tmp, mload(add(proof, 0x1A0)), p)
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
        // ============================================
        // EC accumulation using scratch-buffer pattern.
        // Borrows the free-memory pointer WITHOUT advancing it, eliminating heap allocation
        // (160 bytes per ecMul + 192 bytes per ecAdd = 352 bytes per pair) and function-call
        // overhead (~50 gas per pair). 19 ecMul + 20 ecAdd calls follow.
        // ============================================
        assembly {
            let sc := mload(0x40) // scratch buffer: borrow, NEVER advance 0x40

            // ------------------------------------------------------------------
            // Initialize d1 = prodPerm * firstScalar
            // ------------------------------------------------------------------
            {
                let ptr := mload(add(proof, 0xa0)) // prodPerm pointer
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), tmpScalar) // firstScalar computed above
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, d1, 0x40)) // ecMul → d1
            }

            // ------------------------------------------------------------------
            // d1 += sigma4 * (-secondScalar)
            // secondScalar = alpha * beta * z_w * sigmaWireProd (pre-computed by caller)
            // ------------------------------------------------------------------
            {
                let alpha := mload(challenge)
                let beta := mload(add(challenge, 0x40))
                let s := mulmod(mulmod(mulmod(alpha, beta, p), mload(add(proof, 0x2C0)), p), sigmaWireProd, p)
                let ptr := mload(add(verifyingKey, 0xc0)) // sigma4
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), sub(p, s)) // negate
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }

            // ------------------------------------------------------------------
            // q_lc: d1 += q1 * wireEval0, q2 * wireEval1, q3 * wireEval2, q4 * wireEval3
            // ------------------------------------------------------------------
            {
                let ptr := mload(add(verifyingKey, 0xe0)) // q1
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mload(add(proof, 0x1a0))) // wireEval0
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            {
                let ptr := mload(add(verifyingKey, 0x100)) // q2
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mload(add(proof, 0x1c0))) // wireEval1
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            {
                let ptr := mload(add(verifyingKey, 0x120)) // q3
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mload(add(proof, 0x1e0))) // wireEval2
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            {
                let ptr := mload(add(verifyingKey, 0x140)) // q4
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mload(add(proof, 0x200))) // wireEval3
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }

            // ------------------------------------------------------------------
            // q_M: d1 += qM12 * (w0*w1),  d1 += qM34 * (w2*w3)
            // Cache w01, w23 as assembly locals for qEcc reuse.
            // ------------------------------------------------------------------
            let w01 := mulmod(mload(add(proof, 0x1A0)), mload(add(proof, 0x1C0)), p)
            {
                let ptr := mload(add(verifyingKey, 0x160)) // qM12
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), w01)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            let w23 := mulmod(mload(add(proof, 0x1E0)), mload(add(proof, 0x200)), p)
            {
                let ptr := mload(add(verifyingKey, 0x180)) // qM34
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), w23)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }

            // ------------------------------------------------------------------
            // q_H: d1 += qH1 * w0^5, qH2 * w1^5, qH3 * w2^5, qH4 * w3^5
            // w^5 = w * (w^2)^2
            // ------------------------------------------------------------------
            {
                let w := mload(add(proof, 0x1A0))
                let w2 := mulmod(w, w, p)
                let ptr := mload(add(verifyingKey, 0x1e0)) // qH1
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mulmod(w, mulmod(w2, w2, p), p))
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            {
                let w := mload(add(proof, 0x1C0))
                let w2 := mulmod(w, w, p)
                let ptr := mload(add(verifyingKey, 0x200)) // qH2
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mulmod(w, mulmod(w2, w2, p), p))
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            {
                let w := mload(add(proof, 0x1E0))
                let w2 := mulmod(w, w, p)
                let ptr := mload(add(verifyingKey, 0x220)) // qH3
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mulmod(w, mulmod(w2, w2, p), p))
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            {
                let w := mload(add(proof, 0x200))
                let w2 := mulmod(w, w, p)
                let ptr := mload(add(verifyingKey, 0x240)) // qH4
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mulmod(w, mulmod(w2, w2, p), p))
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }

            // ------------------------------------------------------------------
            // q_O: d1 += qO * (-wireEval4)
            // ------------------------------------------------------------------
            {
                let ptr := mload(add(verifyingKey, 0x1a0)) // qO
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), sub(p, mload(add(proof, 0x220)))) // -wireEval4
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }

            // ------------------------------------------------------------------
            // q_C: d1 += qC  (constant term, ecAdd only — no scalar multiplication)
            // ------------------------------------------------------------------
            {
                let ptr := mload(add(verifyingKey, 0x1c0)) // qC
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }

            // ------------------------------------------------------------------
            // q_Ecc: d1 += qEcc * (w0*w1*w2*w3*w4)
            // Reuse assembly-local w01 = w0*w1, w23 = w2*w3 computed above.
            // ------------------------------------------------------------------
            {
                let ptr := mload(add(verifyingKey, 0x260)) // qEcc
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), mulmod(mulmod(w01, w23, p), mload(add(proof, 0x220)), p))
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }

            // ------------------------------------------------------------------
            // Split quotient terms (5 terms):
            // scalar[0] = 1 - zeta^n  = p - vanishEval
            // scalar[k] = scalar[k-1] * zeta^(n+2)  where zeta^(n+2) = (vanishEval+1) * zeta2
            // ------------------------------------------------------------------
            let vanish := mload(evalData) // vanishEval = zeta^n - 1
            let zetaNp2 := mulmod(addmod(vanish, 1, p), zeta2, p)
            let splitS := sub(p, vanish) // (1 - zeta^n)
            {
                let ptr := mload(add(proof, 0xc0)) // split0
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), splitS)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            splitS := mulmod(splitS, zetaNp2, p)
            {
                let ptr := mload(add(proof, 0xe0)) // split1
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), splitS)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            splitS := mulmod(splitS, zetaNp2, p)
            {
                let ptr := mload(add(proof, 0x100)) // split2
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), splitS)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            splitS := mulmod(splitS, zetaNp2, p)
            {
                let ptr := mload(add(proof, 0x120)) // split3
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), splitS)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
            splitS := mulmod(splitS, zetaNp2, p)
            {
                let ptr := mload(add(proof, 0x140)) // split4
                mstore(sc, mload(ptr))
                mstore(add(sc, 0x20), mload(add(ptr, 0x20)))
                mstore(add(sc, 0x40), splitS)
                pop(staticcall(sub(gas(), 2000), 7, sc, 0x60, sc, 0x40))
                mstore(add(sc, 0x40), mload(d1))
                mstore(add(sc, 0x60), mload(add(d1, 0x20)))
                pop(staticcall(sub(gas(), 2000), 6, sc, 0x80, d1, 0x40))
            }
        }
    }
}
