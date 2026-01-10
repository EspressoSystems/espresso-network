pragma solidity ^0.8.20;

import { Script } from "forge-std/Script.sol";
import { Upgrades, Options } from "openzeppelin-foundry-upgrades/Upgrades.sol";
import { FeeContract as FC } from "../src/FeeContract.sol";

/// @notice Deploys an upgradeable Fee Contract using the OpenZeppelin Upgrades plugin.
contract DeployFeeContractScript is Script {
    string internal contractName = vm.envString("FEE_CONTRACT_ORIGINAL_NAME");

    error OwnerNotAsExpected(address expectedOwner, address currentOwner);

    /// @dev Deploys both the proxy and the implementation contract.
    /// The proxy admin is set as the owner of the contract upon deployment.
    /// The `owner` parameter should be the address of the multisig wallet to ensure proper
    /// ownership management.
    /// @param owner The address that will be set as the owner of the proxy (typically a multisig
    /// wallet).
    function run(address owner)
        public
        returns (address payable proxy, address implementationAddress)
    {
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("DEPLOYER_MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("DEPLOYER_MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);

        address proxyAddress =
            Upgrades.deployUUPSProxy(contractName, abi.encodeCall(FC.initialize, (owner)));

        FC feeContractProxy = FC(payable(proxyAddress));

        // verify post deployment details
        if (feeContractProxy.owner() != owner) {
            revert OwnerNotAsExpected(owner, feeContractProxy.owner());
        }

        // Get the implementation address
        implementationAddress = Upgrades.getImplementationAddress(proxyAddress);

        vm.stopBroadcast();

        return (payable(proxyAddress), implementationAddress);
    }
}

/// @notice Upgrades the fee contract first by deploying the new implementation
/// and then executing the upgrade via the Safe Multisig wallet using the SAFE SDK.
contract UpgradeFeeContractScript is Script {
    string internal originalContractName = vm.envString("FEE_CONTRACT_ORIGINAL_NAME");
    string internal upgradeContractName = vm.envString("FEE_CONTRACT_UPGRADE_NAME");

    /// @dev This function first deploys the new implementation contract using the deployer wallet.
    /// It then uses the SAFE SDK via an ffi command to perform the upgrade through a Safe Multisig
    /// wallet.
    function run() public returns (address implementationAddress, bytes memory result) {
        Options memory opts;
        opts.referenceContract = originalContractName;

        // validate that the new implementation contract is upgrade safe
        Upgrades.validateUpgrade(upgradeContractName, opts);

        // get the deployer to deploy the new implementation contract
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("DEPLOYER_MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("DEPLOYER_MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);

        // deploy the new implementation contract
        FC implementationContract = new FC();

        vm.stopBroadcast();

        //replace with something like this if there is some initiation function to call
        // abi.encodeWithSignature("setNewField(uint256)", 2);
        bytes memory initData = "";

        // call upgradeToAndCall command so that the proxy can be upgraded to call from the new
        // implementation above and
        // execute the command via the Safe Multisig wallet
        string[] memory cmds = new string[](3);
        cmds[0] = "bash";
        cmds[1] = "-c";
        cmds[2] = string(
            abi.encodePacked(
                "source .env.contracts && ts-node contracts/script/multisigTransactionProposals/safeSDK/upgradeProxy.ts upgradeProxy ",
                vm.toString(vm.envAddress("FEE_CONTRACT_PROXY_ADDRESS")),
                " ",
                vm.toString(address(implementationContract)),
                " ",
                vm.toString(initData)
            )
        );

        result = vm.ffi(cmds);

        return (address(implementationContract), result);
    }
}
