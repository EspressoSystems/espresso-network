// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
import { BN254 } from "bn254/BN254.sol";

// Target contract
import { PolynomialEvalV2 as Poly } from "../src/libraries/PolynomialEvalV2.sol";

contract PolynomialEval_newEvalDomain_Test is Test {
    /// @dev diff-test with Rust when `domainSize` is in {2^16 ~ 2^20, 2^5}
    function test_supportedDomainSize_matches() external {
        uint256[3] memory logSizes = [uint256(5), 16, 20];
        for (uint256 i = 0; i < 3; i++) {
            string[] memory cmds = new string[](3);
            cmds[0] = "diff-test";
            cmds[1] = "new-poly-eval-domain";
            cmds[2] = vm.toString(logSizes[i]);

            bytes memory result = vm.ffi(cmds);
            (uint256 sizeInv, uint256 groupGen) = abi.decode(result, (uint256, uint256));

            Poly.EvalDomain memory domain = Poly.newEvalDomain(2 ** logSizes[i]);
            assertEq(sizeInv, domain.sizeInv);
            assertEq(groupGen, domain.elements[1]);
        }
    }

    /// @dev Test revert if domainSize is not among {2^16 ~ 2^20, 2^5}
    /// forge-config: default.allow_internal_expect_revert = true
    function testFuzz_unsupportedDomainSize_reverts(uint256 domainSize) external {
        vm.assume(
            domainSize != 2 ** 16 && domainSize != 2 ** 17 && domainSize != 2 ** 18
                && domainSize != 2 ** 19 && domainSize != 2 ** 20 && domainSize != 2 ** 5
        );

        vm.expectRevert(Poly.UnsupportedDegree.selector);
        Poly.newEvalDomain(domainSize);
    }
}

/// @dev Come with some helper function
contract PolynomialEvalTest is Test {
    /// @dev Generate the domain elements for indexes 0..length
    /// which are essentially g^0, g^1, ..., g^{length-1}
    function domainElements(Poly.EvalDomain memory self, uint256 length)
        internal
        pure
        returns (uint256[] memory elements)
    {
        uint256 groupGen = self.elements[1];
        uint256 tmp = 1;
        uint256 p = BN254.R_MOD;
        elements = new uint256[](length);
        assembly {
            if not(iszero(length)) {
                let ptr := add(elements, 0x20)
                let end := add(ptr, mul(0x20, length))
                mstore(ptr, 1)
                ptr := add(ptr, 0x20)
                // for (; ptr < end; ptr += 32) loop through the memory of `elements`
                for { } lt(ptr, end) { ptr := add(ptr, 0x20) } {
                    tmp := mulmod(tmp, groupGen, p)
                    mstore(ptr, tmp)
                }
            }
        }
    }

    /// @dev Test if the domain elements are generated correctly
    function test_domainElements_matches() external {
        uint256 logSize = 20;
        Poly.EvalDomain memory domain = Poly.newEvalDomain(2 ** logSize);

        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "eval-domain-elements";
        cmds[2] = vm.toString(logSize);
        cmds[3] = vm.toString(uint256(11));

        bytes memory result = vm.ffi(cmds);
        (uint256[] memory elems) = abi.decode(result, (uint256[]));

        for (uint256 i = 0; i < 11; i++) {
            assertEq(elems[i], domain.elements[i]);
        }
    }
}

contract PolynomialEval_evalDataGen_Test is PolynomialEvalTest {
    /// @dev Test if evaluations on the vanishing poly, the lagrange one poly, and the public input
    /// poly are correct.
    function testFuzz_evalDataGen_matches(uint256 zeta, uint256[11] memory publicInput) external {
        uint256 logSize = 20;
        zeta = bound(zeta, 0, BN254.R_MOD - 1);
        BN254.validateScalarField(BN254.ScalarField.wrap(zeta));
        // Since these user-provided `publicInputs` were checked outside before passing in via
        // `BN254.validateScalarField()`, it suffices to assume they are proper for our test here.
        for (uint256 i = 0; i < 11; i++) {
            publicInput[i] = bound(publicInput[i], 0, BN254.R_MOD - 1);
            BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[i]));
        }

        Poly.EvalDomain memory domain = Poly.newEvalDomain(2 ** logSize);

        string[] memory cmds = new string[](5);
        cmds[0] = "diff-test";
        cmds[1] = "eval-data-gen";
        cmds[2] = vm.toString(logSize);
        cmds[3] = vm.toString(bytes32(zeta));
        cmds[4] = vm.toString(abi.encode(publicInput));

        bytes memory result = vm.ffi(cmds);
        (uint256 vanishEval, uint256 lagrangeOne, uint256 piEval) =
            abi.decode(result, (uint256, uint256, uint256));

        Poly.EvalData memory evalData = Poly.evalDataGen(domain, zeta, publicInput);
        assertEq(vanishEval, BN254.ScalarField.unwrap(evalData.vanishEval));
        assertEq(lagrangeOne, BN254.ScalarField.unwrap(evalData.lagrangeOne));
        assertEq(piEval, BN254.ScalarField.unwrap(evalData.piEval));
    }

    /// @dev Test edge cases when zeta is one of the elements in the evaluation domain.
    /// The random evaluation point case (most likely outside evalDomain) is already
    /// tested in `testFuzz_evalDataGen_matches()`
    function test_lagrangeOneCoeffForDomainElements() external view {
        uint256 size = 2 ** 5;
        Poly.EvalDomain memory domain = Poly.newEvalDomain(size);

        uint256[] memory elements = domainElements(domain, size);
        uint256 vanishEval = Poly.evaluateVanishingPoly(domain, elements[0]);

        // L_0(g^0) = 1
        assertEq(
            BN254.ScalarField.unwrap(
                Poly.evaluateLagrangeOne(
                    domain, BN254.ScalarField.wrap(elements[0]), BN254.ScalarField.wrap(vanishEval)
                )
            ),
            1
        );

        // L_i(g^0) = 0 for i \in [size]
        for (uint256 i = 1; i < size; i++) {
            vanishEval = Poly.evaluateVanishingPoly(domain, elements[i]);
            assertEq(
                BN254.ScalarField.unwrap(
                    Poly.evaluateLagrangeOne(
                        domain,
                        BN254.ScalarField.wrap(elements[i]),
                        BN254.ScalarField.wrap(vanishEval)
                    )
                ),
                0
            );
        }
    }

    /// @dev Test edge cases when zeta is one of the elements in the evaluation domain.
    function test_evaluatePiPolyForDomainElements() external view {
        uint256 size = 2 ** 5;
        Poly.EvalDomain memory domain = Poly.newEvalDomain(size);

        uint256[] memory elements = domainElements(domain, size);
        uint256[11] memory publicInputs;
        // arbitrarily pick public input length = 10, and fill in arbitrary values
        for (uint256 i = 0; i < 11; i++) {
            publicInputs[i] = 2 ** i;
        }

        for (uint256 i = 0; i < size; i++) {
            uint256 zeta = elements[i];
            uint256 vanishEval = Poly.evaluateVanishingPoly(domain, zeta);
            if (i < 11) {
                assertEq(vanishEval, 0);
                assertEq(
                    Poly.evaluatePiPoly(domain, publicInputs, zeta, vanishEval), publicInputs[i]
                );
            } else {
                assertEq(Poly.evaluatePiPoly(domain, publicInputs, zeta, vanishEval), 0);
            }
        }
    }
}
