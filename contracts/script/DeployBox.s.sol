// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import { Script } from "forge-std/Script.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { BoxV1 } from "../src/upgradeDemo/BoxV1.sol";

contract DeployBox is Script {
    /// @notice runs the deployment
    /// @return address of the proxy
    function run() external returns (address) {
        address admin = makeAddr("admin");
        address proxy = deployBox(admin);
        return proxy;
    }

    /// @notice deploys the implementation contract and the proxy with the address of implementation
    /// @return address of the proxy
    function deployBox(address admin) public returns (address) {
        vm.startBroadcast(admin);

        BoxV1 box = new BoxV1(); //Our implementation(logic).Proxy will point here to delegate
            // call/borrow the functions

        // Encode the initializer function call
        bytes memory data = abi.encodeWithSelector(
            BoxV1(box).initialize.selector,
            msg.sender // Initial owner/admin of the contract
        );

        ERC1967Proxy proxy = new ERC1967Proxy(address(box), data);
        vm.stopBroadcast();
        return address(proxy);
    }
}
