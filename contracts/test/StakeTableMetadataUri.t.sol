// SPDX-License-Identifier: UNLICENSED

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { PausableUpgradeable } from
    "openzeppelin-contracts-upgradeable/contracts/utils/PausableUpgradeable.sol";
import { StakeTable as S } from "../src/StakeTable.sol";

contract StakeTableMetadataUriTest is Test {
    StakeTableUpgradeV2Test public stakeTableUpgradeTest;
    StakeTableV2 public proxy;
    address public pauser;

    function setUp() public {
        stakeTableUpgradeTest = new StakeTableUpgradeV2Test();
        stakeTableUpgradeTest.setUp();
        pauser = makeAddr("pauser");

        vm.startPrank(stakeTableUpgradeTest.admin());
        S baseProxy = stakeTableUpgradeTest.getStakeTable();
        address admin = baseProxy.owner();
        StakeTableV2.InitialCommission[] memory emptyCommissions;
        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, admin, 0, emptyCommissions
        );
        baseProxy.upgradeToAndCall(address(new StakeTableV2()), initData);
        proxy = StakeTableV2(address(baseProxy));
        vm.stopPrank();
    }

    function registerValidatorWithMetadataUri(
        address validator,
        string memory seed,
        uint16 commission,
        string memory metadataUri
    ) internal {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, seed);
        bytes memory schnorrSig = new bytes(64);

        vm.startPrank(validator);
        vm.expectEmit();
        emit StakeTableV2.ValidatorRegisteredV2(
            validator, blsVK, schnorrVK, commission, sig, schnorrSig, metadataUri
        );
        proxy.registerValidatorV2(blsVK, schnorrVK, sig, schnorrSig, commission, metadataUri);
        vm.stopPrank();
    }

    function test_RegisterValidator_WithMetadataUri() public {
        address validator = makeAddr("validator");
        string memory metadataUri = "dummy-meta";
        uint16 commission = 500;

        registerValidatorWithMetadataUri(validator, "123", commission, metadataUri);
    }

    function test_RegisterValidator_WithEmptyMetadataUri_Reverts() public {
        address validator = makeAddr("validator");
        string memory metadataUri = "";
        uint16 commission = 500;

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        vm.startPrank(validator);
        vm.expectRevert(StakeTableV2.InvalidMetadataUriLength.selector);
        proxy.registerValidatorV2(blsVK, schnorrVK, sig, schnorrSig, commission, metadataUri);
        vm.stopPrank();
    }

    function test_UpdateMetadataUri_Success() public {
        address validator = makeAddr("validator");
        string memory initialUri = "dummy-meta";
        string memory newUri = "dummy-meta-2";

        registerValidatorWithMetadataUri(validator, "123", 500, initialUri);

        vm.startPrank(validator);
        vm.expectEmit();
        emit StakeTableV2.MetadataUriUpdated(validator, newUri);
        proxy.updateMetadataUri(newUri);
        vm.stopPrank();
    }

    function test_UpdateMetadataUri_ToEmptyString_Reverts() public {
        address validator = makeAddr("validator");
        string memory initialUri = "dummy-meta";

        registerValidatorWithMetadataUri(validator, "123", 500, initialUri);

        vm.startPrank(validator);
        vm.expectRevert(StakeTableV2.InvalidMetadataUriLength.selector);
        proxy.updateMetadataUri("");
        vm.stopPrank();
    }

    function test_UpdateMetadataUri_MultipleTimes() public {
        address validator = makeAddr("validator");
        string memory initialUri = "dummy-meta";
        string memory secondUri = "dummy-meta-2";
        string memory thirdUri = "dummy-meta-3";

        registerValidatorWithMetadataUri(validator, "123", 500, initialUri);

        vm.startPrank(validator);

        vm.expectEmit();
        emit StakeTableV2.MetadataUriUpdated(validator, secondUri);
        proxy.updateMetadataUri(secondUri);

        vm.expectEmit();
        emit StakeTableV2.MetadataUriUpdated(validator, thirdUri);
        proxy.updateMetadataUri(thirdUri);

        vm.stopPrank();
    }

    function test_UpdateMetadataUri_RevertWhenNotValidator() public {
        address validator = makeAddr("validator");
        address notValidator = makeAddr("notValidator");
        string memory initialUri = "dummy-meta";

        registerValidatorWithMetadataUri(validator, "123", 500, initialUri);

        vm.startPrank(notValidator);
        vm.expectRevert(S.ValidatorInactive.selector);
        proxy.updateMetadataUri("dummy-meta-2");
        vm.stopPrank();
    }

    function test_UpdateMetadataUri_RevertWhenValidatorExited() public {
        address validator = makeAddr("validator");
        string memory initialUri = "dummy-meta";

        registerValidatorWithMetadataUri(validator, "123", 500, initialUri);

        vm.startPrank(validator);
        proxy.deregisterValidator();

        vm.expectRevert(S.ValidatorAlreadyExited.selector);
        proxy.updateMetadataUri("dummy-meta-2");
        vm.stopPrank();
    }

    function test_UpdateMetadataUri_RevertWhenPaused() public {
        address validator = makeAddr("validator");
        string memory initialUri = "dummy-meta";

        registerValidatorWithMetadataUri(validator, "123", 500, initialUri);

        vm.prank(pauser);
        proxy.pause();

        vm.startPrank(validator);
        vm.expectRevert(PausableUpgradeable.EnforcedPause.selector);
        proxy.updateMetadataUri("dummy-meta-2");
        vm.stopPrank();
    }

    function test_RegisterValidator_WithLongMetadataUri() public {
        address validator = makeAddr("");
        string memory longUri = new string(2048);

        registerValidatorWithMetadataUri(validator, "123", 500, longUri);
    }

    function test_RegisterValidator_WithTooLongMetadataUri_Reverts() public {
        address validator = makeAddr("");
        uint16 commission = 500;

        string memory tooLongUri = new string(2049);

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "123");
        bytes memory schnorrSig = new bytes(64);

        vm.startPrank(validator);
        vm.expectRevert(StakeTableV2.InvalidMetadataUriLength.selector);
        proxy.registerValidatorV2(blsVK, schnorrVK, sig, schnorrSig, commission, tooLongUri);
        vm.stopPrank();
    }

    function test_UpdateMetadataUri_WithTooLongUri_Reverts() public {
        address validator = makeAddr("");
        string memory initialUri = "dummy-meta";

        registerValidatorWithMetadataUri(validator, "123", 500, initialUri);

        string memory tooLongUri = new string(2049);

        vm.startPrank(validator);
        vm.expectRevert(StakeTableV2.InvalidMetadataUriLength.selector);
        proxy.updateMetadataUri(tooLongUri);
        vm.stopPrank();
    }
}
