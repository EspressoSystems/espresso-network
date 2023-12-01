// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { LightClient } from "../../src/LightClient.sol";

/// @dev A helper that wraps LightClient contract for testing
contract LightClientTest is LightClient {
    constructor(LightClientState memory genesis, uint32 numBlockPerEpoch)
        LightClient(genesis, numBlockPerEpoch)
    { }

    /// @dev Directly mutate `currentEpoch` variable for test
    function setCurrentEpoch(uint64 newEpoch) public {
        currentEpoch = newEpoch;
    }
}
