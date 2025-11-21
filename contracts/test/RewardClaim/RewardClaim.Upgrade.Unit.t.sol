// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";

contract RewardClaimUpgradeTest is RewardClaimTest {
    function test_Upgrade_OnlyAdmin() public {
        address newImpl = address(new RewardClaim());

        vm.prank(owner);
        rewardClaim.upgradeToAndCall(newImpl, "");
    }

    function test_Upgrade_RevertsNonAdmin() public {
        address newImpl = address(new RewardClaim());
        address attacker = makeAddr("attacker");
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector, attacker, adminRole
            )
        );
        rewardClaim.upgradeToAndCall(newImpl, "");
    }
}
