// SPDX-License-Identifier: MIT

/* solhint-disable contract-name-camelcase, func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTable as S } from "../src/StakeTable.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { StakeTable_register_Test } from "./StakeTable.t.sol";

/// @title StakeTableV2 Governance Tests
/// @notice Comprehensive tests for governance functions: transferOwnership, grantRole, revokeRole
/// @dev Tests the single-admin governance model where owner() and DEFAULT_ADMIN_ROLE are
/// synchronized
contract StakeTableV2GovernanceTest is Test {
    StakeTableUpgradeV2Test public baseTest;
    StakeTableV2 public proxy;
    address public initialOwner;
    address public originalV1Owner;
    address public pauser;

    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);

    function setUp() public {
        baseTest = new StakeTableUpgradeV2Test();
        baseTest.setUp();
        pauser = makeAddr("pauser");
        StakeTable_register_Test stakeTableRegisterTest = baseTest.stakeTableRegisterTest();

        originalV1Owner = baseTest.admin();
        initialOwner = makeAddr("v2Admin");

        vm.startPrank(originalV1Owner);
        S baseProxy = S(address(stakeTableRegisterTest.stakeTable()));
        StakeTableV2.InitialCommission[] memory emptyCommissions;
        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, pauser, initialOwner, 0, emptyCommissions
        );
        baseProxy.upgradeToAndCall(address(new StakeTableV2()), initData);
        proxy = StakeTableV2(address(baseProxy));
        vm.stopPrank();

        checkGovernanceInvariants();
    }

    // ============================================
    // Invariant Helpers
    // ============================================

    /// @notice Check all critical governance invariants
    /// @dev Call this after any state-changing operation to ensure invariants hold
    function _checkGovernanceInvariants() internal view {
        address currentOwner = proxy.owner();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        require(currentOwner != address(0), "owner() must not be zero address");

        require(proxy.hasRole(adminRole, currentOwner), "owner() must have DEFAULT_ADMIN_ROLE");

        (uint8 majorVersion,,) = proxy.getVersion();
        require(majorVersion == 2, "Contract must be V2");
    }

    function checkGovernanceInvariants() public view {
        _checkGovernanceInvariants();
    }

    // ============================================
    // transferOwnership Tests
    // ============================================

    function test_Setup_VerifiesOwnershipTransferredDuringUpgrade() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertEq(proxy.owner(), initialOwner, "V2 admin should be owner");
        assertTrue(
            proxy.hasRole(adminRole, initialOwner), "V2 admin should have DEFAULT_ADMIN_ROLE"
        );
        assertFalse(
            proxy.hasRole(adminRole, originalV1Owner),
            "Original V1 owner should NOT have DEFAULT_ADMIN_ROLE"
        );

        vm.startPrank(originalV1Owner);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, originalV1Owner, adminRole
            )
        );
        proxy.updateExitEscrowPeriod(200 seconds);
        vm.stopPrank();

        vm.prank(initialOwner);
        proxy.updateExitEscrowPeriod(200 seconds);
    }

    function test_TransferOwnership_Success() public {
        address newOwner = makeAddr("newOwner");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertEq(proxy.owner(), initialOwner, "Initial owner should be set");
        assertTrue(
            proxy.hasRole(adminRole, initialOwner), "Initial owner should have DEFAULT_ADMIN_ROLE"
        );
        assertFalse(proxy.hasRole(adminRole, newOwner), "New owner should not have role yet");

        vm.startPrank(initialOwner);

        vm.expectEmit(true, true, true, true, address(proxy));
        emit RoleGranted(adminRole, newOwner, initialOwner);

        vm.expectEmit(true, true, false, true, address(proxy));
        emit OwnershipTransferred(initialOwner, newOwner);

        vm.expectEmit(true, true, true, true, address(proxy));
        emit RoleRevoked(adminRole, initialOwner, initialOwner);

        proxy.transferOwnership(newOwner);

        vm.stopPrank();

        assertEq(proxy.owner(), newOwner, "Ownership should be transferred");
        assertTrue(proxy.hasRole(adminRole, newOwner), "New owner should have DEFAULT_ADMIN_ROLE");
        assertFalse(
            proxy.hasRole(adminRole, initialOwner), "Old owner should NOT have DEFAULT_ADMIN_ROLE"
        );
    }

    function test_TransferOwnership_ToSameOwner() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertEq(proxy.owner(), initialOwner, "Initial owner should be set");
        assertTrue(proxy.hasRole(adminRole, initialOwner), "Initial owner should have role");

        vm.startPrank(initialOwner);

        vm.expectEmit(true, true, false, true, address(proxy));
        emit OwnershipTransferred(initialOwner, initialOwner);

        proxy.transferOwnership(initialOwner);

        vm.stopPrank();

        assertEq(proxy.owner(), initialOwner, "Owner should remain the same");
        assertTrue(
            proxy.hasRole(adminRole, initialOwner), "Owner should still have role (not revoked!)"
        );
    }

    function test_TransferOwnership_ToSameOwner_MultipleTimes() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);

        for (uint256 i = 0; i < 5; i++) {
            proxy.transferOwnership(initialOwner);

            assertEq(proxy.owner(), initialOwner, "Owner should remain the same");
            assertTrue(
                proxy.hasRole(adminRole, initialOwner),
                "Owner should still have role after multiple self-transfers"
            );
        }

        vm.stopPrank();

        vm.prank(initialOwner);
        proxy.updateExitEscrowPeriod(200 seconds);
    }

    function test_TransferOwnership_RevertsWhenNotAdmin() public {
        address notAdmin = makeAddr("notAdmin");
        address newOwner = makeAddr("newOwner");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, notAdmin, adminRole
            )
        );
        proxy.transferOwnership(newOwner);
        vm.stopPrank();
    }

    function test_TransferOwnership_NewOwnerCanPerformAdminActions() public {
        address newOwner = makeAddr("newOwner");

        vm.prank(initialOwner);
        proxy.transferOwnership(newOwner);

        vm.startPrank(newOwner);

        uint64 newPeriod = 200 seconds;
        proxy.updateExitEscrowPeriod(newPeriod);

        proxy.grantRole(proxy.PAUSER_ROLE(), newOwner);
        proxy.pause();

        vm.stopPrank();

        assertTrue(proxy.paused(), "Contract should be paused");
    }

    function test_TransferOwnership_OldOwnerCannotPerformAdminActions() public {
        address newOwner = makeAddr("newOwner");

        vm.prank(initialOwner);
        proxy.transferOwnership(newOwner);

        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, initialOwner, adminRole
            )
        );
        proxy.updateExitEscrowPeriod(200 seconds);
        vm.stopPrank();
    }

    // ============================================
    // grantRole Tests
    // ============================================

    function test_GrantRole_NonAdminRole_Success() public {
        address newPauser = makeAddr("newPauser");
        bytes32 pauserRole = proxy.PAUSER_ROLE();

        vm.startPrank(initialOwner);

        vm.expectEmit(true, true, true, true, address(proxy));
        emit RoleGranted(pauserRole, newPauser, initialOwner);

        proxy.grantRole(pauserRole, newPauser);

        vm.stopPrank();

        assertTrue(proxy.hasRole(pauserRole, newPauser), "New pauser should have PAUSER_ROLE");
        assertEq(proxy.owner(), initialOwner, "Owner should not change");
    }

    function test_GrantRole_AdminRole_TransfersOwnership() public {
        address newAdmin = makeAddr("newAdmin");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);

        vm.expectEmit(true, true, true, true, address(proxy));
        emit RoleGranted(adminRole, newAdmin, initialOwner);

        vm.expectEmit(true, true, false, true, address(proxy));
        emit OwnershipTransferred(initialOwner, newAdmin);

        vm.expectEmit(true, true, true, true, address(proxy));
        emit RoleRevoked(adminRole, initialOwner, initialOwner);

        proxy.grantRole(adminRole, newAdmin);

        vm.stopPrank();

        assertEq(proxy.owner(), newAdmin, "Ownership should be transferred");
        assertTrue(proxy.hasRole(adminRole, newAdmin), "New admin should have DEFAULT_ADMIN_ROLE");
        assertFalse(
            proxy.hasRole(adminRole, initialOwner), "Old admin should NOT have DEFAULT_ADMIN_ROLE"
        );
    }

    function test_GrantRole_AdminRole_ToCurrentOwner() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);

        proxy.grantRole(adminRole, initialOwner);

        vm.stopPrank();

        assertEq(proxy.owner(), initialOwner, "Owner should remain the same");
        assertTrue(proxy.hasRole(adminRole, initialOwner), "Owner should have role");
    }

    function test_GrantRole_RevertsWhenNotAdmin() public {
        address notAdmin = makeAddr("notAdmin");
        address recipient = makeAddr("recipient");
        bytes32 pauserRole = proxy.PAUSER_ROLE();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, notAdmin, adminRole
            )
        );
        proxy.grantRole(pauserRole, recipient);
        vm.stopPrank();
    }

    function test_GrantRole_AdminRole_EnforcesSingleAdmin() public {
        address admin2 = makeAddr("admin2");
        address admin3 = makeAddr("admin3");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.prank(initialOwner);
        proxy.grantRole(adminRole, admin2);

        assertTrue(proxy.hasRole(adminRole, admin2), "Admin2 should have role");
        assertFalse(proxy.hasRole(adminRole, initialOwner), "InitialOwner should NOT have role");

        vm.prank(admin2);
        proxy.grantRole(adminRole, admin3);

        assertTrue(proxy.hasRole(adminRole, admin3), "Admin3 should have role");
        assertFalse(proxy.hasRole(adminRole, admin2), "Admin2 should NOT have role");
        assertFalse(proxy.hasRole(adminRole, initialOwner), "InitialOwner should NOT have role");
        assertEq(proxy.owner(), admin3, "Admin3 should be owner");
    }

    // ============================================
    // Governance Integration Tests
    // ============================================

    function test_Governance_ChainOfTransfers() public {
        address[] memory admins = new address[](5);
        for (uint256 i = 0; i < 5; i++) {
            admins[i] = makeAddr(string(abi.encodePacked("admin", i)));
        }

        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        address currentAdmin = initialOwner;

        for (uint256 i = 0; i < 5; i++) {
            vm.prank(currentAdmin);
            proxy.transferOwnership(admins[i]);

            assertEq(proxy.owner(), admins[i], "Owner should be transferred");
            assertTrue(proxy.hasRole(adminRole, admins[i]), "New admin should have role");
            assertFalse(
                proxy.hasRole(adminRole, currentAdmin), "Previous admin should NOT have role"
            );

            currentAdmin = admins[i];
        }

        for (uint256 i = 0; i < 4; i++) {
            assertFalse(proxy.hasRole(adminRole, admins[i]), "Previous admins should not have role");
        }
        assertTrue(proxy.hasRole(adminRole, admins[4]), "Final admin should have role");
        assertFalse(proxy.hasRole(adminRole, initialOwner), "Initial owner should not have role");
    }

    function test_Governance_GrantRoleVsTransferOwnership_BothWork() public {
        address newAdmin1 = makeAddr("newAdmin1");
        address newAdmin2 = makeAddr("newAdmin2");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.prank(initialOwner);
        proxy.transferOwnership(newAdmin1);

        assertEq(proxy.owner(), newAdmin1, "Owner should be newAdmin1");
        assertTrue(proxy.hasRole(adminRole, newAdmin1), "newAdmin1 should have role");

        vm.prank(newAdmin1);
        proxy.grantRole(adminRole, newAdmin2);

        assertEq(proxy.owner(), newAdmin2, "Owner should be newAdmin2");
        assertTrue(proxy.hasRole(adminRole, newAdmin2), "newAdmin2 should have role");
        assertFalse(proxy.hasRole(adminRole, newAdmin1), "newAdmin1 should NOT have role");
    }

    function test_Governance_AdminCanUpgradeContract() public {
        address newAdmin = makeAddr("newAdmin");

        vm.prank(initialOwner);
        proxy.transferOwnership(newAdmin);

        vm.startPrank(newAdmin);
        StakeTableV2 newImpl = new StakeTableV2();
        proxy.upgradeToAndCall(address(newImpl), "");
        vm.stopPrank();

        (uint8 majorVersion,,) = proxy.getVersion();
        assertEq(majorVersion, 2, "Version should still be 2");
    }

    function test_Governance_OnlyAdminCanGrantPauserRole() public {
        address newPauser = makeAddr("newPauser");
        address notAdmin = makeAddr("notAdmin");
        bytes32 pauserRole = proxy.PAUSER_ROLE();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, notAdmin, adminRole
            )
        );
        proxy.grantRole(pauserRole, newPauser);
        vm.stopPrank();

        vm.prank(initialOwner);
        proxy.grantRole(pauserRole, newPauser);

        assertTrue(proxy.hasRole(pauserRole, newPauser), "New pauser should have role");
    }

    // ============================================
    // Edge Cases and Security Tests
    // ============================================

    function test_Security_CannotGrantAdminToZeroAddress() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);

        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableInvalidOwner.selector, address(0))
        );
        proxy.grantRole(adminRole, address(0));

        vm.stopPrank();
    }

    function test_Security_MultipleAdminsNeverExist() public {
        address newAdmin = makeAddr("newAdmin");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertTrue(proxy.hasRole(adminRole, initialOwner));
        assertFalse(proxy.hasRole(adminRole, newAdmin));

        vm.prank(initialOwner);
        proxy.transferOwnership(newAdmin);

        assertFalse(proxy.hasRole(adminRole, initialOwner));
        assertTrue(proxy.hasRole(adminRole, newAdmin));
    }

    function test_Invariant_OwnerAlwaysHasAdminRole() public {
        address[] memory admins = new address[](3);
        admins[0] = makeAddr("admin0");
        admins[1] = makeAddr("admin1");
        admins[2] = makeAddr("admin2");

        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        address currentAdmin = initialOwner;

        for (uint256 i = 0; i < 3; i++) {
            vm.prank(currentAdmin);
            proxy.transferOwnership(admins[i]);

            address currentOwner = proxy.owner();
            assertTrue(
                proxy.hasRole(adminRole, currentOwner), "Owner must always have DEFAULT_ADMIN_ROLE"
            );

            currentAdmin = admins[i];
        }
    }

    // ============================================
    // Restricted Governance Actions
    // ============================================

    function test_RenounceOwnership_Reverts() public {
        vm.startPrank(initialOwner);
        vm.expectRevert(StakeTableV2.CannotRenounceOwnership.selector);
        proxy.renounceOwnership();
        vm.stopPrank();
    }

    function test_RevokeAdminRole_Reverts() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);
        vm.expectRevert(StakeTableV2.CannotRevokeDefaultAdmin.selector);
        proxy.revokeRole(adminRole, initialOwner);
        vm.stopPrank();
    }

    function test_RenounceAdminRole_Reverts() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);
        vm.expectRevert(StakeTableV2.CannotRenounceDefaultAdmin.selector);
        proxy.renounceRole(adminRole, initialOwner);
        vm.stopPrank();
    }

    function test_GovernanceDriftCannotBeCreated() public {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);
        vm.expectRevert(StakeTableV2.CannotRevokeDefaultAdmin.selector);
        proxy.revokeRole(adminRole, initialOwner);
        vm.stopPrank();

        vm.startPrank(initialOwner);
        vm.expectRevert(StakeTableV2.CannotRenounceDefaultAdmin.selector);
        proxy.renounceRole(adminRole, initialOwner);
        vm.stopPrank();

        vm.startPrank(initialOwner);
        vm.expectRevert(StakeTableV2.CannotRenounceOwnership.selector);
        proxy.renounceOwnership();
        vm.stopPrank();
    }

    function test_TransferOwnership_RevertsWhenCallerNotAdmin() public {
        address notAdmin = makeAddr("notAdmin");
        address newOwner = makeAddr("newOwner");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);
        proxy.grantRole(proxy.PAUSER_ROLE(), notAdmin); // ensure notAdmin is in actors but not
            // admin
        vm.stopPrank();

        vm.startPrank(notAdmin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, notAdmin, adminRole
            )
        );
        proxy.transferOwnership(newOwner);
        vm.stopPrank();
    }

    function test_RevokeRole_RevertsWhenCallerNotAdmin() public {
        address notAdmin = makeAddr("notAdmin");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);
        proxy.grantRole(proxy.PAUSER_ROLE(), notAdmin);
        vm.stopPrank();

        vm.startPrank(notAdmin);
        vm.expectRevert(StakeTableV2.CannotRevokeDefaultAdmin.selector);
        proxy.revokeRole(adminRole, initialOwner);
        vm.stopPrank();
    }

    function test_RenounceRole_RevertsWhenCallerNotAuthorized() public {
        address other = makeAddr("other");
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        vm.startPrank(initialOwner);
        proxy.grantRole(proxy.PAUSER_ROLE(), other);
        vm.stopPrank();

        vm.startPrank(other);
        vm.expectRevert(StakeTableV2.CannotRenounceDefaultAdmin.selector);
        proxy.renounceRole(adminRole, other);
        vm.stopPrank();
    }
}
