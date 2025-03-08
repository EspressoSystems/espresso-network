// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.0;
pragma experimental ABIEncoderV2;

import { Test } from "forge-std/Test.sol";
import { LightClient as LCV1 } from "../src/LightClient.sol";
import { LightClient as LCV2 } from "../src/LightClient.sol";
import { DeployLightClientContractWithoutMultiSigScript as DeployScript } from
    "./script/LightClientTestScript.s.sol";
import { UpgradeToSameLightClientWithoutMultisigAdminScript as UpgradeScript } from
    "./script/LightClientTestScript.s.sol";
import { BN254 } from "bn254/BN254.sol";

contract LightClientUpgradeSameContractTest is Test {
    LCV1 public lcV1Proxy;
    LCV2 public lcV2Proxy;

    DeployScript public deployer = new DeployScript();
    UpgradeScript public upgrader = new UpgradeScript();

    LCV1.LightClientState public stateV1;
    LCV1.StakeTableState public stakeStateV1;

    address public admin;
    address public proxy;

    uint32 public constant MAX_HISTORY_SECONDS = 864000; //10 days

    // deploy the first implementation with its proxy
    function setUp() public {
        (proxy, admin, stateV1, stakeStateV1) = deployer.run(5, MAX_HISTORY_SECONDS);
        lcV1Proxy = LCV1(proxy);
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

        (
            uint256 threshold,
            BN254.ScalarField stakeTableBlsKeyComm,
            BN254.ScalarField stakeTableSchnorrKeyComm,
            BN254.ScalarField stakeTableAmountComm
        ) = lcV1Proxy.genesisStakeTableState();

        assertEq(
            abi.encode(stakeStateV1),
            abi.encode(
                threshold, stakeTableBlsKeyComm, stakeTableSchnorrKeyComm, stakeTableAmountComm
            )
        );
    }

    // that the data remains the same after upgrading the implementation
    function testUpgradeSameData() public {
        // Upgrade LightClient and check that the genesis state is not changed and that the new
        // field
        // of the upgraded contract is set to 0
        lcV2Proxy = LCV2(upgrader.run(proxy));

        LCV2.LightClientState memory expectedLightClientState =
            LCV2.LightClientState(stateV1.viewNum, stateV1.blockHeight, stateV1.blockCommRoot);

        (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot) =
            lcV1Proxy.finalizedState();
        assertEq(viewNum, expectedLightClientState.viewNum);
        assertEq(blockHeight, expectedLightClientState.blockHeight);
        assertEq(abi.encode(blockCommRoot), abi.encode(expectedLightClientState.blockCommRoot));
    }

    // check that the proxy address remains the same
    function testUpgradesSameProxyAddress() public {
        (uint8 major, uint8 minor, uint8 patch) = lcV1Proxy.getVersion();
        assertEq(major, 1);
        assertEq(minor, 0);
        assertEq(patch, 0);

        //upgrade box
        lcV2Proxy = LCV2(upgrader.run(proxy));
        assertEq(address(lcV2Proxy), address(lcV1Proxy));
    }

    function testMaliciousUpgradeFails() public {
        address attacker = makeAddr("attacker");

        //attempted upgrade as attacker will revert
        vm.prank(attacker);
        vm.expectRevert();
        lcV2Proxy = LCV2(upgrader.run(address(proxy)));
    }
}
