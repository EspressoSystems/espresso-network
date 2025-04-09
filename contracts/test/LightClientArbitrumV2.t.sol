// SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import { LightClientArbitrumV2, ArbSys } from "../src/LightClientArbitrumV2.sol";

contract MockArbSys is ArbSys {
    function arbBlockNumber() external pure override returns (uint256) {
        return 123456;
    }
}

contract LightClientArbitrumV2Test is Test {
    LightClientArbitrumV2 public lc;
    MockArbSys mockArbsys;

    function setUp() public {
        vm.createSelectFork("https://arb1.arbitrum.io/rpc");
        mockArbsys = new MockArbSys();
        vm.etch(address(100), address(mockArbsys).code); // Replace address(100) with mock
        // implementation
        lc = new LightClientArbitrumV2();
    }

    function testCurrentBlockNumber() public view {
        assertNotEq(lc.currentBlockNumber(), block.number);
        assertEq(lc.currentBlockNumber(), ArbSys(address(uint160(100))).arbBlockNumber());
    }
}
