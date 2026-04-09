// SPDX-License-Identifier: UNLICENSED

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StakeTableV3 } from "../src/StakeTableV3.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTable as S } from "../src/StakeTable.sol";
import { EspToken } from "../src/EspToken.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";

contract StakeTableUpgradeToV3Test is Test {
    StakeTableUpgradeV2Test public stakeTableUpgradeTest;

    // Proxy handles at each version level
    S public baseProxy;
    StakeTableV2 public proxyV2;
    StakeTableV3 public proxyV3;

    address public adminAddr;
    address public pauser;
    EspToken public token;
    address public tokenGrantRecipient;

    // Test actors
    address public validator;
    address public delegator;

    uint256 public constant DELEGATE_AMOUNT = 2 ether;

    function setUp() public {
        stakeTableUpgradeTest = new StakeTableUpgradeV2Test();
        stakeTableUpgradeTest.setUp();

        baseProxy = stakeTableUpgradeTest.getStakeTable();
        adminAddr = baseProxy.owner();
        pauser = makeAddr("pauser");
        token = stakeTableUpgradeTest.token();
        tokenGrantRecipient = stakeTableUpgradeTest.tokenGrantRecipient();
        validator = makeAddr("validator");
        delegator = makeAddr("delegator");

        // Upgrade V1 -> V2
        vm.startPrank(adminAddr);
        StakeTableV2.InitialCommission[] memory emptyCommissions;
        bytes memory v2InitData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, adminAddr, 0, emptyCommissions
        );
        baseProxy.upgradeToAndCall(address(new StakeTableV2()), v2InitData);
        proxyV2 = StakeTableV2(address(baseProxy));
        vm.stopPrank();
    }

    // --- Helpers ---

    function _registerValidatorV2(address val, string memory seed) internal {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(val, seed);
        bytes memory schnorrSig = new bytes(64);
        vm.prank(val);
        proxyV2.registerValidatorV2(blsVK, schnorrVK, sig, schnorrSig, 500, "meta");
    }

    function _fundAndDelegate(address del, address val, uint256 amount) internal {
        vm.prank(tokenGrantRecipient);
        token.transfer(del, amount);
        vm.startPrank(del);
        token.approve(address(baseProxy), amount);
        proxyV2.delegate(val, amount);
        vm.stopPrank();
    }

    function _upgradeToV3() internal {
        vm.startPrank(adminAddr);
        bytes memory v3InitData = abi.encodeWithSelector(StakeTableV3.initializeV3.selector);
        proxyV2.upgradeToAndCall(address(new StakeTableV3()), v3InitData);
        proxyV3 = StakeTableV3(address(proxyV2));
        vm.stopPrank();
    }

    // --- Tests ---

    function test_UpgradeV2ToV3_PreservesState() public {
        // Set up state in V2
        _registerValidatorV2(validator, "1");
        _fundAndDelegate(delegator, validator, DELEGATE_AMOUNT);

        // Upgrade
        _upgradeToV3();

        // Validator still exists and is active
        (uint256 delegatedAmount, S.ValidatorStatus status) = proxyV3.validators(validator);
        assertEq(uint8(status), uint8(S.ValidatorStatus.Active));

        // Delegation amount preserved
        assertEq(delegatedAmount, DELEGATE_AMOUNT);
        assertEq(proxyV3.delegations(validator, delegator), DELEGATE_AMOUNT);

        // Version is (3,0,0)
        (uint8 major, uint8 minor, uint8 patch) = proxyV3.getVersion();
        assertEq(major, 3);
        assertEq(minor, 0);
        assertEq(patch, 0);

        // V2 functions still work: delegate more
        uint256 extraAmount = 1 ether;
        vm.prank(tokenGrantRecipient);
        token.transfer(delegator, extraAmount);
        vm.startPrank(delegator);
        token.approve(address(baseProxy), extraAmount);
        proxyV3.delegate(validator, extraAmount);
        vm.stopPrank();
        assertEq(proxyV3.delegations(validator, delegator), DELEGATE_AMOUNT + extraAmount);

        // V2 functions still work: undelegate (full extra amount so no dust)
        vm.prank(delegator);
        proxyV3.undelegate(validator, extraAmount);

        // V2 functions still work: updateConsensusKeysV2
        (
            BN254.G2Point memory newBlsVK,
            EdOnBN254.EdOnBN254Point memory newSchnorrVK,
            BN254.G1Point memory newSig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "2");
        bytes memory newSchnorrSig = new bytes(64);
        vm.prank(validator);
        proxyV3.updateConsensusKeysV2(newBlsVK, newSchnorrVK, newSig, newSchnorrSig);
    }

    function test_UpgradeV2ToV3_ReinitializeReverts() public {
        _upgradeToV3();

        vm.prank(adminAddr);
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        proxyV3.initializeV3();
    }

    /// forge-config: default.allow_internal_expect_revert = true
    function test_UpgradeV2ToV3_UnauthorizedReverts() public {
        address notAdmin = makeAddr("notAdmin");

        address v3Impl = address(new StakeTableV3());
        bytes memory v3InitData = abi.encodeWithSelector(StakeTableV3.initializeV3.selector);

        bytes32 adminRole = proxyV2.DEFAULT_ADMIN_ROLE();
        vm.prank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, notAdmin, adminRole
            )
        );
        proxyV2.upgradeToAndCall(v3Impl, v3InitData);
    }

    function test_UpgradeV2ToV3_V2OpsAfterUpgrade() public {
        _registerValidatorV2(validator, "1");
        _fundAndDelegate(delegator, validator, DELEGATE_AMOUNT);
        _upgradeToV3();

        // delegate works
        uint256 moreAmount = 1 ether;
        vm.prank(tokenGrantRecipient);
        token.transfer(delegator, moreAmount);
        vm.startPrank(delegator);
        token.approve(address(baseProxy), moreAmount);
        proxyV3.delegate(validator, moreAmount);
        vm.stopPrank();
        assertEq(proxyV3.delegations(validator, delegator), DELEGATE_AMOUNT + moreAmount);

        // undelegate works
        uint256 undelegateAmount = 1 ether;
        vm.prank(delegator);
        proxyV3.undelegate(validator, undelegateAmount);
        (uint256 undelegationAmt, uint256 unlocksAt) = proxyV3.undelegations(validator, delegator);
        assertEq(undelegationAmt, undelegateAmount);
        assertGt(unlocksAt, block.timestamp);

        // claimWithdrawal works after escrow period
        vm.warp(unlocksAt + 1);
        uint256 balBefore = token.balanceOf(delegator);
        vm.prank(delegator);
        proxyV3.claimWithdrawal(validator);
        assertEq(token.balanceOf(delegator), balBefore + undelegateAmount);

        // updateConsensusKeysV2 works
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = stakeTableUpgradeTest.genClientWallet(validator, "2");
        bytes memory schnorrSig = new bytes(64);
        vm.prank(validator);
        proxyV3.updateConsensusKeysV2(blsVK, schnorrVK, sig, schnorrSig);
    }

    function test_UpgradeV2ToV3_PendingUndelegationPreserved() public {
        _registerValidatorV2(validator, "1");
        _fundAndDelegate(delegator, validator, DELEGATE_AMOUNT);

        // Create undelegation in V2
        uint256 undelegateAmount = 400;
        vm.prank(delegator);
        proxyV2.undelegate(validator, undelegateAmount);

        (uint256 amtBefore, uint256 unlocksAtBefore) = proxyV2.undelegations(validator, delegator);
        assertEq(amtBefore, undelegateAmount);

        // Upgrade to V3
        _upgradeToV3();

        // Undelegation preserved
        (uint256 amtAfter, uint256 unlocksAtAfter) = proxyV3.undelegations(validator, delegator);
        assertEq(amtAfter, undelegateAmount);
        assertEq(unlocksAtAfter, unlocksAtBefore);

        // Claim in V3 after escrow period
        vm.warp(unlocksAtAfter + 1);
        uint256 balBefore = token.balanceOf(delegator);
        vm.prank(delegator);
        proxyV3.claimWithdrawal(validator);
        assertEq(token.balanceOf(delegator), balBefore + undelegateAmount);
    }

    function test_UpgradeV2ToV3_ExitedValidatorPreserved() public {
        _registerValidatorV2(validator, "1");
        _fundAndDelegate(delegator, validator, DELEGATE_AMOUNT);

        // Exit validator in V2
        vm.prank(validator);
        proxyV2.deregisterValidator();

        (, S.ValidatorStatus statusBefore) = proxyV2.validators(validator);
        assertEq(uint8(statusBefore), uint8(S.ValidatorStatus.Exited));

        // Upgrade to V3
        _upgradeToV3();

        // Validator still exited
        (, S.ValidatorStatus statusAfter) = proxyV3.validators(validator);
        assertEq(uint8(statusAfter), uint8(S.ValidatorStatus.Exited));

        // Claim exit funds after escrow period
        vm.warp(block.timestamp + proxyV3.exitEscrowPeriod() + 1);
        uint256 balBefore = token.balanceOf(delegator);
        vm.prank(delegator);
        proxyV3.claimValidatorExit(validator);
        assertEq(token.balanceOf(delegator), balBefore + DELEGATE_AMOUNT);
    }
}
