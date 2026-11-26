// SPDX-License-Identifier: UNLICENSED

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test, Vm } from "forge-std/Test.sol";
import { StakeTableV4 } from "../src/StakeTableV4.sol";
import { StakeTableV3 } from "../src/StakeTableV3.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTable as S } from "../src/StakeTable.sol";
import { EspToken } from "../src/EspToken.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "../src/libraries/BLSSig.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import {
    PausableUpgradeable
} from "openzeppelin-contracts-upgradeable/contracts/utils/PausableUpgradeable.sol";

/// @dev The setUp upgrades V1 -> V2 -> V3, registers `legacyValidator` with V3 and delegates to
/// it, then upgrades to V4. So every test runs on V4 with one validator from before the upgrade.
contract StakeTableV4Test is Test {
    StakeTableUpgradeV2Test public stakeTableUpgradeTest;
    StakeTableV3 public proxyV3;
    StakeTableV4 public proxyV4;
    address public pauser;
    address public adminAddr;
    EspToken public token;
    address public tokenGrantRecipient;

    address public legacyValidator;
    address public delegator;
    RegV4 internal legacyReg;

    uint256 public constant DELEGATE_AMOUNT = 2 ether;
    uint256 internal constant SLH_DSA_KEY_LENGTH = 32;
    uint256 internal constant SLH_DSA_SIG_LENGTH = 7856;

    struct RegV4 {
        BN254.G2Point blsVK;
        EdOnBN254.EdOnBN254Point schnorrVK;
        BN254.G1Point blsSig;
        bytes schnorrSig;
        uint16 commission;
        string metadataUri;
        bytes32 x25519Key;
        string p2pAddr;
        bytes slhDsaVK;
        bytes slhDsaSig;
    }

    function setUp() public {
        stakeTableUpgradeTest = new StakeTableUpgradeV2Test();
        stakeTableUpgradeTest.setUp();
        pauser = makeAddr("pauser");
        token = stakeTableUpgradeTest.token();
        tokenGrantRecipient = stakeTableUpgradeTest.tokenGrantRecipient();
        legacyValidator = makeAddr("legacy");
        delegator = makeAddr("delegator");

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

        // State from before the V4 upgrade
        legacyReg = defaultRegV4(legacyValidator, "100");
        vm.prank(legacyValidator);
        proxyV3.registerValidatorV3(
            legacyReg.blsVK,
            legacyReg.schnorrVK,
            legacyReg.blsSig,
            legacyReg.schnorrSig,
            legacyReg.commission,
            legacyReg.metadataUri,
            legacyReg.x25519Key,
            legacyReg.p2pAddr
        );
        fundAndDelegate(delegator, legacyValidator, DELEGATE_AMOUNT);

        // Upgrade to V4
        vm.startPrank(adminAddr);
        bytes memory v4InitData = abi.encodeWithSelector(StakeTableV4.initializeV4.selector);
        proxyV3.upgradeToAndCall(address(new StakeTableV4()), v4InitData);
        proxyV4 = StakeTableV4(address(proxyV3));
        vm.stopPrank();
    }

    // ========== helpers ==========

    function fundAndDelegate(address del, address val, uint256 amount) internal {
        vm.prank(tokenGrantRecipient);
        token.transfer(del, amount);
        vm.startPrank(del);
        token.approve(address(proxyV3), amount);
        proxyV3.delegate(val, amount);
        vm.stopPrank();
    }

    function defaultRegV4(address validator, string memory seed) internal returns (RegV4 memory r) {
        (r.blsVK, r.schnorrVK, r.blsSig) = stakeTableUpgradeTest.genClientWallet(validator, seed);
        r.schnorrSig = new bytes(64);
        r.commission = 500;
        r.metadataUri = "meta";
        r.x25519Key = canonicalX25519Key(keccak256(bytes(seed)));
        r.p2pAddr = "host:8080";
        r.slhDsaVK = slhDsaKey(seed);
        r.slhDsaSig = slhDsaSig(0);
    }

    /// @dev A 32-byte SLH-DSA key derived from `seed`. The contract only checks the length.
    function slhDsaKey(string memory seed) internal pure returns (bytes memory) {
        return abi.encodePacked(keccak256(abi.encodePacked("slh-dsa", seed)));
    }

    /// @dev Signature bytes of the right length, every byte `fill`. Not a valid SLH-DSA
    /// signature: the contract does not check it.
    function slhDsaSig(uint8 fill) internal pure returns (bytes memory sig) {
        sig = new bytes(SLH_DSA_SIG_LENGTH);
        for (uint256 i = 0; i < sig.length; i++) {
            sig[i] = bytes1(fill);
        }
    }

    /// @dev Map a seed to a canonical x25519 key: little-endian value in [1, CURVE25519_P - 1].
    function canonicalX25519Key(bytes32 seed) internal pure returns (bytes32) {
        uint256 v = uint256(seed) % (((uint256(1) << 255) - 19) - 1) + 1;
        uint256 be = 0;
        for (uint256 i = 0; i < 32; i++) {
            be |= uint256(uint8(v >> (8 * i))) << (8 * (31 - i));
        }
        return bytes32(be);
    }

    function registerV4(address validator, RegV4 memory r) internal {
        vm.prank(validator);
        proxyV4.registerValidatorV4(
            r.blsVK,
            r.schnorrVK,
            r.blsSig,
            r.schnorrSig,
            r.commission,
            r.metadataUri,
            r.x25519Key,
            r.p2pAddr,
            r.slhDsaVK,
            r.slhDsaSig
        );
    }

    function updateSlhDsaKey(address validator, bytes memory vk, bytes memory sig) internal {
        vm.prank(validator);
        proxyV4.updateSlhDsaKey(vk, sig);
    }

    // ========== upgrade ==========

    function test_UpgradeV3ToV4_PreservesState() public {
        (uint256 delegatedAmount, S.ValidatorStatus status) = proxyV4.validators(legacyValidator);
        assertEq(uint8(status), uint8(S.ValidatorStatus.Active));
        assertEq(delegatedAmount, DELEGATE_AMOUNT);
        assertEq(proxyV4.delegations(legacyValidator, delegator), DELEGATE_AMOUNT);
        assertTrue(proxyV4.x25519Keys(legacyReg.x25519Key));

        (uint8 major, uint8 minor, uint8 patch) = proxyV4.getVersion();
        assertEq(major, 4);
        assertEq(minor, 0);
        assertEq(patch, 0);

        // V4 registration sees the V3 registration and the V3 x25519 key.
        RegV4 memory again = defaultRegV4(legacyValidator, "101");
        vm.expectRevert(S.ValidatorAlreadyRegistered.selector);
        registerV4(legacyValidator, again);

        address newValidator = makeAddr("new");
        RegV4 memory reuse = defaultRegV4(newValidator, "102");
        reuse.x25519Key = legacyReg.x25519Key;
        vm.expectRevert(StakeTableV3.X25519KeyAlreadyUsed.selector);
        registerV4(newValidator, reuse);
    }

    function test_GetVersion() public view {
        (uint8 major, uint8 minor, uint8 patch) = proxyV4.getVersion();
        assertEq(major, 4);
        assertEq(minor, 0);
        assertEq(patch, 0);
    }

    // ========== registerValidatorV4 ==========

    function test_RegisterValidatorV4_Success() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        assertFalse(proxyV4.slhDsaKeys(keccak256(r.slhDsaVK)));

        vm.expectEmit();
        emit StakeTableV3.ValidatorRegisteredV3(
            validator,
            r.blsVK,
            r.schnorrVK,
            r.commission,
            r.blsSig,
            r.schnorrSig,
            r.metadataUri,
            r.x25519Key,
            r.p2pAddr
        );
        vm.expectEmit();
        emit StakeTableV4.SlhDsaKeyUpdated(validator, r.slhDsaVK, r.slhDsaSig);
        registerV4(validator, r);

        (, S.ValidatorStatus status) = proxyV4.validators(validator);
        assertEq(uint8(status), uint8(S.ValidatorStatus.Active));
        (uint16 commission,) = proxyV4.commissionTracking(validator);
        assertEq(commission, r.commission);
        assertTrue(proxyV4.blsKeys(proxyV4._hashBlsKey(r.blsVK)));
        assertTrue(proxyV4.x25519Keys(r.x25519Key));
        assertTrue(proxyV4.slhDsaKeys(keccak256(r.slhDsaVK)));
    }

    function test_RegisterValidatorV4_EmitsExactlyTwoEvents() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");

        vm.recordLogs();
        registerV4(validator, r);
        Vm.Log[] memory logs = vm.getRecordedLogs();

        assertEq(logs.length, 2);
        assertEq(logs[0].emitter, address(proxyV4));
        assertEq(logs[0].topics[0], StakeTableV3.ValidatorRegisteredV3.selector);
        assertEq(logs[0].topics[1], bytes32(uint256(uint160(validator))));
        assertEq(logs[1].emitter, address(proxyV4));
        assertEq(logs[1].topics[0], StakeTableV4.SlhDsaKeyUpdated.selector);
        assertEq(logs[1].topics[1], bytes32(uint256(uint160(validator))));
        (bytes memory vk, bytes memory sig) = abi.decode(logs[1].data, (bytes, bytes));
        assertEq(vk, r.slhDsaVK);
        assertEq(sig, r.slhDsaSig);
    }

    /// The contract does not check the SLH-DSA signature, only its length.
    function test_RegisterValidatorV4_AnySlhDsaSigBytes_Success() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        r.slhDsaSig = slhDsaSig(0xab);

        vm.expectEmit();
        emit StakeTableV4.SlhDsaKeyUpdated(validator, r.slhDsaVK, r.slhDsaSig);
        registerV4(validator, r);
        assertTrue(proxyV4.slhDsaKeys(keccak256(r.slhDsaVK)));
    }

    function test_RegisterValidatorV4_AlreadyRegistered_Reverts() public {
        address validator = makeAddr("validator");
        registerV4(validator, defaultRegV4(validator, "1"));

        RegV4 memory r = defaultRegV4(validator, "2");
        vm.expectRevert(S.ValidatorAlreadyRegistered.selector);
        registerV4(validator, r);
    }

    function test_RegisterValidatorV4_DuplicateX25519_Reverts() public {
        address val1 = makeAddr("val1");
        RegV4 memory r1 = defaultRegV4(val1, "1");
        registerV4(val1, r1);

        address val2 = makeAddr("val2");
        RegV4 memory r2 = defaultRegV4(val2, "2");
        r2.x25519Key = r1.x25519Key;
        vm.expectRevert(StakeTableV3.X25519KeyAlreadyUsed.selector);
        registerV4(val2, r2);
    }

    function test_RegisterValidatorV4_InvalidP2pAddr_Reverts() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        r.p2pAddr = "localhost";

        vm.expectRevert(StakeTableV3.InvalidP2pAddr.selector);
        registerV4(validator, r);
    }

    function test_RegisterValidatorV4_InvalidBlsSig_Reverts() public {
        RegV4 memory r = defaultRegV4(makeAddr("validator"), "1");

        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        registerV4(makeAddr("other"), r);
    }

    function test_RegisterValidatorV4_SlhDsaKeyWrongLength_Reverts() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");

        uint256[4] memory lengths = [uint256(0), 31, 33, 64];
        for (uint256 i = 0; i < lengths.length; i++) {
            r.slhDsaVK = new bytes(lengths[i]);
            vm.expectRevert(StakeTableV4.InvalidSlhDsaKey.selector);
            registerV4(validator, r);
        }
    }

    function test_RegisterValidatorV4_SlhDsaSigWrongLength_Reverts() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");

        uint256[4] memory lengths = [uint256(0), 64, SLH_DSA_SIG_LENGTH - 1, SLH_DSA_SIG_LENGTH + 1];
        for (uint256 i = 0; i < lengths.length; i++) {
            r.slhDsaSig = new bytes(lengths[i]);
            vm.expectRevert(StakeTableV4.InvalidSlhDsaSig.selector);
            registerV4(validator, r);
        }
    }

    function test_RegisterValidatorV4_SlhDsaKeyOfOtherValidator_Reverts() public {
        address val1 = makeAddr("val1");
        RegV4 memory r1 = defaultRegV4(val1, "1");
        registerV4(val1, r1);

        address val2 = makeAddr("val2");
        RegV4 memory r2 = defaultRegV4(val2, "2");
        r2.slhDsaVK = r1.slhDsaVK;
        vm.expectRevert(StakeTableV4.SlhDsaKeyAlreadyUsed.selector);
        registerV4(val2, r2);
    }

    /// A key set with updateSlhDsaKey is used, too.
    function test_RegisterValidatorV4_SlhDsaKeyFromUpdate_Reverts() public {
        bytes memory key = slhDsaKey("updated");
        updateSlhDsaKey(legacyValidator, key, slhDsaSig(0));

        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        r.slhDsaVK = key;
        vm.expectRevert(StakeTableV4.SlhDsaKeyAlreadyUsed.selector);
        registerV4(validator, r);
    }

    function test_RegisterValidatorV4_Paused_Reverts() public {
        vm.prank(pauser);
        proxyV4.pause();

        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        registerV4(validator, r);

        // The pause is the only reason for the revert: the same call works after unpause.
        vm.prank(pauser);
        proxyV4.unpause();
        vm.expectEmit();
        emit StakeTableV4.SlhDsaKeyUpdated(validator, r.slhDsaVK, r.slhDsaSig);
        registerV4(validator, r);
    }

    function test_RegisterValidatorV3_Deprecated_Reverts() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");

        vm.prank(validator);
        vm.expectRevert(StakeTableV2.DeprecatedFunction.selector);
        proxyV4.registerValidatorV3(
            r.blsVK,
            r.schnorrVK,
            r.blsSig,
            r.schnorrSig,
            r.commission,
            r.metadataUri,
            r.x25519Key,
            r.p2pAddr
        );
    }

    // ========== updateSlhDsaKey ==========

    /// A validator registered with V3 before the upgrade sets its first key.
    function test_UpdateSlhDsaKey_LegacyValidator_Success() public {
        bytes memory key = slhDsaKey("legacy-slh");
        bytes memory sig = slhDsaSig(0xff);
        assertFalse(proxyV4.slhDsaKeys(keccak256(key)));

        vm.expectEmit();
        emit StakeTableV4.SlhDsaKeyUpdated(legacyValidator, key, sig);
        updateSlhDsaKey(legacyValidator, key, sig);

        assertTrue(proxyV4.slhDsaKeys(keccak256(key)));
    }

    function test_UpdateSlhDsaKey_Rotation_Success() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        registerV4(validator, r);

        bytes memory key2 = slhDsaKey("rotated");
        bytes memory sig2 = slhDsaSig(0x02);
        vm.expectEmit();
        emit StakeTableV4.SlhDsaKeyUpdated(validator, key2, sig2);
        updateSlhDsaKey(validator, key2, sig2);

        assertTrue(proxyV4.slhDsaKeys(keccak256(r.slhDsaVK)));
        assertTrue(proxyV4.slhDsaKeys(keccak256(key2)));
    }

    function test_UpdateSlhDsaKey_SameKeyAgain_Reverts() public {
        bytes memory key = slhDsaKey("legacy-slh");
        updateSlhDsaKey(legacyValidator, key, slhDsaSig(0));

        vm.expectRevert(StakeTableV4.SlhDsaKeyAlreadyUsed.selector);
        updateSlhDsaKey(legacyValidator, key, slhDsaSig(0));
    }

    /// After a rotation the first key stays used: neither the same validator nor another one can
    /// set it again.
    function test_UpdateSlhDsaKey_OldKeyAfterRotation_Reverts() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        registerV4(validator, r);
        updateSlhDsaKey(validator, slhDsaKey("rotated"), slhDsaSig(0));

        vm.expectRevert(StakeTableV4.SlhDsaKeyAlreadyUsed.selector);
        updateSlhDsaKey(validator, r.slhDsaVK, slhDsaSig(0));

        vm.expectRevert(StakeTableV4.SlhDsaKeyAlreadyUsed.selector);
        updateSlhDsaKey(legacyValidator, r.slhDsaVK, slhDsaSig(0));
    }

    function test_UpdateSlhDsaKey_KeyOfOtherValidator_Reverts() public {
        address validator = makeAddr("validator");
        RegV4 memory r = defaultRegV4(validator, "1");
        registerV4(validator, r);

        vm.expectRevert(StakeTableV4.SlhDsaKeyAlreadyUsed.selector);
        updateSlhDsaKey(legacyValidator, r.slhDsaVK, slhDsaSig(0));
    }

    function test_UpdateSlhDsaKey_KeyWrongLength_Reverts() public {
        uint256[4] memory lengths = [uint256(0), 31, 33, 64];
        for (uint256 i = 0; i < lengths.length; i++) {
            vm.expectRevert(StakeTableV4.InvalidSlhDsaKey.selector);
            updateSlhDsaKey(legacyValidator, new bytes(lengths[i]), slhDsaSig(0));
        }
    }

    function test_UpdateSlhDsaKey_SigWrongLength_Reverts() public {
        bytes memory key = slhDsaKey("legacy-slh");
        uint256[4] memory lengths = [uint256(0), 64, SLH_DSA_SIG_LENGTH - 1, SLH_DSA_SIG_LENGTH + 1];
        for (uint256 i = 0; i < lengths.length; i++) {
            vm.expectRevert(StakeTableV4.InvalidSlhDsaSig.selector);
            updateSlhDsaKey(legacyValidator, key, new bytes(lengths[i]));
        }
    }

    function test_UpdateSlhDsaKey_Inactive_Reverts() public {
        address nobody = makeAddr("nobody");
        vm.expectRevert(S.ValidatorInactive.selector);
        updateSlhDsaKey(nobody, slhDsaKey("nobody"), slhDsaSig(0));
    }

    function test_UpdateSlhDsaKey_Exited_Reverts() public {
        vm.prank(legacyValidator);
        proxyV4.deregisterValidator();

        vm.expectRevert(S.ValidatorAlreadyExited.selector);
        updateSlhDsaKey(legacyValidator, slhDsaKey("legacy-slh"), slhDsaSig(0));
    }

    function test_UpdateSlhDsaKey_Paused_Reverts() public {
        vm.prank(pauser);
        proxyV4.pause();

        bytes memory key = slhDsaKey("legacy-slh");
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        updateSlhDsaKey(legacyValidator, key, slhDsaSig(0));

        // The pause is the only reason for the revert: the same call works after unpause.
        vm.prank(pauser);
        proxyV4.unpause();
        vm.expectEmit();
        emit StakeTableV4.SlhDsaKeyUpdated(legacyValidator, key, slhDsaSig(0));
        updateSlhDsaKey(legacyValidator, key, slhDsaSig(0));
    }
}
