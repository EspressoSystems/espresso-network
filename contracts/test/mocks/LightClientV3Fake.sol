// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { LightClientV2Fake } from "./LightClientV2Fake.sol";

/// @dev a Fake V3 for testing upgradability purposes only
contract LightClientV3Fake is LightClientV2Fake {
    uint256 public anotherField;

    /// @notice Initialize v3
    /// @param _newField   New field amount
    /// @dev the reinitializer modifier is used to reinitialize new versions of a contract and
    /// is called at most once. The modifier has an uint64 argument which indicates the next
    /// contract version.
    /// when the base implementation contract is initialized for the first time, the _initialized
    /// version
    /// is set to 1. Since this is the 3rd implementation, the next contract version is 3.
    function initializeV3(uint256 _newField) external reinitializer(3) {
        anotherField = _newField;
    }

    /// @notice Use this to get the implementation contract version
    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (3, 0, 0);
    }
}
