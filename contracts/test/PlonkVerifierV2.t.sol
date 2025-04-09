// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */
/* solhint-disable no-inline-assembly  */

// NOTE: For developers and auditors: we mainly test the consistency between the outputs between
// Solidity and Jellyfish library, with the help of fuzzer-generated inputs from Forge Testing.
// Inside the logic of `verify()`, variables values only need to be consistent and valid
// (i.e. valid group or field elements) and don't need to be from a correct proof/public input.
// Only the last step `_verifyOpeningProof` will test *correctness* of these parameters.
// Therefore, we employ more randomly generated dummy inputs for most tests for robustness,
// and only rely on Rust-code to generate correct inputs for the `_verifyOpeningProof`.

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
import { BN254 } from "bn254/BN254.sol";
import { IPlonkVerifier } from "../src/interfaces/IPlonkVerifier.sol";
import { LightClientStateUpdateVKV2 as VkLib } from
    "../src/libraries/LightClientStateUpdateVKV2.sol";
import { PolynomialEvalV2 as Poly } from "../src/libraries/PolynomialEvalV2.sol";

// Target contract
import { PlonkVerifierV2 as V } from "../src/libraries/PlonkVerifierV2.sol";

/// @dev Common helpers/utils for PlonkVerifier tests
contract PlonkVerifierCommonTest is Test {
    IPlonkVerifier.VerifyingKey vk;

    constructor() {
        vk = VkLib.getVk();
    }

    /// @dev Sanitize a single value to be valid scalar field Bn254::Fr.
    function sanitizeScalarField(uint256 a) public pure returns (uint256) {
        a = bound(a, 0, BN254.R_MOD - 1);
        BN254.validateScalarField(BN254.ScalarField.wrap(a));
        return a;
    }

    /// @dev Sanitize all values in `a` to be valid scalar fields Bn254::Fr.
    /// This is helpful to sanitize fuzzer-generated random `uint[]` values.
    function sanitizeScalarFields(uint256[11] memory a) public pure returns (uint256[11] memory) {
        for (uint256 i = 0; i < a.length; i++) {
            a[i] = sanitizeScalarField(a[i]);
        }
        return a;
    }

    /// @dev Generate a random valid (format-wise) proof from a random seed
    function dummyProof(uint64 seed) public returns (IPlonkVerifier.PlonkProof memory) {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "dummy-proof";
        cmds[2] = vm.toString(seed);

        bytes memory result = vm.ffi(cmds);
        (IPlonkVerifier.PlonkProof memory proof) = abi.decode(result, (IPlonkVerifier.PlonkProof));
        return proof;
    }

    /// @dev helper function to generate some dummy but format-valid arguments for
    /// `prepareOpeningProof` step. The verifyingKey should be fixed/loaded from library,
    /// proof should be generated via `dummyProof()`, other inputs are from fuzzers.
    function dummyArgsForOpeningProof(uint64 seed, uint256[11] memory publicInput)
        public
        returns (
            IPlonkVerifier.VerifyingKey memory,
            IPlonkVerifier.PlonkProof memory,
            V.Challenges memory,
            Poly.EvalData memory
        )
    {
        IPlonkVerifier.PlonkProof memory proof = dummyProof(seed);
        V.Challenges memory chal = V._computeChallenges(vk, publicInput, proof);

        Poly.EvalDomain memory domain = Poly.newEvalDomain(vk.domainSize);
        // pre-compute evaluation data
        Poly.EvalData memory evalData = Poly.evalDataGen(domain, chal.zeta, publicInput);

        return (vk, proof, chal, evalData);
    }
}

