pragma solidity ^0.8.20;

import { Script } from "forge-std/Script.sol";

import { Options, Upgrades } from "openzeppelin-foundry-upgrades/Upgrades.sol";
/// @notice Deploys an upgradeable Plonk Verifier Contract using the OpenZeppelin Upgrades plugin.
/// @dev The Upgrades library has a deployImplementation function which is used here

/// TODO we made these scripts in a rush for deployment and they are not very well tested
/// we plan to consolidate everything in the rust deployment scripts
contract DeployPlonkVerifierV2Script is Script {
    string public contractName = "PlonkVerifierV2.sol";

    function run() public returns (address contractAddress) {
        // get the deployer info from the environment and start broadcast as the deployer
        address deployer;
        string memory ledgerCommand = vm.envString("USE_HARDWARE_WALLET");
        if (keccak256(bytes(ledgerCommand)) == keccak256(bytes("true"))) {
            deployer = vm.envAddress("DEPLOYER_HARDWARE_WALLET_ADDRESS");
        } else {
            // get the deployer info from the environment
            string memory seedPhrase = vm.envString("MNEMONIC");
            uint32 seedPhraseOffset = uint32(vm.envUint("MNEMONIC_OFFSET"));
            (deployer,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        }

        vm.startBroadcast(deployer);
        // Deploy the library
        Options memory opts;
        address plonkVeriifer = Upgrades.deployImplementation(contractName, opts);

        vm.stopBroadcast();

        return (plonkVeriifer);
    }
}
