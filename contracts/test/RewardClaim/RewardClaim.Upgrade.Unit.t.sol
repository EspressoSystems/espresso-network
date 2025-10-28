// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "./RewardClaim.t.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract RewardClaimUpgradeTest is RewardClaimTest {
    function test_Upgrade_OnlyOwner() public {
        address newImpl = address(new RewardClaim());

        vm.prank(owner);
        rewardClaim.upgradeToAndCall(newImpl, "");
    }

    function test_Upgrade_RevertsNonOwner() public {
        address newImpl = address(new RewardClaim());
        address attacker = makeAddr("attacker");

        vm.prank(attacker);
        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableUnauthorizedAccount.selector, attacker)
        );
        rewardClaim.upgradeToAndCall(newImpl, "");
    }
}