contract PlonkVerifier_constants_Test is PlonkVerifierCommonTest {
    /// @dev Test constants declared matches that from Jellyfish
    function test_correctConstants() external {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "plonk-constants";

        bytes memory result = vm.ffi(cmds);
        (
            uint256 k1,
            uint256 k2,
            uint256 k3,
            uint256 k4,
            uint256 betaHX0,
            uint256 betaHX1,
            uint256 betaHY0,
            uint256 betaHY1
        ) = abi.decode(
            result, (uint256, uint256, uint256, uint256, uint256, uint256, uint256, uint256)
        );

        assertEq(k1, V.COSET_K1);
        assertEq(k2, V.COSET_K2);
        assertEq(k3, V.COSET_K3);
        assertEq(k4, V.COSET_K4);
        assertEq(betaHX0, V.BETA_H_X0);
        assertEq(betaHX1, V.BETA_H_X1);
        assertEq(betaHY0, V.BETA_H_Y0);
        assertEq(betaHY1, V.BETA_H_Y1);
    }
}

// Mostly identical with `PlonkVerifier_batchVerify_Test`
contract PlonkVerifier_verify_Test is PlonkVerifierCommonTest {
    /// @dev Test happy path of `verify`.
    function test_verify_succeeds() external {
        vm.pauseGasMetering();
        string[] memory cmds = new string[](2);
        cmds[0] = "diff-test";
        cmds[1] = "plonk-verify";

        bytes memory result = vm.ffi(cmds);
        (
            IPlonkVerifier.VerifyingKey memory verifyingKey,
            uint256[11] memory publicInput,
            IPlonkVerifier.PlonkProof memory proof
        ) = abi.decode(
            result, (IPlonkVerifier.VerifyingKey, uint256[11], IPlonkVerifier.PlonkProof)
        );

        vm.resumeGasMetering();
        assert(V.verify(verifyingKey, publicInput, proof));
    }

    /// @dev Test when bad verifying key is supplied, the verification should fail
    function testFuzz_badVerifyingKey_fails(uint256 nthPoint) external {
        string[] memory cmds = new string[](2);
        cmds[0] = "diff-test";
        cmds[1] = "plonk-verify";

        bytes memory result = vm.ffi(cmds);
        (
            IPlonkVerifier.VerifyingKey memory verifyingKey,
            uint256[11] memory publicInput,
            IPlonkVerifier.PlonkProof memory proof
        ) = abi.decode(
            result, (IPlonkVerifier.VerifyingKey, uint256[11], IPlonkVerifier.PlonkProof)
        );

        // there are 18 points in verifying key
        // randomly choose one to mutate
        nthPoint = bound(nthPoint, 0, 17);

        BN254.G1Point memory badPoint;
        assembly {
            // the first point offset is 0x40
            let badPointRef := add(verifyingKey, add(mul(nthPoint, 0x20), 0x40))
            badPoint := mload(badPointRef)
        }

        // modify the point to be invalid
        badPoint = BN254.add(badPoint, BN254.P1());

        assembly {
            let badPointRef := add(verifyingKey, add(mul(nthPoint, 0x20), 0x40))
            mstore(badPointRef, badPoint)
        }

        assert(!V.verify(verifyingKey, publicInput, proof));
    }

    // @dev Test when bad public input is supplied, the verification should fail
    // We know our `gen_circuit_for_test` in `diff_test.rs` has only 8 public inputs
    function testFuzz_badPublicInput_fails(uint256[11] calldata randPublicInput) external {
        uint256[11] memory badPublicInput;
        for (uint256 i = 0; i < 11; i++) {
            badPublicInput[i] = randPublicInput[i];
        }
        badPublicInput = sanitizeScalarFields(badPublicInput);

        string[] memory cmds = new string[](2);
        cmds[0] = "diff-test";
        cmds[1] = "plonk-verify";

        bytes memory result = vm.ffi(cmds);
        (IPlonkVerifier.VerifyingKey memory verifyingKey,, IPlonkVerifier.PlonkProof memory proof) =
        abi.decode(result, (IPlonkVerifier.VerifyingKey, uint256[11], IPlonkVerifier.PlonkProof));

        assert(!V.verify(verifyingKey, badPublicInput, proof));
    }

    /// @dev Test when bad proof is supplied, the verification should fail
    function testFuzz_badProof_fails(uint64 seed) external {
        IPlonkVerifier.PlonkProof memory badProof = dummyProof(seed);

        string[] memory cmds = new string[](2);
        cmds[0] = "diff-test";
        cmds[1] = "plonk-verify";

        bytes memory result = vm.ffi(cmds);
        (IPlonkVerifier.VerifyingKey memory verifyingKey, uint256[11] memory publicInput,) = abi
            .decode(result, (IPlonkVerifier.VerifyingKey, uint256[11], IPlonkVerifier.PlonkProof));

        assert(!V.verify(verifyingKey, publicInput, badProof));
    }
}

