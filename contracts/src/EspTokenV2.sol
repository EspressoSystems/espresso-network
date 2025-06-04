// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./EspToken.sol";

contract EspTokenV2 is EspToken {
    constructor() {
        _disableInitializers();
    }

    function name() public pure override returns (string memory) {
        return "Espresso";
    }

    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (2, 0, 0);
    }
}
