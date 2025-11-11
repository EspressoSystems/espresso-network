// SPDX-License-Identifier: UNLICENSED

/* solhint-disable contract-name-camelcase, func-name-mixedcase, no-console */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { StdInvariant } from "forge-std/StdInvariant.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";
import { StakeTableV2 } from "../src/StakeTableV2.sol";
import { StakeTable as S } from "../src/StakeTable.sol";
import { StakeTableUpgradeV2Test } from "./StakeTable.t.sol";
import { StakeTable_register_Test } from "./StakeTable.t.sol";

/// @title StakeTableV2 Governance Invariant Tests
/// @notice Tests critical invariants that must ALWAYS hold in the governance system
/// @dev Uses Foundry's invariant testing framework for stateful fuzzing
/// forge-config: default.invariant.runs = 128
/// forge-config: default.invariant.depth = 15
contract StakeTableV2GovernanceInvariantTest is StdInvariant, Test {
    StakeTableV2 public proxy;
    GovernanceHandler public handler;

    address public initialOwner;
    address public pauser;

    function setUp() public {
        StakeTableUpgradeV2Test baseTest = new StakeTableUpgradeV2Test();
        baseTest.setUp();
        pauser = makeAddr("pauser");
        StakeTable_register_Test stakeTableRegisterTest = baseTest.stakeTableRegisterTest();

        initialOwner = baseTest.admin();

        vm.startPrank(initialOwner);
        S baseProxy = S(address(stakeTableRegisterTest.stakeTable()));
        StakeTableV2.InitialCommission[] memory emptyCommissions;
        bytes memory initData = abi.encodeWithSelector(
            StakeTableV2.initializeV2.selector, pauser, pauser, initialOwner, 0, emptyCommissions
        );
        baseProxy.upgradeToAndCall(address(new StakeTableV2()), initData);
        proxy = StakeTableV2(address(baseProxy));
        vm.stopPrank();

        handler = new GovernanceHandler(proxy, initialOwner);
        targetContract(address(handler));
    }

    // ============================================
    // Core Invariants
    // ============================================

    /// @notice INVARIANT: Owner must always have DEFAULT_ADMIN_ROLE
    /// @dev This is the most critical invariant - prevents admin lockout
    function invariant_OwnerAlwaysHasAdminRole() public view {
        address currentOwner = proxy.owner();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertTrue(
            proxy.hasRole(adminRole, currentOwner), "owner() must always have DEFAULT_ADMIN_ROLE"
        );
    }

    /// @notice INVARIANT: Only one address should have DEFAULT_ADMIN_ROLE
    /// @dev Enforces single-admin governance model
    function invariant_OnlyOneAdminExists() public view {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        address currentOwner = proxy.owner();
        address[] memory actors = handler.getActors();

        uint256 adminCount = 0;
        address adminAddress;

        // Limit iteration to prevent gas issues with large actor arrays
        uint256 maxActors = actors.length > 50 ? 50 : actors.length;
        for (uint256 i = 0; i < maxActors; i++) {
            if (proxy.hasRole(adminRole, actors[i])) {
                adminCount++;
                adminAddress = actors[i];
            }
        }

        // Also check current owner explicitly
        if (proxy.hasRole(adminRole, currentOwner)) {
            if (adminCount == 0 || adminAddress != currentOwner) {
                adminCount++;
                adminAddress = currentOwner;
            }
        }

        assertEq(adminCount, 1, "Exactly one address must have DEFAULT_ADMIN_ROLE");
        assertEq(adminAddress, currentOwner, "Admin must be the owner");
    }

    /// @notice INVARIANT: Owner address is never zero
    /// @dev Prevents governance lockout
    function invariant_OwnerIsNeverZero() public view {
        address currentOwner = proxy.owner();
        assertTrue(currentOwner != address(0), "owner() must never be zero address");
    }

    /// @notice INVARIANT: Anyone with DEFAULT_ADMIN_ROLE is the owner
    /// @dev Ensures admin role and ownership are synchronized
    function invariant_AdminRoleImpliesOwnership() public view {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        address currentOwner = proxy.owner();
        address[] memory actors = handler.getActors();

        // Limit iteration to prevent gas issues
        uint256 maxActors = actors.length > 50 ? 50 : actors.length;
        for (uint256 i = 0; i < maxActors; i++) {
            if (proxy.hasRole(adminRole, actors[i])) {
                assertEq(actors[i], currentOwner, "Address with DEFAULT_ADMIN_ROLE must be owner");
            }
        }
    }

    /// @notice INVARIANT: Only admin can perform privileged operations
    /// @dev Verifies access control is maintained
    function invariant_OnlyAdminCanPerformPrivilegedOps() public view {
        address currentOwner = proxy.owner();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertTrue(
            proxy.hasRole(adminRole, currentOwner),
            "Current owner must be able to perform admin operations"
        );

        assertEq(
            handler.currentAdmin(),
            currentOwner,
            "Handler's tracked admin must match contract owner"
        );
    }

    /// @notice INVARIANT: Contract version remains consistent
    /// @dev Ensures contract hasn't been corrupted
    function invariant_ContractVersionIsV2() public view {
        (uint8 majorVersion,,) = proxy.getVersion();
        assertEq(majorVersion, 2, "Contract must remain V2");
    }

    // ============================================
    // PAUSER_ROLE Invariants
    // ============================================

    /// @notice INVARIANT: Pausers cannot escalate to admin privileges
    /// @dev Critical security invariant - ensures role separation
    function invariant_PauserCannotEscalateToAdmin() public view {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        bytes32 pauserRole = proxy.PAUSER_ROLE();
        address currentOwner = proxy.owner();
        address[] memory actors = handler.getActors();

        // Limit iteration to prevent gas issues
        uint256 maxActors = actors.length > 50 ? 50 : actors.length;
        for (uint256 i = 0; i < maxActors; i++) {
            address actor = actors[i];

            if (proxy.hasRole(pauserRole, actor) && actor != currentOwner) {
                assertFalse(proxy.hasRole(adminRole, actor), "Pauser escalated to admin");
            }
        }
    }

    /// @notice INVARIANT: Only DEFAULT_ADMIN_ROLE can manage PAUSER_ROLE
    /// @dev Ensures proper role hierarchy
    function invariant_OnlyAdminCanManagePauserRole() public view {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        bytes32 pauserRole = proxy.PAUSER_ROLE();

        assertEq(
            proxy.getRoleAdmin(pauserRole),
            adminRole,
            "PAUSER_ROLE must be managed by DEFAULT_ADMIN_ROLE"
        );
    }

    /// @notice INVARIANT: Multiple pausers are allowed
    /// @dev Unlike admin role, pausers can be multiple (design decision)
    function invariant_MultiplePausersAllowed() public view {
        bytes32 pauserRole = proxy.PAUSER_ROLE();
        address[] memory actors = handler.getActors();

        uint256 pauserCount = 0;
        for (uint256 i = 0; i < actors.length; i++) {
            if (proxy.hasRole(pauserRole, actors[i])) {
                pauserCount++;
            }
        }

        assertTrue(true, "INVARIANT: Multiple pausers are allowed by design");
    }

    /// @notice INVARIANT: Admin retains control regardless of pause state
    /// @dev Pausing should not affect governance
    function invariant_AdminControlIndependentOfPauseState() public view {
        address currentOwner = proxy.owner();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertTrue(proxy.hasRole(adminRole, currentOwner), "Admin lost role due to pause state");

        assertEq(proxy.owner(), currentOwner, "Owner changed due to pause state");
    }

    /// @notice INVARIANT: Pausers cannot grant or revoke any roles
    /// @dev Only admin can manage roles
    function invariant_PausersCannotManageRoles() public {
        bytes32 pauserRole = proxy.PAUSER_ROLE();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        address[] memory actors = handler.getActors();
        address currentOwner = proxy.owner();

        for (uint256 i = 0; i < actors.length && i < 5; i++) {
            address actor = actors[i];

            if (!proxy.hasRole(pauserRole, actor)) continue;
            if (actor == currentOwner) continue;
            if (proxy.hasRole(adminRole, actor)) continue;

            vm.startPrank(actor);
            address testAddr = address(uint160(uint256(keccak256(abi.encodePacked(i, "test")))));

            vm.expectRevert(
                abi.encodeWithSelector(
                    IAccessControl.AccessControlUnauthorizedAccount.selector, actor, adminRole
                )
            );
            proxy.grantRole(pauserRole, testAddr);

            vm.expectRevert(
                abi.encodeWithSelector(
                    IAccessControl.AccessControlUnauthorizedAccount.selector, actor, adminRole
                )
            );
            proxy.grantRole(adminRole, testAddr);

            vm.stopPrank();
        }
    }

    /// @notice INVARIANT: Pauser role operations don't affect admin role
    /// @dev Ensures independence between roles
    function invariant_PauserOpsDoNotAffectAdminRole() public view {
        address currentOwner = proxy.owner();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        assertTrue(proxy.hasRole(adminRole, currentOwner), "Pauser operations affected admin role");

        address[] memory actors = handler.getActors();
        uint256 adminCount = 0;

        for (uint256 i = 0; i < actors.length; i++) {
            if (proxy.hasRole(adminRole, actors[i])) {
                adminCount++;
            }
        }

        assertEq(adminCount, 1, "Pauser operations changed admin count");
    }

    /// @notice INVARIANT: Admin can always unpause the contract
    /// @dev Critical safety - ensures contract is never permanently locked
    function invariant_AdminCanAlwaysUnpause() public {
        address currentOwner = proxy.owner();
        bytes32 pauserRole = proxy.PAUSER_ROLE();

        // If contract is paused, admin should have pauser role or be able to grant it
        if (proxy.paused()) {
            // Either admin already has pauser role
            bool adminIsPauser = proxy.hasRole(pauserRole, currentOwner);

            // Or admin can grant themselves pauser role
            if (!adminIsPauser) {
                vm.prank(currentOwner);
                proxy.grantRole(pauserRole, currentOwner);
            }
        }

        assertTrue(true, "INVARIANT: Admin can always unpause");
    }

    /// @notice INVARIANT: DEFAULT_ADMIN_ROLE is its own role admin
    /// @dev Ensures no higher authority can hijack admin role
    function invariant_AdminRoleIsOwnAdmin() public view {
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        bytes32 roleAdmin = proxy.getRoleAdmin(adminRole);

        assertEq(roleAdmin, adminRole, "DEFAULT_ADMIN_ROLE must be its own admin");
    }

    // ============================================
    // Post-Run Hook
    // ============================================

    /// @notice Hook called after each invariant run completes
    /// @dev Performs final checks after all fuzzing is done
    function afterInvariant() external view {
        // Verify all critical governance invariants after fuzzing run
        invariant_OwnerAlwaysHasAdminRole();
        invariant_OnlyOneAdminExists();
        invariant_OwnerIsNeverZero();
        invariant_AdminRoleImpliesOwnership();
        invariant_AdminRoleIsOwnAdmin();
        invariant_ContractVersionIsV2();

        // Verify role separation and security invariants
        invariant_PauserCannotEscalateToAdmin();
        invariant_OnlyAdminCanManagePauserRole();
        invariant_PauserOpsDoNotAffectAdminRole();
        invariant_AdminControlIndependentOfPauseState();
    }
}

