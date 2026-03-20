// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Script } from "forge-std/Script.sol";
// import {DevOpsTools} from "../lib/foundry-devops/src/DevOpsTools.sol";
import { DemoBoxV2 } from "../../demo/upgradeDemo/DemoBoxV2.sol";
import { DemoBoxV1 } from "../../demo/upgradeDemo/DemoBoxV1.sol";

contract UpgradeBoxScript is Script {
    /// @notice runs the upgrade
    /// @param mostRecentlyDeployedProxy address of deployed proxy
    /// @return address of the proxy
    /// TODO get the most recent deployment from the devops tooling
    function run(address admin, address mostRecentlyDeployedProxy) external returns (address) {
        DemoBoxV2 newAddy = new DemoBoxV2(); //gets the address of the new implementation
        address proxy = upgradeBox(admin, mostRecentlyDeployedProxy, address(newAddy));
        return proxy;
    }

    /// @notice upgrades the box by calling the upgrade function the implementation contract via
    /// the proxy
    /// @param proxyAddress address of proxy
    /// @param newBox address of new implementation
    /// @return address of the proxy
    function upgradeBox(address admin, address proxyAddress, address newBox)
        public
        returns (address)
    {
        DemoBoxV1 proxy = DemoBoxV1(proxyAddress); //make the function call on the previous
        // implementation
        vm.prank(admin);
        proxy.upgradeToAndCall(newBox, ""); //proxy address now points to the new implementation
        return address(proxy);
    }
}
