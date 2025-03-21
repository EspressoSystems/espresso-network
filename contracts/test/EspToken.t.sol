// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import { Test } from "forge-std/Test.sol";

// Target contract
import { EspToken } from "../src/EspToken.sol";
import { DeployEspTokenScript } from "./script/EspToken.s.sol";

contract EspTokenUpgradabilityTest is Test {
    address payable public proxy;
    address public admin;
    address tokenGrantRecipient;
    EspToken public espTokenProxy;

    function setUp() public {
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        DeployEspTokenScript deployer = new DeployEspTokenScript();
        (proxy, admin) = deployer.run(tokenGrantRecipient);
        espTokenProxy = EspToken(proxy);
    }

    // For now we just check that the contract is deployed and minted balance is as expected.

    function testDeployment() public payable {
        assertEq(espTokenProxy.balanceOf(tokenGrantRecipient), 1_000_000_000 ether);
    }
}
