// SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";

/// @dev test top
/// This library implements the verification of the BLS signature scheme over the BN254 curve
/// following
/// the rust implementation at
// solhint-disable-next-line
/// https://github.com/EspressoSystems/jellyfish/blob/e1e683c287f20160738e6e737295dd8f9e70577a/primitives/src/signatures/bls_over_bn254.rs
library BLSSig {
    error BLSSigVerificationFailed();
    error BLSSigIsInfinity();
    error BLSVKIsInfinity();

    // TODO gas optimization
    function _uint256FromBytesLittleEndian(uint8[] memory input) private pure returns (uint256) {
        uint256 r = 0;
        for (uint256 i = 0; i < input.length; i++) {
            r += 2 ** (8 * i) * input[i];
        }
        return r;
    }

    /// @dev Takes a sequence of bytes and turn in into another sequence of bytes with fixed size.
    /// Equivalent of
    // solhint-disable-next-line
    /// https://github.com/arkworks-rs/algebra/blob/1f7b3c6b215e98fa3130b39d2967f6b43df41e04/ff/src/fields/field_hashers/expander/mod.rs#L37
    /// @param message message to be "expanded"
    /// @return fixed size array of bytes
    function expand(bytes memory message) internal pure returns (bytes memory) {
        uint8 blockSize = 48;
        uint256 bLen = 32; // Output length of sha256 in number of bytes
        bytes1 ell = 0x02; // (n+(bLen-1))/bLen where n=48

        // Final value of buffer must be: z_pad || message || lib_str || 0 || dst_prime

        // z_pad
        bytes memory buffer = new bytes(blockSize);

        // message
        buffer = bytes.concat(buffer, message);

        // lib_str
        buffer = bytes.concat(buffer, hex"00", bytes1(blockSize));

        // 0 separator
        buffer = bytes.concat(buffer, hex"00");

        // dst_prime = [1,1]
        bytes2 dstPrime = 0x0101;

        buffer = bytes.concat(buffer, dstPrime);

        bytes32 b0 = keccak256(buffer);

        buffer = bytes.concat(b0, hex"01", dstPrime);

        bytes32 bi = keccak256(buffer);

        // Building uniform_bytes
        bytes memory uniformBytes = new bytes(blockSize);

        // Copy bi into uniform_bytes
        bytes memory biBytes = bytes.concat(bi);
        for (uint256 i = 0; i < biBytes.length; i++) {
            uniformBytes[i] = biBytes[i];
        }

        bytes memory b0Bytes = bytes.concat(b0);

        // In our case ell=2 so we do not have an outer loop
        // solhint-disable-next-line
        // https://github.com/arkworks-rs/algebra/blob/1f7b3c6b215e98fa3130b39d2967f6b43df41e04/ff/src/fields/field_hashers/expander/mod.rs#L100

        buffer = "";
        for (uint256 j = 0; j < bLen; j++) {
            bytes1 v = bytes1(b0Bytes[j] ^ biBytes[j]);
            buffer = bytes.concat(buffer, v);
        }
        buffer = bytes.concat(buffer, ell, dstPrime);

        bi = keccak256(buffer);
        biBytes = bytes.concat(bi);

        for (uint256 i = 0; i < blockSize - bLen; i++) {
            uniformBytes[bLen + i] = biBytes[i];
        }

        return uniformBytes;
    }

    /// @dev Hash a sequence of bytes to a field element in Fq. Equivalent of
    // solhint-disable-next-line
    /// https://github.com/arkworks-rs/algebra/blob/1f7b3c6b215e98fa3130b39d2967f6b43df41e04/ff/src/fields/field_hashers/mod.rs#L65
    /// @param message input message to be hashed
    /// @return field element in Fq
    function hashToField(bytes memory message) internal pure returns (uint256) {
        bytes memory uniformBytes = expand(message);

        // Reverse uniform_bytes
        uint256 n = uniformBytes.length;
        assert(n == 48);
        bytes memory uniformBytesReverted = new bytes(n);

        for (uint256 i = 0; i < n; i++) {
            uniformBytesReverted[i] = uniformBytes[n - i - 1];
        }

        // solhint-disable-next-line
        // https://github.com/arkworks-rs/algebra/blob/bc991d44c5e579025b7ed56df3d30267a7b9acac/ff/src/fields/prime.rs#L72

        // Do the split
        uint256 numBytesDirectlyToConvert = 31; // Fixed for Fq

        // Process the second slice
        uint8[] memory secondSlice = new uint8[](numBytesDirectlyToConvert);

        for (uint256 i = 0; i < numBytesDirectlyToConvert; i++) {
            secondSlice[i] = uint8(uniformBytesReverted[n - numBytesDirectlyToConvert + i]);
        }

        uint256 res = _uint256FromBytesLittleEndian(secondSlice);

        uint256 windowSize = 256;
        uint256 p = BN254.P_MOD;
        // Handle the first slice
        uint256 arrSize = n - numBytesDirectlyToConvert;
        for (uint256 i = 0; i < arrSize; i++) {
            // Compute field element from a single byte
            uint256 fieldElem = uint256(uint8(uniformBytesReverted[arrSize - i - 1])); // In reverse

            res = mulmod(res, windowSize, p);
            res = addmod(res, fieldElem, p);
        }
        return res;
    }

    /// @dev Hash a sequence of bytes to a group element in BN254.G_1. We use the hash-and-pray
    /// algorithm for now.
    /// Rust implementation can be found at
    // solhint-disable-next-line
    /// https://github.com/EspressoSystems/jellyfish/blob/e1e683c287f20160738e6e737295dd8f9e70577a/primitives/src/signatures/bls_over_bn254.rs#L318
    /// @param input message to be hashed
    /// @return group element in G_1
    function hashToCurve(bytes memory input) internal view returns (BN254.G1Point memory) {
        uint256 x = hashToField(input);

        uint256 p = BN254.P_MOD;

        uint256 b = 3;

        // solhint-disable-next-line var-name-mixedcase
        uint256 Y = mulmod(x, x, p);
        Y = mulmod(Y, x, p);
        Y = addmod(Y, b, p);

        // Check Y is a quadratic residue
        BN254.BaseField y;
        bool isQr;
        (isQr, y) = BN254.quadraticResidue(BN254.BaseField.wrap(Y));

        while (!isQr) {
            x = addmod(x, 1, p);
            Y = mulmod(x, x, p);
            Y = mulmod(Y, x, p);
            Y = addmod(Y, b, p);
            (isQr, y) = BN254.quadraticResidue(BN254.BaseField.wrap(Y));
        }

        return BN254.G1Point(BN254.BaseField.wrap(x), y);
    }

    /// @dev Verify a bls signature. Reverts if the signature is invalid
    /// @param message message to check the signature against
    /// @param sig signature represented as a point in BN254.G_1
    /// @param pk public key represented as a point in BN254.G_2
    function verifyBlsSig(bytes memory message, BN254.G1Point memory sig, BN254.G2Point memory pk)
        internal
        view
    {
        // The pairing precompile treats (0,0) G1 and all-zero G2 inputs as the point at
        // infinity, so pk=(0,0,0,0) with sig=(0,0) would satisfy the pairing equation
        // (e(hash, O) * e(O, P2) = 1) without any proof of possession. Honest keys and
        // signatures are never the identity, so reject both explicitly.
        if (BN254.isInfinity(sig)) {
            revert BLSSigIsInfinity();
        }
        if (
            BN254.BaseField.unwrap(pk.x0) == 0 && BN254.BaseField.unwrap(pk.x1) == 0
                && BN254.BaseField.unwrap(pk.y0) == 0 && BN254.BaseField.unwrap(pk.y1) == 0
        ) {
            revert BLSVKIsInfinity();
        }

        // Check the signature is a valid G1 point
        // Note: for non-zero G2 points the pairing precompile checks curve and subgroup
        // membership; a cheaper standalone G2 check is not possible in practice
        // https://ethresear.ch/t/fast-mathbb-g-2-subgroup-check-in-bn254/13974
        BN254.validateG1Point(sig);

        // Hardcoded suffix "BLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_"
        // solhint-disable-next-line
        // https://github.com/EspressoSystems/jellyfish/blob/e1e683c287f20160738e6e737295dd8f9e70577a/primitives/src/constants.rs#L30
        bytes memory csidSuffix = "BLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_";

        bytes memory input = bytes.concat(message, csidSuffix);

        BN254.G1Point memory hash = hashToCurve(input);

        if (!BN254.pairingProd2(hash, pk, BN254.negate(sig), BN254.P2())) {
            revert BLSSigVerificationFailed();
        }
    }
}
