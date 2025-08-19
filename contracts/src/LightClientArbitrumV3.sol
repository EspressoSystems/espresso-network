// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { LightClientV3 } from "./LightClientV3.sol";

interface ArbSys {
    function arbBlockNumber() external view returns (uint256);
}

contract LightClientArbitrumV3 is LightClientV3 {
    function currentBlockNumber() public view virtual override returns (uint256) {
        return ArbSys(address(uint160(100))).arbBlockNumber();
    }
}
