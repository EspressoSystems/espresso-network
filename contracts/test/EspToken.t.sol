// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import { Test } from "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

// Target contract
import { EspToken } from "../src/EspToken.sol";

contract EspTokenUpgradabilityTest is Test {
    address public admin;
    address tokenGrantRecipient;
    EspToken public token;

    function setUp() public {
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        admin = makeAddr("admin");

        EspToken tokenImpl = new EspToken();
        bytes memory initData =
            abi.encodeWithSignature("initialize(address,address)", admin, tokenGrantRecipient);
        ERC1967Proxy proxy = new ERC1967Proxy(address(tokenImpl), initData);
        token = EspToken(payable(address(proxy)));
    }

    // For now we just check that the contract is deployed and minted balance is as expected.

    function testDeployment() public payable {
        assertEq(token.name(), "Espresso Token");
        assertEq(token.symbol(), "ESP");
        assertEq(token.balanceOf(tokenGrantRecipient), 10_000_000_000 ether);
    }
}
