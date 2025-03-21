// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { LightClient } from "./LightClient.sol";

interface ArbSys {
    function arbBlockNumber() external view returns (uint256);
}

contract LightClientArbitrum is LightClient {
    function currentBlockNumber() public view virtual override returns (uint256) {
        return ArbSys(address(uint160(100))).arbBlockNumber();
    }
}