contract PlonkVerifier_validateProof_Test is PlonkVerifierCommonTest {
    /// @dev Test that a valid proof shouldn't revert
    function test_validProof_succeeds() external {
        // a valid proof
        IPlonkVerifier.PlonkProof memory proof = dummyProof(42);

        V._validateProof(proof);
    }

    /// @dev Randomly pick a coordinate of a point among points in a proof
    /// mutate it to another value so that the point is no longer valid,
    /// test if our check will revert.
    /// forge-config: default.allow_internal_expect_revert = true
    function testFuzz_RevertIfProofContainsInvalidGroup(uint256 nthPoint, bool testX) external {
        // a valid proof
        IPlonkVerifier.PlonkProof memory proof = dummyProof(42);

        // we are testing the `nthPoint` in the `proof`,
        // only mutating a single field element (either x or y coordinate)
        // There are 13 points in total.
        nthPoint = bound(nthPoint, 0, 12);

        assembly {
            switch testX
            case true {
                // mutate the x coordinate
                mstore(mload(add(proof, mul(0x20, nthPoint))), 0x1234)
            }
            default {
                // else, mutate y coordinate
                mstore(add(mload(add(proof, mul(0x20, nthPoint))), 0x20), 0x1234)
            }
        }

        vm.expectRevert("Bn254: invalid G1 point");
        V._validateProof(proof);
    }

    /// @dev Randomly pick field in a proof mutate it to invalid value
    /// test if our check will revert.
    /// forge-config: default.allow_internal_expect_revert = true
    function testFuzz_RevertIfProofContainsInvalidField(uint256 nthField) external {
        // a valid proof
        IPlonkVerifier.PlonkProof memory proof = dummyProof(42);
        uint256 invalidField = BN254.R_MOD;

        // we are testing the `nthField` in the `proof`,
        // There are 10 field elements in total (with 13 points in front)
        nthField = bound(nthField, 0, 9);

        assembly {
            let start := add(proof, mul(0x20, 13))
            mstore(add(start, mul(nthField, 0x20)), invalidField)
        }

        vm.expectRevert("Bn254: invalid scalar field");
        V._validateProof(proof);
    }
}

contract PlonkVerifier_computeChallenges_Test is PlonkVerifierCommonTest {
    /// @dev Test `computeChallenges` matches that of Jellyfish
    function testFuzz_computeChallenges_matches(uint64 seed, uint256[11] memory publicInput)
        external
    {
        IPlonkVerifier.PlonkProof memory proof = dummyProof(seed);
        publicInput = sanitizeScalarFields(publicInput);

        string[] memory cmds = new string[](5);
        cmds[0] = "diff-test";
        cmds[1] = "plonk-compute-chal";
        cmds[2] = vm.toString(abi.encode(vk));
        cmds[3] = vm.toString(abi.encode(publicInput));
        cmds[4] = vm.toString(abi.encode(proof));

        bytes memory result = vm.ffi(cmds);
        V.Challenges memory chal = abi.decode(result, (V.Challenges));

        V.Challenges memory c = V._computeChallenges(vk, publicInput, proof);
        assertEq(chal.alpha, c.alpha);
        assertEq(chal.alpha2, c.alpha2);
        assertEq(chal.alpha3, c.alpha3);
        assertEq(chal.beta, c.beta);
        assertEq(chal.gamma, c.gamma);
        assertEq(chal.zeta, c.zeta);
        assertEq(chal.v, c.v);
        assertEq(chal.u, c.u);
    }
}
