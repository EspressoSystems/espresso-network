// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";

contract RewardClaimRolesTest is RewardClaimTest {
    bytes32 public constant PAUSER_ROLE = keccak256("PAUSER_ROLE");
    bytes32 public constant DEFAULT_ADMIN_ROLE = 0x00;

    function test_GrantPauserRole() public {
        address newPauser = makeAddr("newPauser");

        vm.prank(owner);
        rewardClaim.grantRole(PAUSER_ROLE, newPauser);

        assertTrue(rewardClaim.hasRole(PAUSER_ROLE, newPauser));

        vm.prank(newPauser);
        rewardClaim.pause();
    }

    function test_RevokePauserRole() public {
        vm.prank(owner);
        rewardClaim.revokeRole(PAUSER_ROLE, pauser);

        assertFalse(rewardClaim.hasRole(PAUSER_ROLE, pauser));

        vm.prank(pauser);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, pauser, PAUSER_ROLE
            )
        );
        rewardClaim.pause();
    }

    function test_GrantRole_RevertsNonAdmin() public {
        address attacker = makeAddr("attacker");
        address newPauser = makeAddr("newPauser");

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                attacker,
                DEFAULT_ADMIN_ROLE
            )
        );
        rewardClaim.grantRole(PAUSER_ROLE, newPauser);
    }

    function test_RevokeRole_RevertsNonAdmin() public {
        address attacker = makeAddr("attacker");

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                attacker,
                DEFAULT_ADMIN_ROLE
            )
        );
        rewardClaim.revokeRole(PAUSER_ROLE, pauser);
    }
}
