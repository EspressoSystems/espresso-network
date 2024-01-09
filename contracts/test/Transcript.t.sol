// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
import { BN254 } from "bn254/BN254.sol";
import { IPlonkVerifier } from "../src/interfaces/IPlonkVerifier.sol";
import { LightClientStateUpdateVKTest as VkTest } from "./mocks/LightClientStateUpdateVKTest.sol";

// Target contract
import { Transcript as T } from "../src/libraries/Transcript.sol";

contract Transcript_appendMessage_Test is Test {
    using T for T.TranscriptData;

    /// @dev Test if `appendMessage` matches that of the Jellyfish's code
    function testFuzz_appendMessage_matches(
        T.TranscriptData memory transcript,
        bytes memory message
    ) external {
        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "transcript-append-msg";
        cmds[2] = vm.toString(abi.encode(transcript));
        cmds[3] = vm.toString(abi.encode(message));

        bytes memory result = vm.ffi(cmds);
        (T.TranscriptData memory updated) = abi.decode(result, (T.TranscriptData));

        transcript.appendMessage(message);
        assertEq(updated.transcript, transcript.transcript);
        assertEq(updated.state[0], transcript.state[0]);
        assertEq(updated.state[1], transcript.state[1]);
    }
}

contract Transcript_appendFieldElement_Test is Test {
    using T for T.TranscriptData;

    /// @dev Test if `appendFieldElement` matches that of Jellyfish
    function testFuzz_appendFieldElement_matches(
        T.TranscriptData memory transcript,
        uint256 fieldElement
    ) external {
        fieldElement = bound(fieldElement, 0, BN254.R_MOD - 1);
        BN254.validateScalarField(BN254.ScalarField.wrap(fieldElement));

        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "transcript-append-field";
        cmds[2] = vm.toString(abi.encode(transcript));
        cmds[3] = vm.toString(bytes32(fieldElement));

        bytes memory result = vm.ffi(cmds);
        (T.TranscriptData memory updated) = abi.decode(result, (T.TranscriptData));

        transcript.appendFieldElement(BN254.ScalarField.wrap(fieldElement));
        assertEq(updated.transcript, transcript.transcript);
        assertEq(updated.state[0], transcript.state[0]);
        assertEq(updated.state[1], transcript.state[1]);
    }
}

contract Transcript_appendGroupElement_Test is Test {
    using T for T.TranscriptData;

    /// @dev Test if `appendGroupElement` matches that of Jellyfish
    function testFuzz_appendGroupElement_matches(
        T.TranscriptData memory transcript,
        uint256 randScalar
    ) external {
        randScalar = bound(randScalar, 0, BN254.R_MOD - 1);
        BN254.validateScalarField(BN254.ScalarField.wrap(randScalar));
        BN254.G1Point memory randPoint =
            BN254.scalarMul(BN254.P1(), BN254.ScalarField.wrap(randScalar));

        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "transcript-append-group";
        cmds[2] = vm.toString(abi.encode(transcript));
        cmds[3] = vm.toString(abi.encode(randPoint));

        bytes memory result = vm.ffi(cmds);
        (T.TranscriptData memory updated) = abi.decode(result, (T.TranscriptData));

        transcript.appendGroupElement(randPoint);
        assertEq(updated.transcript, transcript.transcript);
        assertEq(updated.state[0], transcript.state[0]);
        assertEq(updated.state[1], transcript.state[1]);
    }

    /// @dev Test special case where the identity point (or infinity) is appended.
    function test_appendInfinityPoint_succeeds(T.TranscriptData memory transcript) external {
        BN254.G1Point memory infinity = BN254.infinity();
        assert(BN254.isInfinity(infinity));

        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "transcript-append-group";
        cmds[2] = vm.toString(abi.encode(transcript));
        cmds[3] = vm.toString(abi.encode(infinity));

        bytes memory result = vm.ffi(cmds);
        (T.TranscriptData memory updated) = abi.decode(result, (T.TranscriptData));

        transcript.appendGroupElement(infinity);
        assertEq(updated.transcript, transcript.transcript);
        assertEq(updated.state[0], transcript.state[0]);
        assertEq(updated.state[1], transcript.state[1]);
    }
}

contract Transcript_getAndAppendChallenge_Test is Test {
    using T for T.TranscriptData;

    /// @dev Test if `getAndAppendChallenge` matches that of Jellyfish
    function testFuzz_getAndAppendChallenge_matches(T.TranscriptData memory transcript) external {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "transcript-get-chal";
        cmds[2] = vm.toString(abi.encode(transcript));

        bytes memory result = vm.ffi(cmds);
        (T.TranscriptData memory updated, uint256 chal) =
            abi.decode(result, (T.TranscriptData, uint256));

        uint256 challenge = transcript.getAndAppendChallenge();

        assertEq(updated.transcript, transcript.transcript);
        assertEq(updated.state[0], transcript.state[0]);
        assertEq(updated.state[1], transcript.state[1]);
        assertEq(chal, challenge);
    }
}

contract Transcript_appendVkAndPubInput_Test is Test {
    using T for T.TranscriptData;

    /// @dev Test if `appendVkAndPubInput` matches that of Jellyfish
    function testFuzz_appendVkAndPubInput_matches(
        T.TranscriptData memory transcript,
        uint256[] memory publicInput
    ) external {
        for (uint256 i = 0; i < publicInput.length; i++) {
            publicInput[i] = bound(publicInput[i], 0, BN254.R_MOD - 1);
            BN254.validateScalarField(BN254.ScalarField.wrap(publicInput[i]));
        }
        IPlonkVerifier.VerifyingKey memory vk = VkTest.getVk();

        string[] memory cmds = new string[](5);
        cmds[0] = "diff-test";
        cmds[1] = "transcript-append-vk-and-pi";
        cmds[2] = vm.toString(abi.encode(transcript));
        cmds[3] = vm.toString(abi.encode(vk));
        cmds[4] = vm.toString(abi.encode(publicInput));

        bytes memory result = vm.ffi(cmds);
        (T.TranscriptData memory updated) = abi.decode(result, (T.TranscriptData));

        transcript.appendVkAndPubInput(vk, publicInput);

        assertEq(updated.transcript, transcript.transcript, "transcript field mismatch");
        assertEq(updated.state[0], transcript.state[0], "state[0] field mismatch");
        assertEq(updated.state[1], transcript.state[1], "state[1] field mismatch");
    }
}

contract Transcript_appendProofEvaluations_Test is Test {
    using T for T.TranscriptData;

    /// @dev Test if `appendProofEvaluations` matches that of Jellyfish
    function testFuzz_appendProofEvaluations_matches(T.TranscriptData memory transcript) external {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "transcript-append-proof-evals";
        cmds[2] = vm.toString(abi.encode(transcript));

        bytes memory result = vm.ffi(cmds);
        (T.TranscriptData memory updated, IPlonkVerifier.PlonkProof memory proof) =
            abi.decode(result, (T.TranscriptData, IPlonkVerifier.PlonkProof));

        transcript.appendProofEvaluations(proof);

        assertEq(updated.transcript, transcript.transcript, "transcript field mismatch");
        assertEq(updated.state[0], transcript.state[0], "state[0] field mismatch");
        assertEq(updated.state[1], transcript.state[1], "state[1] field mismatch");
    }
}
