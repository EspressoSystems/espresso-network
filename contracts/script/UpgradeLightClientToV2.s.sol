// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Script } from "forge-std/Script.sol";

import { LightClientV2 as LCV2 } from "../test/LightClientV2.sol";
import { LightClient as LC } from "../src/LightClient.sol";

contract UpgradeLightClientScript is Script {
    /// @notice runs the upgrade
    /// @param mostRecentlyDeployedProxy address of deployed proxy
    /// @return address of the proxy
    /// TODO get the most recent deployment from the devops tooling
    function run(uint32 seedPhraseOffset, address mostRecentlyDeployedProxy, uint256 newField)
        external
        returns (address)
    {
        string memory seedPhrase = vm.envString("MNEMONIC");
        (address admin,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        vm.startBroadcast(admin);
        address proxy = upgradeLightClient(mostRecentlyDeployedProxy, address(new LCV2()), newField);
        return proxy;
    }

    /// @notice upgrades the light client contract by calling the upgrade function the
    /// implementation contract via
    /// the proxy
    /// @param proxyAddress address of proxy
    /// @param newLightClient address of new implementation
    /// @return address of the proxy
    function upgradeLightClient(address proxyAddress, address newLightClient, uint256 newField)
        public
        returns (address)
    {
        LC proxy = LC(proxyAddress); //make the function call on the previous implementation

        proxy.upgradeToAndCall(newLightClient, abi.encodeCall(LCV2.initializeV2, newField)); //proxy
            // address now points to the new
            // implementation
        vm.stopBroadcast();
        return address(proxy);
    }
}
