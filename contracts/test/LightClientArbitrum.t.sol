// SPDX-License-Identifier: Unlicensed
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import { LightClientArbitrum, ArbSys } from "../src/LightClientArbitrum.sol";

contract MockArbSys is ArbSys {
    function arbBlockNumber() external pure override returns (uint256) {
        return 123456;
    }
}

contract LightClientArbitrumTest is Test {
    LightClientArbitrum public lc;
    MockArbSys mockArbsys;

    function setUp() public {
        vm.createSelectFork("https://arb1.arbitrum.io/rpc");
        mockArbsys = new MockArbSys();
        vm.etch(address(100), address(mockArbsys).code); // Replace address(100) with mock
        // implementation
        lc = new LightClientArbitrum();
    }

    function testCurrentBlockNumber() public view {
        assertNotEq(lc.currentBlockNumber(), block.number);
        assertEq(lc.currentBlockNumber(), ArbSys(address(uint160(100))).arbBlockNumber());
    }
}
