// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

interface ILightClient {
    function blocksPerEpoch() external view returns (uint64);
}
