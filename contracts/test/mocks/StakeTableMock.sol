// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { StakeTable } from "../../src/StakeTable.sol";

contract StakeTableMock is StakeTable {
    constructor(address _tokenAddress, address _lightClientAddress, uint256 _exitEscrowPeriod)
        StakeTable()
    {
        initializeAtBlock();
        initializeState(_tokenAddress, _lightClientAddress, _exitEscrowPeriod);
    }
}
