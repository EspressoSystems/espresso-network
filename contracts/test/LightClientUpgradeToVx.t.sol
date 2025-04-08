// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { LightClient as LCV1 } from "../src/LightClient.sol";
import { LightClientV2Fake as LCV2 } from "./mocks/LightClientV2Fake.sol";
import { LightClientV3Fake as LCV3 } from "./mocks/LightClientV3Fake.sol";
// import { DeployLightClientContractWithoutMultiSigScript as DeployScript } from
//     "../script/LightClient.s.sol";

import { DeployLightClientContractWithoutMultiSigScript as DeployScript } from
    "./script/LightClientTestScript.s.sol";
import { UpgradeLightClientScript as UpgradeScript } from "./script/UpgradeLightClientToV2.s.sol";
import { DowngradeLightClientScript as DowngradeScript } from
    "./script/DowngradeLightClientV2ToV1.s.sol";

import { UpgradeLightClientScript as ULCV3 } from "./script/UpgradeLightClientToV3.s.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { BN254 } from "bn254/BN254.sol";
import { IPlonkVerifier as V } from "../src/interfaces/IPlonkVerifier.sol";
import { Upgrades } from "openzeppelin-foundry-upgrades/Upgrades.sol";
import { ERC1967Utils } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Utils.sol";

