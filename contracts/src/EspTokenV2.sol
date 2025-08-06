// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "./EspToken.sol";
import { AccessControlUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/AccessControlUpgradeable.sol";

contract EspTokenV2 is EspToken, AccessControlUpgradeable {
    bytes32 public constant MINTER_ROLE = keccak256("MINTER_ROLE");

    constructor() {
        _disableInitializers();
    }

    function initializeV2() public reinitializer(2) {
        __AccessControl_init();
        _grantRole(DEFAULT_ADMIN_ROLE, owner());
    }

    function mint(address to, uint256 amount) public onlyRole(MINTER_ROLE) {
        _mint(to, amount);
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
