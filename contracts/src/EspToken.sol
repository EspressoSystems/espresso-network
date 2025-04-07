// contracts/GLDToken.sol
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

// TODO MA: should we really use solmate, or just stick with openzeppelin?
import { ERC20Upgradeable } from
    "@openzeppelin/contracts-upgradeable/token/ERC20/ERC20Upgradeable.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { UUPSUpgradeable } from
    "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";

// TODO: Timelock
contract EspToken is Initializable, ERC20Upgradeable, OwnableUpgradeable, UUPSUpgradeable {
    /// @notice upgrade event when the proxy updates the implementation it's pointing to
    event Upgrade(address implementation);

    /// @notice since the constructor initializes storage on this contract we disable it
    /// @dev storage is on the proxy contract since it calls this contract via delegatecall
    /// @custom:oz-upgrades-unsafe-allow constructor
    constructor() {
        _disableInitializers();
    }

    /// @notice This contract is called by the proxy when you deploy this contract
    function initialize(address _owner, address _initialGrantRecipient) public initializer {
        __ERC20_init("Espresso Token", "ESP");
        __Ownable_init(_owner);
        __UUPSUpgradeable_init();
        _mint(_initialGrantRecipient, 1000_000_000 ether);
    }

    /// @notice only the owner can authorize an upgrade
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {
        emit Upgrade(newImplementation);
    }

    /// @notice Use this to get the implementation contract version
    function getVersion()
        public
        pure
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (1, 0, 0);
    }
}