contract LightClientUpgradeToVxTest is Test {
    LCV1 public lcV1Proxy;
    LCV2 public lcV2Proxy;
    LCV3 public lcV3Proxy;

    DeployScript public deployer = new DeployScript();
    UpgradeScript public upgraderV2 = new UpgradeScript();
    DowngradeScript public downgrader = new DowngradeScript();
    ULCV3 public upgraderV3 = new ULCV3();

    LCV1.LightClientState public stateV1;
    LCV1.StakeTableState public stakeStateV1;

    address public admin;
    address public proxy;
    address public lcV1Impl;

    uint32 public constant MAX_HISTORY_SECONDS = 864000; //10 days

    // deploy the first implementation with its proxy
    function setUp() public {
        (proxy, admin, stateV1, stakeStateV1) = deployer.run(5, MAX_HISTORY_SECONDS);
        lcV1Proxy = LCV1(proxy);
        lcV1Impl = Upgrades.getImplementationAddress(proxy);
        assertNotEq(lcV1Impl, address(0));
    }

    function testCorrectInitialization() public view {
        (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot) =
            lcV1Proxy.genesisState();
        assertEq(viewNum, stateV1.viewNum);
        assertEq(blockHeight, stateV1.blockHeight);
        assertEq(abi.encode(blockCommRoot), abi.encode(stateV1.blockCommRoot));

        (viewNum, blockHeight, blockCommRoot) = lcV1Proxy.finalizedState();
        assertEq(viewNum, stateV1.viewNum);
        assertEq(blockHeight, stateV1.blockHeight);
        assertEq(abi.encode(blockCommRoot), abi.encode(stateV1.blockCommRoot));
    }

    // test that the data remains the same after upgrading the implementation
    function testUpgradeSameDataV1ToV2() public {
        // Upgrade LightClient and check that the genesis state is not changed and that the new
        // field
        // of the upgraded contract is set to 0
        uint256 myNewField = 123;
        uint256 extraField = 2;
        lcV2Proxy = LCV2(upgraderV2.run(proxy, myNewField, extraField, admin));

        assertEq(lcV2Proxy.newField(), myNewField);

        LCV1.LightClientState memory expectedLightClientState =
            LCV1.LightClientState(stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot);

        LCV2.ExtendedLightClientState memory expectedExtendedLightClientState = LCV2
            .ExtendedLightClientState(
            stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot, extraField
        );

        // compare with the current version of the light client state
        (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot) =
            lcV2Proxy.finalizedState();
        assertEq(viewNum, expectedLightClientState.viewNum);
        assertEq(blockHeight, expectedLightClientState.blockHeight);
        assertEq(abi.encode(blockCommRoot), abi.encode(expectedLightClientState.blockCommRoot));

        // compare with the extended light client state
        (
            uint64 viewNumV2,
            uint64 blockHeightV2,
            BN254.ScalarField blockCommRootV2,
            uint256 extraFieldV2
        ) = lcV2Proxy.extendedFinalizedState();
        assertEq(viewNumV2, expectedExtendedLightClientState.viewNum);
        assertEq(blockHeightV2, expectedExtendedLightClientState.blockHeight);
        assertEq(
            abi.encode(blockCommRootV2), abi.encode(expectedExtendedLightClientState.blockCommRoot)
        );
        assertEq(extraFieldV2, expectedExtendedLightClientState.extraField);
    }

    // test that the data remains the same after upgrading the implementation
    function testRollbackV2toV1() public {
        // Upgrade LightClient and check that the genesis state is not changed and that the new
        // field
        // of the upgraded contract is set to 0
        uint256 myNewField = 123;
        uint256 extraField = 2;
        lcV2Proxy = LCV2(upgraderV2.run(proxy, myNewField, extraField, admin));

        assertEq(lcV2Proxy.newField(), myNewField);

        LCV1.LightClientState memory expectedLightClientState =
            LCV1.LightClientState(stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot);

        LCV2.ExtendedLightClientState memory expectedExtendedLightClientState = LCV2
            .ExtendedLightClientState(
            stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot, extraField
        );

        // compare with the current version of the light client state
        (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot) =
            lcV2Proxy.finalizedState();
        assertEq(viewNum, expectedLightClientState.viewNum);
        assertEq(blockHeight, expectedLightClientState.blockHeight);
        assertEq(abi.encode(blockCommRoot), abi.encode(expectedLightClientState.blockCommRoot));

        // compare with the extended light client state
        (
            uint64 viewNumV2,
            uint64 blockHeightV2,
            BN254.ScalarField blockCommRootV2,
            uint256 extraFieldV2
        ) = lcV2Proxy.extendedFinalizedState();
        assertEq(viewNumV2, expectedExtendedLightClientState.viewNum);
        assertEq(blockHeightV2, expectedExtendedLightClientState.blockHeight);
        assertEq(
            abi.encode(blockCommRootV2), abi.encode(expectedExtendedLightClientState.blockCommRoot)
        );
        assertEq(extraFieldV2, expectedExtendedLightClientState.extraField);

        // Now time to downgrade
        // rollback to lightclient v1 by upgrading v2 to v1
        lcV1Proxy = LCV1(downgrader.run(proxy, admin, lcV1Impl));

        // re-confirm that the proxy address is the same for both versions
        assertEq(address(lcV1Proxy), address(lcV2Proxy));

        // confirm that the implementation address is the same as the first time we deployed, so we
        // know the downgrade worked
        assertEq(address(Upgrades.getImplementationAddress(address(lcV1Proxy))), address(lcV1Impl));

        // ensure that the genesis states are still the same as the original contract
        testCorrectInitialization();
    }

    // test that the data remains the same after upgrading the implementation
    function testExpectRevertRollbackV2toInvalidAddress() public {
        // Upgrade LightClient and check that the genesis state is not changed and that the new
        // field
        // of the upgraded contract is set to 0
        uint256 myNewField = 123;
        uint256 extraField = 2;
        lcV2Proxy = LCV2(upgraderV2.run(proxy, myNewField, extraField, admin));

        assertEq(lcV2Proxy.newField(), myNewField);

        // get current light client implementation address
        address lcV2Impl = Upgrades.getImplementationAddress(address(lcV2Proxy));

        // Now time to downgrade, use invalid lcV1 impl address
        // we expect a revert when you try to downgrade to an invalid address, the proxy address
        vm.expectRevert(
            abi.encodeWithSelector(ERC1967Utils.ERC1967InvalidImplementation.selector, proxy)
        );
        LCV1(downgrader.run(proxy, admin, proxy));

        // re-confirm that the proxy address is the same for both versions
        assertEq(address(lcV1Proxy), address(lcV2Proxy));

        // confirm that the implementation address is the same as the recently deployed light client
        // v2
        // since the downgrade would have reverted
        assertEq(address(Upgrades.getImplementationAddress(address(lcV2Proxy))), lcV2Impl);

        // ensure that the genesis states are still the same as the original contract
        testCorrectInitialization();
    }

    /**
     * TODO:
     * show that downgrading to the wrong light client impl should fail
     */

    // test that the data remains the same after upgrading the implementation
    function testExpectRevertUpgradeSameDataV1ToV2ReinitializeTwice() public {
        // Upgrade LightClient and check that the genesis state is not changed and that the new
        // field
        // of the upgraded contract is set to 0
        uint256 myNewField = 123;
        uint256 extraField = 2;
        lcV2Proxy = LCV2(upgraderV2.run(proxy, myNewField, extraField, admin));

        assertEq(lcV2Proxy.newField(), myNewField);

        LCV1.LightClientState memory expectedLightClientState =
            LCV1.LightClientState(stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot);

        LCV2.ExtendedLightClientState memory expectedExtendedLightClientState = LCV2
            .ExtendedLightClientState(stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot, 2);

        (
            uint64 viewNumV2,
            uint64 blockHeightV2,
            BN254.ScalarField blockCommRootV2,
            uint256 extraFieldV2
        ) = lcV2Proxy.extendedFinalizedState();
        assertEq(viewNumV2, expectedLightClientState.viewNum);
        assertEq(blockHeightV2, expectedLightClientState.blockHeight);
        assertEq(abi.encode(blockCommRootV2), abi.encode(expectedLightClientState.blockCommRoot));

        assertEq(viewNumV2, expectedExtendedLightClientState.viewNum);
        assertEq(blockHeightV2, expectedExtendedLightClientState.blockHeight);
        assertEq(
            abi.encode(blockCommRootV2), abi.encode(expectedExtendedLightClientState.blockCommRoot)
        );
        assertEq(extraFieldV2, expectedExtendedLightClientState.extraField);

        // expect a revert when we try to reinitialize
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        lcV2Proxy.initializeV2(5, 3);
    }

    // test that the data remains the same after upgrading the implementation
    function testUpgradeSameDataV2ToV3() public {
        // Upgrade LightClient and check that the genesis state is not changed and that the new
        // field
        // of the upgraded contract is set to 0
        uint256 myNewField = 123;
        uint256 myNewFieldV3 = 456;
        uint256 extraField = 2;

        lcV2Proxy = LCV2(upgraderV2.run(proxy, myNewField, extraField, admin));

        assertEq(lcV2Proxy.newField(), myNewField);

        LCV1.LightClientState memory expectedLightClientState =
            LCV1.LightClientState(stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot);

        LCV2.ExtendedLightClientState memory expectedExtendedLightClientState = LCV2
            .ExtendedLightClientState(
            stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot, extraField
        );

        // compare with the current version of the light client state
        (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot) =
            lcV2Proxy.finalizedState();
        assertEq(viewNum, expectedLightClientState.viewNum);
        assertEq(blockHeight, expectedLightClientState.blockHeight);
        assertEq(abi.encode(blockCommRoot), abi.encode(expectedLightClientState.blockCommRoot));

        // compare with the extended light client state
        (
            uint64 viewNumV2,
            uint64 blockHeightV2,
            BN254.ScalarField blockCommRootV2,
            uint256 extraFieldV2
        ) = lcV2Proxy.extendedFinalizedState();
        assertEq(viewNumV2, expectedExtendedLightClientState.viewNum);
        assertEq(blockHeightV2, expectedExtendedLightClientState.blockHeight);
        assertEq(
            abi.encode(blockCommRootV2), abi.encode(expectedExtendedLightClientState.blockCommRoot)
        );
        assertEq(extraFieldV2, expectedExtendedLightClientState.extraField);

        // upgrade to v3
        lcV3Proxy = LCV3(upgraderV3.run(proxy, myNewFieldV3, admin));

        assertEq(lcV3Proxy.newField(), myNewField);
        assertEq(lcV3Proxy.anotherField(), myNewFieldV3);

        (viewNum, blockHeight, blockCommRoot) = lcV3Proxy.finalizedState();
        assertEq(viewNum, expectedLightClientState.viewNum);
        assertEq(blockHeight, expectedLightClientState.blockHeight);
        assertEq(abi.encode(blockCommRoot), abi.encode(expectedLightClientState.blockCommRoot));

        // compare with the extended light client state
        (
            uint64 viewNumV3,
            uint64 blockHeightV3,
            BN254.ScalarField blockCommRootV3,
            uint256 extraFieldV3
        ) = lcV2Proxy.extendedFinalizedState();
        assertEq(viewNumV3, expectedExtendedLightClientState.viewNum);
        assertEq(blockHeightV3, expectedExtendedLightClientState.blockHeight);
        assertEq(
            abi.encode(blockCommRootV3), abi.encode(expectedExtendedLightClientState.blockCommRoot)
        );
        assertEq(extraFieldV3, expectedExtendedLightClientState.extraField);
    }

    // test that the tx reverts if we try to reinitialize more than once
    function testExpectRevertUpgradeSameDataV2ToV3ReinitializeTwice() public {
        // Upgrade LightClient and check that the genesis state is not changed and that the new
        // field
        // of the upgraded contract is set to 0
        uint256 myNewField = 123;
        uint256 myNewFieldV3 = 456;
        uint256 extraField = 2;
        // upgrade to v2 first
        lcV2Proxy = LCV2(upgraderV2.run(proxy, myNewField, extraField, admin));

        // upgrade to v3
        lcV3Proxy = LCV3(upgraderV3.run(proxy, myNewFieldV3, admin));

        // expect a revert when we try to reinitialize
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        lcV3Proxy.initializeV3(6);
    }

    // check that the proxy address remains the same
    function testUpgradesSameProxyAddress() public {
        (uint8 major, uint8 minor, uint8 patch) = lcV1Proxy.getVersion();
        assertEq(major, 1);
        assertEq(minor, 0);
        assertEq(patch, 0);

        //upgrade box
        uint256 newField = 123;
        uint256 extraField = 2;
        lcV2Proxy = LCV2(upgraderV2.run(proxy, newField, extraField, admin));
        assertEq(address(lcV2Proxy), address(lcV1Proxy));
        (uint8 majorV2, uint8 minorV2, uint8 patchV2) = lcV2Proxy.getVersion();
        assertEq(majorV2, 2);
        assertEq(minorV2, 0);
        assertEq(patchV2, 0);
    }

    function testMaliciousUpgradeToV2Fails() public {
        address attacker = makeAddr("attacker");

        //attempted upgrade as attacker will revert
        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, attacker)
        );

        uint256 newField = 123;
        uint256 extraField = 2;
        lcV2Proxy = LCV2(upgraderV2.run(address(proxy), newField, extraField, attacker));
    }

    function testMaliciousUpgradeToV32Fails() public {
        address attacker = makeAddr("attacker");

        //attempted upgrade as attacker will revert

        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, attacker)
        );
        lcV3Proxy = LCV3(upgraderV3.run(address(proxy), 456, attacker));
    }
}
