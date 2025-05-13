// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import { Test } from "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

// Target contracts
import { EspToken } from "../src/EspToken.sol";
import { EspTokenV2 } from "../src/EspTokenV2.sol";

contract EspTokenUpgradabilityTest is Test {
    address public admin;
    address tokenGrantRecipient;
    EspToken public token;

    function setUp() public {
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        admin = makeAddr("admin");

        EspToken tokenImpl = new EspToken();
        bytes memory initData = abi.encodeWithSignature(
            "initialize(address,address,uint256,string,string)",
            admin,
            tokenGrantRecipient,
            3_590_000_000,
            "Espresso Token",
            "ESP"
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(tokenImpl), initData);
        token = EspToken(payable(address(proxy)));
    }

    // For now we just check that the contract is deployed and minted balance is as expected.

    function testDeployment() public payable {
        assertEq(token.name(), "Espresso Token");
        assertEq(token.symbol(), "ESP");
        assertEq(token.balanceOf(tokenGrantRecipient), 3_590_000_000 ether);
    }

    function testUpgrade() public {
        EspTokenV2 tokenV2 = new EspTokenV2();
        vm.startPrank(admin);
        token.upgradeToAndCall(address(tokenV2), "");
        vm.stopPrank();
        assertEq(token.name(), "Espresso");
        assertEq(token.symbol(), "ESP");
        assertEq(token.balanceOf(tokenGrantRecipient), 3_590_000_000 ether);
        (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion) = token.getVersion();
        assertEq(majorVersion, 2);
        assertEq(minorVersion, 0);
        assertEq(patchVersion, 0);
    }
}
