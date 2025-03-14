// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { StakeTable } from "../../src/StakeTable.sol";
import { LightClientV2Mock } from "./LightClientV2Mock.sol";

contract StakeTableMock is StakeTable {
    constructor(
        address token,
        address lightClientAddress,
        uint64 churnRate,
        uint256 minStakeAmount,
        address initialOwner
    ) StakeTable(token, lightClientAddress, churnRate, minStakeAmount, initialOwner) 
    // solhint-disable-next-line no-empty-blocks
    { }

    function setRegistrationEpoch(uint64 epoch) public {
        registrationEpoch = epoch;
    }

    function setExitEpoch(uint64 epoch) public {
        exitEpoch = epoch;
    }

    // Expose the internal function for testing by calling the super implementation
    function mockPushToRegistrationQueue() public {
        super.pushToRegistrationQueue();
    }

    function mockPushToExitQueue() public {
        super.pushToExitQueue();
    }

    // expose the internal function for testing by calling the super implementation
    function mockExitEscrowPeriod(Node memory node) public pure returns (uint64) {
        return super.exitEscrowPeriod(node);
    }

    function mockUpdateHotShotBlocksPerEpoch(uint64 newHotShotBlocksPerEpoch) public {
        LightClientV2Mock(address(lightClient)).setBlocksPerEpoch(newHotShotBlocksPerEpoch);
    }
}
