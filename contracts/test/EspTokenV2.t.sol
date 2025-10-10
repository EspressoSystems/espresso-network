// SPDX-License-Identifier: Unlicensed

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.0;

import { Test } from "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import { EspToken } from "../src/EspToken.sol";
import { EspTokenV2 } from "../src/EspTokenV2.sol";

contract EspTokenV2Test is Test {
    EspTokenV2 public token;
    address public owner;
    address public rewardClaim;
    address public user;

    function setUp() public {
        owner = address(this);
        rewardClaim = makeAddr("rewardClaim");
        user = makeAddr("user");

        EspToken implementation = new EspToken();
        bytes memory initData = abi.encodeWithSelector(
            EspToken.initialize.selector, owner, owner, 1000000, "Espresso Token", "ESP"
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(implementation), initData);

        EspToken(address(proxy)).upgradeToAndCall(address(new EspTokenV2()), "");

        token = EspTokenV2(address(proxy));
        token.initializeV2(rewardClaim);
    }

    function test_mint_ByRewardClaim() public {
        uint256 balanceBefore = token.balanceOf(user);
        uint256 totalSupplyBefore = token.totalSupply();

        vm.expectEmit();
        emit IERC20.Transfer(address(0), user, 1);

        vm.prank(rewardClaim);
        token.mint(user, 1);

        assertEq(token.balanceOf(user), balanceBefore + 1);
        assertEq(token.totalSupply(), totalSupplyBefore + 1);
    }

    function testFuzz_mint_ByRewardClaim(address recipient, uint256 amount) public {
        vm.assume(recipient != address(0));
        amount = bound(amount, 1, type(uint256).max / 2);

        uint256 balanceBefore = token.balanceOf(recipient);
        uint256 totalSupplyBefore = token.totalSupply();

        vm.expectEmit();
        emit IERC20.Transfer(address(0), recipient, amount);

        vm.prank(rewardClaim);
        token.mint(recipient, amount);

        assertEq(token.balanceOf(recipient), balanceBefore + amount);
        assertEq(token.totalSupply(), totalSupplyBefore + amount);
    }

    function test_mint_ByUnauthorizedReverts() public {
        vm.prank(user);
        vm.expectRevert(EspTokenV2.OnlyRewardClaim.selector);
        token.mint(user, 1);
    }

    function testFuzz_mint_ByUnauthorizedReverts(address unauthorized) public {
        vm.assume(unauthorized != rewardClaim);
        vm.prank(unauthorized);
        vm.expectRevert(EspTokenV2.OnlyRewardClaim.selector);
        token.mint(user, 1);
    }
}
