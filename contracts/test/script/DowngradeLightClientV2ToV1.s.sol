// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Script } from "forge-std/Script.sol";

import { LightClientV2Fake as LCV2 } from "../mocks/LightClientV2Fake.sol";

contract DowngradeLightClientScript is Script {
    /// @notice runs the downgrade
    /// @param mostRecentlyDeployedProxy address of deployed proxy
    /// @param lightClientImplAddr address of old light client implementation that we're downgrading
    /// to
    /// @return address of the proxy
    function run(address mostRecentlyDeployedProxy, address admin, address lightClientImplAddr)
        external
        returns (address)
    {
        vm.startBroadcast(admin);
        address proxy = downgradeLightClient(mostRecentlyDeployedProxy, lightClientImplAddr);
        vm.stopBroadcast();
        return proxy;
    }

    /// @notice downgrades the light client contract by calling the upgrade function in the
    /// implementation contract via the proxy
    /// @param proxyAddress address of proxy
    /// @param oldLightClient address of old implementation
    /// @return address of the proxy
    function downgradeLightClient(address proxyAddress, address oldLightClient)
        public
        returns (address)
    {
        LCV2 proxy = LCV2(proxyAddress); //make the function call on the previous implementation

        // "upgrade" the proxy address so that it now points to the old implementation
        proxy.upgradeToAndCall(oldLightClient, "");
        return address(proxy);
    }
}
