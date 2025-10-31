// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { ReentrancyGuardUpgradeable } from
    "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import "../../src/RewardClaim.sol";

contract MinimalToken {
    function totalSupply() external pure returns (uint256) {
        return 1_000_000;
    }
}

contract RewardClaimReentrancyTest is Test {
    RewardClaim public rewardClaim;
    bytes32 constant REENTRANCY_SLOT =
        0x9b779b17422d0df92223018b32b4d1fa46e071723d6817e2486d003becc55f00;
    uint256 constant ENTERED = 2;

    function setUp() public {
        address owner = address(this);
        MinimalToken token = new MinimalToken();

        rewardClaim = RewardClaim(
            address(
                new ERC1967Proxy(
                    address(new RewardClaim()),
                    abi.encodeWithSignature(
                        "initialize(address,address,address,address)",
                        owner,
                        address(token),
                        makeAddr("lightClient"),
                        owner
                    )
                )
            )
        );
    }

    // @dev Regression test to ensure nonReentrant modifier remains on claimRewards().
    // While a compromised token could mint directly, this test prevents future
    // developers from removing the modifier thinking "we trust the token" or
    // "this is unnecessary gas overhead". The modifier makes security properties
    // simpler to reason about and is intentionally kept.
    function test_ClaimRewards_ReentrancyBlocked() public {
        vm.store(address(rewardClaim), REENTRANCY_SLOT, bytes32(ENTERED));

        vm.expectRevert(ReentrancyGuardUpgradeable.ReentrancyGuardReentrantCall.selector);
        rewardClaim.claimRewards(1, "");
    }

    // @dev Regression test to ensure nonReentrant modifier remains on setDailyLimit().
    // This protects against reentrancy during the external call to espToken.totalSupply().
    // While unlikely to be exploited, this provides defense-in-depth security for critical
    // security parameters and prevents future developers from removing the modifier.
    function test_SetDailyLimit_ReentrancyBlocked() public {
        vm.store(address(rewardClaim), REENTRANCY_SLOT, bytes32(ENTERED));

        vm.expectRevert(ReentrancyGuardUpgradeable.ReentrancyGuardReentrantCall.selector);
        rewardClaim.setDailyLimit(100);
    }
}
