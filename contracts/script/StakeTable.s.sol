pragma solidity ^0.8.0;

import { Script } from "forge-std/Script.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
/// @notice Deploys the upgradable light client contract
/// the admin is not a multisig wallet but is the same as the associated mnemonic
/// used in staging deployments only

contract DeployStakeTableScript is Script {
    function run() external returns (address payable proxyAddress, address admin) {
        string memory seedPhrase = vm.envString("DEPLOYER_MNEMONIC");
        uint32 seedPhraseOffset = uint32(vm.envUint("DEPLOYER_MNEMONIC_OFFSET"));
        (admin,) = deriveRememberKey(seedPhrase, seedPhraseOffset);
        vm.startBroadcast(admin);

        address tokenAddress = vm.envAddress("TOKEN_CONTRACT_PROXY_ADDRESS");
        address lightClientAddress = vm.envAddress("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS");
        uint256 exitEscrowPeriod = vm.envUint("EXIT_ESCROW_PERIOD");
        address initialOwner = vm.envAddress("INITIAL_OWNER");

        StakeTable stakeTableContract = new StakeTable();

        // Encode the initializer function call
        bytes memory data = abi.encodeWithSignature(
            "initialize(address,address,uint256,address)",
            tokenAddress,
            lightClientAddress,
            exitEscrowPeriod,
            initialOwner
        );

        // our proxy
        ERC1967Proxy proxy = new ERC1967Proxy(address(stakeTableContract), data);
        vm.stopBroadcast();

        proxyAddress = payable(address(proxy));

        return (proxyAddress, address(stakeTableContract));
    }
}
