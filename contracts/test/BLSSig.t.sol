// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
import { BN254 } from "bn254/BN254.sol";

// Target contract
import { BLSSig } from "../src/libraries/BLSSig.sol";

contract BLSSig_Test is Test {
    function genBLSSig(bytes memory message)
        private
        returns (BN254.G2Point memory, BN254.G1Point memory)
    {
        // Generate a BLS verification key and signature  using rust code
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "gen-bls-sig";
        cmds[2] = vm.toString(message);

        bytes memory result = vm.ffi(cmds);

        (BN254.G2Point memory vk, BN254.G1Point memory sig) =
            abi.decode(result, (BN254.G2Point, BN254.G1Point));
        return (vk, sig);
    }

    function testFuzz_BLS_hashes_computation(bytes memory input) external {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "gen-bls-hashes";
        cmds[2] = vm.toString(input);

        bytes memory result = vm.ffi(cmds);

        (uint256 fieldElem, BN254.G1Point memory hashToCurveElem) =
            abi.decode(result, (uint256, BN254.G1Point));

        assertEq(fieldElem, BLSSig.hashToField(input));
        assertEq(abi.encode(hashToCurveElem), abi.encode(BLSSig.hashToCurve(input)));
    }

    function test_SigVerification_Succeeds() external {
        bytes memory message = "Hi";
        (BN254.G2Point memory vk, BN254.G1Point memory sig) = genBLSSig(message);
        BLSSig.verifyBlsSig(message, sig, vk);
    }

    // This is due to a current limitation/bug of foundry. See
    // https://github.com/foundry-rs/foundry/issues/4405
    // See https://github.com/EspressoSystems/espresso-sequencer/issues/847
    function wrapVerifyBlsSig(
        bytes memory message,
        BN254.G1Point memory sig,
        BN254.G2Point memory vk
    ) public view {
        BLSSig.verifyBlsSig(message, sig, vk);
    }

    function testFuzz_RevertWhen_SignatureIsInvalid(uint256 exp) external {
        bytes memory message = "Hi";
        BN254.ScalarField expScalar = BN254.ScalarField.wrap(exp);

        BN254.G1Point memory badSig = BN254.P1();
        badSig = BN254.scalarMul(badSig, expScalar);
        (BN254.G2Point memory vk,) = genBLSSig(message);
        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        this.wrapVerifyBlsSig(message, badSig, vk);
    }

    function testFuzz_RevertWhen_usingWrongVK(uint64 exp) external {
        bytes memory message = "Hi";
        exp = uint64(bound(exp, 1, type(uint64).max));

        (BN254.G2Point memory vk, BN254.G1Point memory sig) = genBLSSig(message);
        vk; // To avoid compiler warning

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "gen-random-g2-point";
        cmds[2] = vm.toString(exp);

        bytes memory result = vm.ffi(cmds);

        BN254.G2Point memory badVK = abi.decode(result, (BN254.G2Point));

        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        this.wrapVerifyBlsSig(message, sig, badVK);
    }

    /// @dev Ensure the verification can detect invalid points provided as signature. Note: checking
    /// pk belong to G2 is not possible in practice
    /// https://ethresear.ch/t/fast-mathbb-g-2-subgroup-check-in-bn254/13974
    function testFuzz_RevertWhen_SignatureIsAnInvalidPoint(uint256 x, uint256 y) external {
        bytes memory message = "Hi";
        (BN254.G2Point memory vk,) = genBLSSig(message);

        // Make sure the point is invalid by picking a non valid field element for the x component.
        uint256 invalidX = bound(x, BN254.P_MOD, type(uint256).max);
        uint256 invalidY = bound(y, BN254.P_MOD, type(uint256).max);

        BN254.G1Point memory invalidPoint =
            BN254.G1Point(BN254.BaseField.wrap(invalidX), BN254.BaseField.wrap(2));
        vm.expectRevert("InvalidG1()");
        this.wrapVerifyBlsSig(message, invalidPoint, vk);

        invalidPoint = BN254.G1Point(BN254.BaseField.wrap(10), BN254.BaseField.wrap(invalidY));
        vm.expectRevert("InvalidG1()");
        this.wrapVerifyBlsSig(message, invalidPoint, vk);
    }
}
