// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.20;

import { Script } from "forge-std/Script.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { EspToken } from "../src/EspTokenV2.sol";

/// @notice Deploys an upgradeable Fee Contract using the OpenZeppelin Upgrades plugin.
///
/// To deploy with the ledger, add the --ledger flag to the forge script command:
///
/// forge script --ledger DeployEspTokenScript --broadcast
///
/// To use a mnemonic
/// echo test test test test test test test test test test test junk | tr -d '\n' > .mnemonic
/// forge script --mnemonics .mnemonic DeployEspTokenScript --broadcast
contract DeployEspTokenScript is Script {
    string internal contractName = "EspTokenV2.sol";

    error OwnerNotAsExpected(address expectedOwner, address currentOwner);

    /// @dev Deploys both the proxy and the implementation contract.
    /// The proxy admin is set as the owner of the contract upon deployment.
    function run()
        public
        returns (
            address proxyAddress,
            address owner,
            address initialRecipient,
            uint256 initialSupply,
            string memory name,
            string memory symbol
        )
    {
        vm.startBroadcast();

        owner = vm.envAddress("ESP_TOKEN_INITIAL_OWNER");
        initialRecipient = vm.envAddress("ESP_TOKEN_INITIAL_RECIPIENT");
        initialSupply = vm.envUint("ESP_TOKEN_INITIAL_SUPPLY");
        name = vm.envString("ESP_TOKEN_NAME");
        symbol = vm.envString("ESP_TOKEN_SYMBOL");

        EspToken tokenImpl = new EspToken();

        // Encode the initializer function call
        bytes memory data = abi.encodeWithSignature(
            "initialize(address,address,uint256,string,string)",
            owner,
            initialRecipient,
            initialSupply,
            name,
            symbol
        );

        // our proxy
        ERC1967Proxy proxy = new ERC1967Proxy(address(tokenImpl), data);
        vm.stopBroadcast();

        // Cast the proxy to the EspToken type
        proxyAddress = address(proxy);
        EspToken token = EspToken(proxyAddress);

        verifyDeployment(token, owner, initialRecipient, initialSupply, name, symbol);
    }

    function verifyDeployment(
        EspToken token,
        address owner,
        address initialRecipient,
        uint256 initialSupply,
        string memory name,
        string memory symbol
    ) public {
        // Check the owner
        if (token.owner() != owner) {
            revert OwnerNotAsExpected(owner, token.owner());
        }

        // Check the name
        if (keccak256(bytes(token.name())) != keccak256(bytes(name))) {
            revert("Token name does not match expected value");
        }

        // Check the symbol
        if (keccak256(bytes(token.symbol())) != keccak256(bytes(symbol))) {
            revert("Token symbol does not match expected value");
        }

        // Check the total supply
        if (token.totalSupply() != initialSupply * 10 ** token.decimals()) {
            revert("Token total supply does not match expected value");
        }

        // Check the initial recipient balance
        if (token.balanceOf(initialRecipient) != initialSupply * 10 ** token.decimals()) {
            revert("Initial recipient balance does not match expected value");
        }
    }
}
