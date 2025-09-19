// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Script } from "forge-std/Script.sol";

import { LightClientV2Fake as LCV2 } from "../mocks/LightClientV2Fake.sol";
import { LightClientV3Fake as LCV3 } from "../mocks/LightClientV3Fake.sol";

contract UpgradeLightClientScript is Script {
    /// @notice runs the upgrade
    /// @param mostRecentlyDeployedProxy address of deployed proxy
    /// @return address of the proxy
    /// TODO get the most recent deployment from the devops tooling
    function run(address mostRecentlyDeployedProxy, uint256 newField, address admin)
        external
        returns (address)
    {
        address proxy =
            upgradeLightClient(admin, mostRecentlyDeployedProxy, address(new LCV3()), newField);
        return proxy;
    }

    /// @notice upgrades the light client contract by calling the upgrade function the
    /// implementation contract via
    /// the proxy
    /// @param admin address of admin to broadcast as
    /// @param proxyAddress address of proxy
    /// @param newLightClient address of new implementation
    /// @return address of the proxy
    function upgradeLightClient(
        address admin,
        address proxyAddress,
        address newLightClient,
        uint256 newField
    ) public returns (address) {
        vm.startBroadcast(admin);
        LCV2 proxy = LCV2(proxyAddress); //make the function call on the previous implementation

        proxy.upgradeToAndCall(newLightClient, abi.encodeCall(LCV3.initializeV3, newField)); //proxy
            // address now points to the new
            // implementation
        vm.stopBroadcast();
        return address(proxy);
    }
}
