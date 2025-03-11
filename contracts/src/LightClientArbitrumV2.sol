// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { LightClientV2 } from "./LightClientV2.sol";

interface ArbSys {
    function arbBlockNumber() external view returns (uint256);
}

contract LightClientArbitrumV2 is LightClientV2 {
    function currentBlockNumber() public view virtual override returns (uint256) {
        return ArbSys(address(uint160(100))).arbBlockNumber();
    }
}