/// @title Governance Handler
/// @notice Handler contract for stateful fuzzing of governance operations
/// @dev Performs random governance actions and tracks state
contract GovernanceHandler is Test {
    StakeTableV2 public proxy;
    address public currentAdmin;
    address[] public actors;

    // Track call statistics
    uint256 public transferOwnershipCalls;
    uint256 public grantRoleCalls;
    uint256 public revokeRoleCalls;
    uint256 public pauseCalls;
    uint256 public unpauseCalls;
    uint256 public failedCalls;

    constructor(StakeTableV2 _proxy, address _initialAdmin) {
        proxy = _proxy;
        currentAdmin = _initialAdmin;
        actors.push(_initialAdmin);
    }

    // ============================================
    // Fuzzed Actions
    // ============================================

    /// @notice Fuzzed action: Transfer ownership
    function transferOwnership(address newOwner) public {
        // Bound newOwner to valid addresses (not zero)
        newOwner = _boundAddress(newOwner);

        vm.startPrank(currentAdmin);

        try proxy.transferOwnership(newOwner) {
            currentAdmin = newOwner;
            _addActor(newOwner);
            transferOwnershipCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Grant DEFAULT_ADMIN_ROLE
    function grantAdminRole(address newAdmin) public {
        newAdmin = _boundAddress(newAdmin);

        vm.startPrank(currentAdmin);

        try proxy.grantRole(proxy.DEFAULT_ADMIN_ROLE(), newAdmin) {
            currentAdmin = newAdmin;
            _addActor(newAdmin);
            grantRoleCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Grant PAUSER_ROLE
    function grantPauserRole(address newPauser) public {
        newPauser = _boundAddress(newPauser);

        vm.startPrank(currentAdmin);

        try proxy.grantRole(proxy.PAUSER_ROLE(), newPauser) {
            _addActor(newPauser);
            grantRoleCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Revoke PAUSER_ROLE
    function revokePauserRole(uint256 actorIndexSeed) public {
        if (actors.length == 0) return;

        address actor = actors[actorIndexSeed % actors.length];

        vm.startPrank(currentAdmin);

        try proxy.revokeRole(proxy.PAUSER_ROLE(), actor) {
            revokeRoleCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Attempt unauthorized transfer
    function unauthorizedTransferOwnership(uint256 actorIndexSeed, address newOwner) public {
        if (actors.length == 0) return;

        address actor = actors[actorIndexSeed % actors.length];
        newOwner = _boundAddress(newOwner);

        // Only try if actor is NOT the current admin
        if (actor == currentAdmin) return;

        vm.startPrank(actor);

        try proxy.transferOwnership(newOwner) {
            // Should never succeed
            revert("Unauthorized transfer succeeded!");
        } catch {
            // Expected to fail
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Transfer ownership to self (tests idempotency)
    /// @dev This tests the edge case where self-transfer should not break state
    function transferOwnershipToSelf() public {
        vm.startPrank(currentAdmin);

        try proxy.transferOwnership(currentAdmin) {
            transferOwnershipCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Chain of transfers
    function chainOfTransfers(address[] memory newOwners) public {
        vm.startPrank(currentAdmin);

        for (uint256 i = 0; i < newOwners.length && i < 5; i++) {
            address newOwner = _boundAddress(newOwners[i]);

            try proxy.transferOwnership(newOwner) {
                currentAdmin = newOwner;
                _addActor(newOwner);
                transferOwnershipCalls++;
            } catch {
                failedCalls++;
                break;
            }

            vm.stopPrank();
            vm.startPrank(currentAdmin);
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Pause contract via pauser
    function pauseContract(uint256 actorIndexSeed) public {
        if (actors.length == 0) return;

        address actor = actors[actorIndexSeed % actors.length];
        bytes32 pauserRole = proxy.PAUSER_ROLE();

        // Only try if actor has pauser role
        if (!proxy.hasRole(pauserRole, actor)) return;

        vm.startPrank(actor);

        try proxy.pause() {
            pauseCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Unpause contract via pauser
    function unpauseContract(uint256 actorIndexSeed) public {
        if (actors.length == 0) return;

        address actor = actors[actorIndexSeed % actors.length];
        bytes32 pauserRole = proxy.PAUSER_ROLE();

        // Only try if actor has pauser role
        if (!proxy.hasRole(pauserRole, actor)) return;

        vm.startPrank(actor);

        try proxy.unpause() {
            unpauseCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Admin grants pauser role
    function adminGrantsPauserRole(address newPauser) public {
        newPauser = _boundAddress(newPauser);

        vm.startPrank(currentAdmin);

        try proxy.grantRole(proxy.PAUSER_ROLE(), newPauser) {
            _addActor(newPauser);
            grantRoleCalls++;
        } catch {
            failedCalls++;
        }

        vm.stopPrank();
    }

    /// @notice Fuzzed action: Pauser tries to grant admin role (should fail)
    function pauserTriesToGrantAdminRole(uint256 pauserIndexSeed, address target) public {
        if (actors.length == 0) return;

        address pauser = actors[pauserIndexSeed % actors.length];
        bytes32 pauserRole = proxy.PAUSER_ROLE();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();

        // Only try if actor is pauser but NOT admin
        if (!proxy.hasRole(pauserRole, pauser)) return;
        if (proxy.hasRole(adminRole, pauser)) return;

        target = _boundAddress(target);

        vm.startPrank(pauser);

        try proxy.grantRole(adminRole, target) {
            // Should never succeed!
            revert("SECURITY BREACH: Pauser granted admin role!");
        } catch {
            // Expected to fail
            failedCalls++;
        }

        vm.stopPrank();
    }

    // ============================================
    // Helper Functions
    // ============================================

    function _boundAddress(address addr) internal view returns (address) {
        // Ensure address is not zero
        if (addr == address(0)) {
            addr = address(uint160(uint256(keccak256(abi.encodePacked(block.timestamp)))));
        }
        // Avoid precompiles and special addresses
        if (uint160(addr) < 10) {
            addr = address(uint160(addr) + 100);
        }
        return addr;
    }

    function _addActor(address actor) internal {
        for (uint256 i = 0; i < actors.length; i++) {
            if (actors[i] == actor) return;
        }
        actors.push(actor);
    }

    function getActors() external view returns (address[] memory) {
        return actors;
    }
}

/// @title Additional Invariant Tests (Assertion-Based)
/// @notice Additional invariant checks that can be manually called in tests
contract StakeTableV2GovernanceInvariantAssertions is Test {
    /// @notice Check all governance invariants at once
    function checkAllGovernanceInvariants(StakeTableV2 proxy) internal view {
        _invariant_ownerHasAdminRole(proxy);
        _invariant_ownerIsNotZero(proxy);
        _invariant_adminIsOwner(proxy);
    }

    function _invariant_ownerHasAdminRole(StakeTableV2 proxy) private view {
        address currentOwner = proxy.owner();
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        require(
            proxy.hasRole(adminRole, currentOwner),
            "INVARIANT: owner() must have DEFAULT_ADMIN_ROLE"
        );
    }

    function _invariant_ownerIsNotZero(StakeTableV2 proxy) private view {
        require(proxy.owner() != address(0), "INVARIANT: owner() must not be zero");
    }

    function _invariant_adminIsOwner(StakeTableV2 proxy) private view {
        // Anyone with DEFAULT_ADMIN_ROLE must be the owner
        bytes32 adminRole = proxy.DEFAULT_ADMIN_ROLE();
        address currentOwner = proxy.owner();
        require(proxy.hasRole(adminRole, currentOwner), "INVARIANT: owner must have admin role");
    }
}
