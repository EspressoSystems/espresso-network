// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Script } from "forge-std/Script.sol";

import { LightClientV2Fake as LCV2 } from "../mocks/LightClientV2Fake.sol";
import { LightClient as LC } from "../../src/LightClient.sol";

contract UpgradeLightClientScript is Script {
    /// @notice runs the upgrade
    /// @param mostRecentlyDeployedProxy address of deployed proxy
    /// @return address of the proxy
    /// TODO get the most recent deployment from the devops tooling
    function run(
        address mostRecentlyDeployedProxy,
        uint256 newField,
        uint256 extraField,
        address admin
    ) external returns (address) {
        vm.startBroadcast(admin);
        address proxy =
            upgradeLightClient(mostRecentlyDeployedProxy, address(new LCV2()), newField, extraField);
        vm.stopBroadcast();
        return proxy;
    }

    /// @notice upgrades the light client contract by calling the upgrade function the
    /// implementation contract via
    /// the proxy
    /// @param proxyAddress address of proxy
    /// @param newLightClient address of new implementation
    /// @return address of the proxy
    function upgradeLightClient(
        address proxyAddress,
        address newLightClient,
        uint256 newField,
        uint256 extraField
    ) public returns (address) {
        LC proxy = LC(proxyAddress); //make the function call on the previous implementation

        proxy.upgradeToAndCall(
            newLightClient, abi.encodeCall(LCV2.initializeV2, (newField, extraField))
        ); //proxy
            // address now points to the new
            // implementation
        return address(proxy);
    }
}
