// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import { ReentrancyGuardUpgradeable } from
    "@openzeppelin/contracts-upgradeable/utils/ReentrancyGuardUpgradeable.sol";
import "./mocks/MockRewardClaim.sol";
import "../src/interfaces/IRewardClaim.sol";

contract MaliciousEspToken is ERC20 {
    IRewardClaim public rewardClaim;
    bool private _reentered;

    constructor() ERC20("ESP", "ESP") {
        _mint(msg.sender, 1_000_000);
    }

    function initializeV2(address _rewardClaim) external {
        rewardClaim = IRewardClaim(_rewardClaim);
    }

    function mint(address, uint256 amount) public {
        if (!_reentered) {
            _reentered = true;
            rewardClaim.claimRewards(0, "");
        }
        _mint(msg.sender, amount);
    }
}

contract RewardClaimReentrancyTest is Test {
    MockRewardClaim public rewardClaim;
    MaliciousEspToken public token;

    function setUp() public {
        address owner = address(this);

        token = new MaliciousEspToken();

        rewardClaim = MockRewardClaim(
            address(
                new ERC1967Proxy(
                    address(new MockRewardClaim()),
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

        token.initializeV2(address(rewardClaim));
    }

    // @dev Regression test to ensure nonReentrant modifier remains on claimRewards().
    // While a compromised token could mint directly, this test prevents future
    // developers from removing the modifier thinking "we trust the token" or
    // "this is unnecessary gas overhead". The modifier makes security properties
    // simpler to reason about and is intentionally kept.
    function test_ReentrancyBlocked() public {
        vm.expectRevert(ReentrancyGuardUpgradeable.ReentrancyGuardReentrantCall.selector);
        rewardClaim.claimRewards(1, "");
    }
}
