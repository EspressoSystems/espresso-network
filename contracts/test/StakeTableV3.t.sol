// SPDX-License-Identifier: UNLICENSED

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StakeTableV3 } from "../src/StakeTableV3.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTable as S } from "../src/StakeTable.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "../src/libraries/BLSSig.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import {
    PausableUpgradeable
} from "openzeppelin-contracts-upgradeable/contracts/utils/PausableUpgradeable.sol";

contract StakeTableV3Test is Test {
    StakeTableUpgradeV2Test public stakeTableUpgradeTest;
    StakeTableV3 public proxyV3;
    address public pauser;
    address public adminAddr;

    function setUp() public {
        stakeTableUpgradeTest = new StakeTableUpgradeV2Test();
        stakeTableUpgradeTest.setUp();
        pauser = makeAddr("pauser");

        // Upgrade to V2
        vm.startPrank(stakeTableUpgradeTest.admin());
        S baseProxy = stakeTableUpgradeTest.getStakeTable();
        adminAddr = baseProxy.owner();
        StakeTableV2.InitialCommission[] memory emptyCommissions;
        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, adminAddr, 0, emptyCommissions
        );
        baseProxy.upgradeToAndCall(address(new StakeTableV2()), initData);
        StakeTableV2 proxyV2 = StakeTableV2(address(baseProxy));

        // Upgrade to V3
        bytes memory v3InitData = abi.encodeWithSelector(StakeTableV3.initializeV3.selector);
        proxyV2.upgradeToAndCall(address(new StakeTableV3()), v3InitData);
        proxyV3 = StakeTableV3(address(proxyV2));
        vm.stopPrank();
    }

    function registerValidatorV3(
        address validator,
        string memory seed,
        uint16 commission,
        string memory metadataUri,
        bytes32 x25519Key,
        string memory p2pAddr
    ) internal {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, seed);
        bytes memory schnorrSig = new bytes(64);
        vm.prank(validator);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, commission, metadataUri, x25519Key, p2pAddr
        );
    }

    /// Build a bytes32 x25519 key from its little-endian layout: LE byte 0 (least significant)
    /// is `lsb`, LE bytes 1..30 are 0xff, LE byte 31 (holds the top bit) is `msb`. LE byte i
    /// maps to bytes32 index i.
    function leBoundaryKey(uint8 lsb, uint8 msb) internal pure returns (bytes32) {
        uint256 be = (uint256(lsb) << 248) | (((uint256(1) << 240) - 1) << 8) | uint256(msb);
        return bytes32(be);
    }

    // ========== validateP2pAddr ==========

    function test_ValidateP2pAddr_ValidIpv4() public view {
        proxyV3.validateP2pAddr("192.168.1.1:8080");
    }

    function test_ValidateP2pAddr_ValidIpv6() public view {
        proxyV3.validateP2pAddr("::1:8080");
    }

    function test_ValidateP2pAddr_ValidHostname() public view {
        proxyV3.validateP2pAddr("node.example.com:8080");
    }

    function test_ValidateP2pAddr_NoColon() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("localhost");
    }

    function test_ValidateP2pAddr_EmptyHost() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr(":8080");
    }

    function test_ValidateP2pAddr_EmptyPort() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("host:");
    }

    function test_ValidateP2pAddr_PortOverflow() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("host:70000");
    }

    function test_ValidateP2pAddr_PortNonNumeric() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("host:abc");
    }

    function test_ValidateP2pAddr_LeadingZeroPort() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("host:08080");
    }

    function test_ValidateP2pAddr_SingleZeroPort_Reverts() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("host:0");
    }

    function test_ValidateP2pAddr_Empty() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("");
    }

    function test_ValidateP2pAddr_TooLong() public {
        bytes memory buf = new bytes(513);
        for (uint256 i = 0; i < 513; i++) {
            buf[i] = "a";
        }
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr(string(buf));
    }

    function test_ValidateP2pAddr_ExactMaxLength() public view {
        // Build a 512-byte string: 507 chars host + ":" + 4 chars port = 512
        bytes memory buf = new bytes(512);
        for (uint256 i = 0; i < 507; i++) {
            buf[i] = "a";
        }
        buf[507] = ":";
        buf[508] = "8";
        buf[509] = "0";
        buf[510] = "8";
        buf[511] = "0";
        proxyV3.validateP2pAddr(string(buf));
    }

    function test_ValidateP2pAddr_Multiaddr() public {
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.validateP2pAddr("/ip4/1.2.3.4/tcp/4001");
    }

    // ========== registerValidatorV3 ==========

    function test_RegisterValidatorV3_Success() public {
        address validator = makeAddr("validator");
        bytes32 x25519Key = bytes32(uint256(1));
        string memory p2pAddr = "node.example.com:8080";

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        vm.expectEmit();
        emit StakeTableV3.ValidatorRegisteredV3(
            validator, blsVK, schnorrVK, 500, sig, schnorrSig, "meta", x25519Key, p2pAddr
        );
        vm.prank(validator);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, 500, "meta", x25519Key, p2pAddr
        );
    }

    function test_RegisterValidatorV3_ZeroX25519_Reverts() public {
        address validator = makeAddr("validator");

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, 500, "meta", bytes32(0), "host:8080"
        );
    }

    /// All 19 non-canonical values [2^255-19, 2^255-1] are rejected.
    function test_RegisterValidatorV3_NonCanonicalX25519_Reverts() public {
        address validator = makeAddr("validator");

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        for (uint8 k = 0; k < 19; k++) {
            bytes32 key = leBoundaryKey(0xed + k, 0x7f);
            vm.prank(validator);
            vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
            proxyV3.registerValidatorV3(
                blsVK, schnorrVK, sig, schnorrSig, 500, "meta", key, "host:8080"
            );
        }
    }

    /// 2^255-20 (p-1) is the largest canonical value and is accepted.
    function test_RegisterValidatorV3_MaxCanonicalX25519_Success() public {
        registerValidatorV3(
            makeAddr("validator"), "123", 500, "meta", leBoundaryKey(0xec, 0x7f), "host:8080"
        );
    }

    /// LE byte 31 = 0x7f with a non-0xff middle byte stays below p and is accepted.
    function test_RegisterValidatorV3_High7fCanonicalX25519_Success() public {
        registerValidatorV3(
            makeAddr("validator"), "123", 500, "meta", leBoundaryKey(0x00, 0x7f), "host:8080"
        );
    }

    /// Encodings with bit 255 set alias another key and are rejected.
    function test_RegisterValidatorV3_TopBitSetX25519_Reverts() public {
        address validator = makeAddr("validator");

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        // LE value 2^255: only LE byte 31 set to 0x80
        bytes32 topBitOnly = bytes32(uint256(0x80));
        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, 500, "meta", topBitOnly, "host:8080"
        );

        bytes32 allOnes = bytes32(type(uint256).max);
        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, 500, "meta", allOnes, "host:8080"
        );
    }

    /// A typical valid key (all LE bytes 0x07, top bit clear) registers.
    function test_RegisterValidatorV3_TypicalX25519_Success() public {
        bytes32 key =
            bytes32(uint256(0x0707070707070707070707070707070707070707070707070707070707070707));
        registerValidatorV3(makeAddr("validator"), "123", 500, "meta", key, "host:8080");
    }

    function test_RegisterValidatorV3_EmptyP2p_Reverts() public {
        address validator = makeAddr("validator");

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, 500, "meta", bytes32(uint256(1)), ""
        );
    }

    function test_RegisterValidatorV3_LongP2p_Reverts() public {
        address validator = makeAddr("validator");

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        bytes memory buf = new bytes(513);
        for (uint256 i = 0; i < 513; i++) {
            buf[i] = "a";
        }

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, 500, "meta", bytes32(uint256(1)), string(buf)
        );
    }

    function test_RegisterValidatorV3_DuplicateX25519_Reverts() public {
        bytes32 x25519Key = bytes32(uint256(42));
        registerValidatorV3(makeAddr("val1"), "1", 500, "meta", x25519Key, "host1:8080");

        address val2 = makeAddr("val2");
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(val2, "2");
        bytes memory schnorrSig = new bytes(64);

        vm.prank(val2);
        vm.expectRevert(StakeTableV3.X25519KeyAlreadyUsed.selector);
        proxyV3.registerValidatorV3(
            blsVK, schnorrVK, sig, schnorrSig, 500, "meta", x25519Key, "host2:8080"
        );
    }

    /// @dev Regression: the pairing precompile treats all-zero G2 as infinity, so a zero blsVK
    /// with a zero blsSig used to pass BLS verification, allowing registration without proof of
    /// possession.
    function test_RegisterValidatorV3_ZeroBlsVKAndSig_Reverts() public {
        address validator = makeAddr("validator");

        (, EdOnBN254.EdOnBN254Point memory schnorrVK,) =
            stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        BN254.G2Point memory zeroBlsVK = BN254.G2Point(
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0)
        );
        BN254.G1Point memory zeroBlsSig =
            BN254.G1Point(BN254.BaseField.wrap(0), BN254.BaseField.wrap(0));

        vm.prank(validator);
        vm.expectRevert(BLSSig.BLSSigIsInfinity.selector);
        proxyV3.registerValidatorV3(
            zeroBlsVK,
            schnorrVK,
            zeroBlsSig,
            schnorrSig,
            500,
            "meta",
            bytes32(uint256(1)),
            "host:8080"
        );
    }

    /// @dev Same infinity hole via updateConsensusKeysV2 (inherited from V2).
    function test_UpdateConsensusKeysV2_ZeroBlsVKAndSig_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        (, EdOnBN254.EdOnBN254Point memory newSchnorrVK,) =
            stakeTableUpgradeTest.genClientWallet(validator, "45");
        bytes memory schnorrSig = new bytes(64);

        BN254.G2Point memory zeroBlsVK = BN254.G2Point(
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0)
        );
        BN254.G1Point memory zeroBlsSig =
            BN254.G1Point(BN254.BaseField.wrap(0), BN254.BaseField.wrap(0));

        vm.prank(validator);
        vm.expectRevert(BLSSig.BLSSigIsInfinity.selector);
        proxyV3.updateConsensusKeysV2(zeroBlsVK, newSchnorrVK, zeroBlsSig, schnorrSig);
    }

    function test_RegisterValidatorV2_Deprecated_Reverts() public {
        address validator = makeAddr("validator");

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        vm.prank(validator);
        vm.expectRevert(StakeTableV2.DeprecatedFunction.selector);
        proxyV3.registerValidatorV2(blsVK, schnorrVK, sig, schnorrSig, 500, "meta");
    }

    // ========== updateNetworkConfig ==========

    function test_UpdateNetworkConfig_Success() public {
        address validator = makeAddr("validator");
        bytes32 regKey = bytes32(uint256(1));
        registerValidatorV3(validator, "123", 500, "meta", regKey, "host:8080");

        bytes32 newKey = bytes32(uint256(2));
        string memory newAddr = "newhost:9090";

        vm.expectEmit();
        emit StakeTableV3.X25519KeyUpdated(validator, newKey);
        vm.expectEmit();
        emit StakeTableV3.P2pAddrUpdated(validator, newAddr);
        vm.prank(validator);
        proxyV3.updateNetworkConfig(newKey, newAddr);
    }

    function test_UpdateNetworkConfig_Inactive_Reverts() public {
        address nobody = makeAddr("nobody");
        vm.prank(nobody);
        vm.expectRevert(S.ValidatorInactive.selector);
        proxyV3.updateNetworkConfig(bytes32(uint256(1)), "host:8080");
    }

    function test_UpdateNetworkConfig_Exited_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        proxyV3.deregisterValidator();

        vm.prank(validator);
        vm.expectRevert(S.ValidatorAlreadyExited.selector);
        proxyV3.updateNetworkConfig(bytes32(uint256(2)), "host:9090");
    }

    function test_UpdateNetworkConfig_ZeroX25519_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
        proxyV3.updateNetworkConfig(bytes32(0), "host:9090");
    }

    function test_UpdateNetworkConfig_NonCanonicalX25519_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
        proxyV3.updateNetworkConfig(leBoundaryKey(0xed, 0x7f), "host:9090");
    }

    function test_UpdateNetworkConfig_EmptyP2p_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.updateNetworkConfig(bytes32(uint256(2)), "");
    }

    function test_UpdateNetworkConfig_DuplicateX25519_Reverts() public {
        bytes32 key1 = bytes32(uint256(1));
        bytes32 key2 = bytes32(uint256(2));
        registerValidatorV3(makeAddr("val1"), "1", 500, "meta", key1, "host1:8080");
        registerValidatorV3(makeAddr("val2"), "2", 500, "meta", key2, "host2:8080");

        vm.prank(makeAddr("val2"));
        vm.expectRevert(StakeTableV3.X25519KeyAlreadyUsed.selector);
        proxyV3.updateNetworkConfig(key1, "host2:9090");
    }

    function test_UpdateNetworkConfig_Repeated_Success() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        bytes32 key2 = bytes32(uint256(2));
        vm.expectEmit();
        emit StakeTableV3.X25519KeyUpdated(validator, key2);
        vm.expectEmit();
        emit StakeTableV3.P2pAddrUpdated(validator, "host:9090");
        vm.prank(validator);
        proxyV3.updateNetworkConfig(key2, "host:9090");

        bytes32 key3 = bytes32(uint256(3));
        vm.expectEmit();
        emit StakeTableV3.X25519KeyUpdated(validator, key3);
        vm.expectEmit();
        emit StakeTableV3.P2pAddrUpdated(validator, "host:9091");
        vm.prank(validator);
        proxyV3.updateNetworkConfig(key3, "host:9091");
    }

    function test_UpdateNetworkConfig_OwnX25519_Reverts() public {
        bytes32 key = bytes32(uint256(1));
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", key, "host:8080");

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.X25519KeyAlreadyUsed.selector);
        proxyV3.updateNetworkConfig(key, "host:9090");
    }

    function test_UpdateNetworkConfig_Paused_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(pauser);
        proxyV3.pause();

        vm.prank(validator);
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        proxyV3.updateNetworkConfig(bytes32(uint256(2)), "host:9090");
    }

    // ========== updateP2pAddr ==========

    function test_UpdateP2pAddr_Success() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.expectEmit();
        emit StakeTableV3.P2pAddrUpdated(validator, "newhost:9090");
        vm.prank(validator);
        proxyV3.updateP2pAddr("newhost:9090");
    }

    function test_UpdateP2pAddr_Inactive_Reverts() public {
        address nobody = makeAddr("nobody");
        vm.prank(nobody);
        vm.expectRevert(S.ValidatorInactive.selector);
        proxyV3.updateP2pAddr("host:8080");
    }

    function test_UpdateP2pAddr_Exited_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        proxyV3.deregisterValidator();

        vm.prank(validator);
        vm.expectRevert(S.ValidatorAlreadyExited.selector);
        proxyV3.updateP2pAddr("host:9090");
    }

    function test_UpdateP2pAddr_Empty_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.updateP2pAddr("");
    }

    function test_UpdateP2pAddr_Long_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        bytes memory buf = new bytes(513);
        for (uint256 i = 0; i < 513; i++) {
            buf[i] = "a";
        }

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.updateP2pAddr(string(buf));
    }

    function test_UpdateP2pAddr_Paused_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(pauser);
        proxyV3.pause();

        vm.prank(validator);
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        proxyV3.updateP2pAddr("host:9090");
    }

    function test_UpdateP2pAddr_Repeated_Success() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.expectEmit();
        emit StakeTableV3.P2pAddrUpdated(validator, "host1:9090");
        vm.prank(validator);
        proxyV3.updateP2pAddr("host1:9090");

        vm.expectEmit();
        emit StakeTableV3.P2pAddrUpdated(validator, "host2:9091");
        vm.prank(validator);
        proxyV3.updateP2pAddr("host2:9091");
    }

    // ========== updateX25519Key ==========

    function test_UpdateX25519Key_Success() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        bytes32 newKey = bytes32(uint256(2));
        vm.expectEmit();
        emit StakeTableV3.X25519KeyUpdated(validator, newKey);
        vm.prank(validator);
        proxyV3.updateX25519Key(newKey);
    }

    function test_UpdateX25519Key_Inactive_Reverts() public {
        address nobody = makeAddr("nobody");
        vm.prank(nobody);
        vm.expectRevert(S.ValidatorInactive.selector);
        proxyV3.updateX25519Key(bytes32(uint256(1)));
    }

    function test_UpdateX25519Key_Exited_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        proxyV3.deregisterValidator();

        vm.prank(validator);
        vm.expectRevert(S.ValidatorAlreadyExited.selector);
        proxyV3.updateX25519Key(bytes32(uint256(2)));
    }

    function test_UpdateX25519Key_ZeroKey_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
        proxyV3.updateX25519Key(bytes32(0));
    }

    function test_UpdateX25519Key_NonCanonical_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidX25519Key.selector);
        proxyV3.updateX25519Key(leBoundaryKey(0xed, 0x7f));
    }

    function test_UpdateX25519Key_DuplicateKey_Reverts() public {
        bytes32 key = bytes32(uint256(42));
        registerValidatorV3(makeAddr("val1"), "1", 500, "meta", key, "host1:8080");

        address val2 = makeAddr("val2");
        registerValidatorV3(val2, "2", 500, "meta", bytes32(uint256(43)), "host2:8080");

        vm.prank(val2);
        vm.expectRevert(StakeTableV3.X25519KeyAlreadyUsed.selector);
        proxyV3.updateX25519Key(key);
    }

    function test_UpdateX25519Key_Paused_Reverts() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        vm.prank(pauser);
        proxyV3.pause();

        vm.prank(validator);
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        proxyV3.updateX25519Key(bytes32(uint256(2)));
    }

    function test_UpdateNetworkConfig_AtomicRevert() public {
        address validator = makeAddr("validator");
        bytes32 regKey = bytes32(uint256(1));
        registerValidatorV3(validator, "123", 500, "meta", regKey, "host:8080");

        bytes32 newKey = bytes32(uint256(2));

        // Try updateNetworkConfig with valid key but invalid addr (empty string)
        vm.prank(validator);
        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        proxyV3.updateNetworkConfig(newKey, "");

        // Key should NOT be consumed -- can still use it
        vm.expectEmit();
        emit StakeTableV3.X25519KeyUpdated(validator, newKey);
        vm.expectEmit();
        emit StakeTableV3.P2pAddrUpdated(validator, "host:9090");
        vm.prank(validator);
        proxyV3.updateNetworkConfig(newKey, "host:9090");
    }

    function test_UpdateX25519Key_Repeated_Success() public {
        address validator = makeAddr("validator");
        registerValidatorV3(validator, "123", 500, "meta", bytes32(uint256(1)), "host:8080");

        bytes32 key2 = bytes32(uint256(2));
        vm.expectEmit();
        emit StakeTableV3.X25519KeyUpdated(validator, key2);
        vm.prank(validator);
        proxyV3.updateX25519Key(key2);

        bytes32 key3 = bytes32(uint256(3));
        vm.expectEmit();
        emit StakeTableV3.X25519KeyUpdated(validator, key3);
        vm.prank(validator);
        proxyV3.updateX25519Key(key3);
    }
}
