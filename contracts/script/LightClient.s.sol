pragma solidity ^0.8.20;

import { Script } from "forge-std/Script.sol";

import {
    Defender,
    ApprovalProcessResponse,
    ProposeUpgradeResponse
} from "openzeppelin-foundry-upgrades/Defender.sol";
import { Upgrades, Options } from "openzeppelin-foundry-upgrades/Upgrades.sol";
import { LightClient as LC } from "../src/LightClient.sol";
import { UtilsScript } from "./Utils.s.sol";
import { LightClientV2 as LCV2 } from "../test/LightClientV2.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

/// @notice use this script to deploy the upgradeable light client contract
/// without openzepelin defender
/// @dev be sure to pass the multisig wallet as the owner of this contract
contract LightClientDeployScript is Script {
    string public contractName = "LightClient.sol";

    function run(uint32 numBlocksPerEpoch, uint32 numInitValidators, address owner)
        public
        returns (
            address proxyAddress,
            address implementationAddress,
            LC.LightClientState memory state
        )
    {
        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = vm.toString(numBlocksPerEpoch);
        cmds[3] = vm.toString(uint256(numInitValidators));

        bytes memory result = vm.ffi(cmds);
        (state,,) = abi.decode(result, (LC.LightClientState, bytes32, bytes32));

        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        proxyAddress = Upgrades.deployUUPSProxy(
            contractName, abi.encodeCall(LC.initialize, (state, numBlocksPerEpoch, owner))
        );

        // Get the implementation address
        implementationAddress = Upgrades.getImplementationAddress(proxyAddress);

        vm.stopBroadcast();

        return (proxyAddress, implementationAddress, state);
    }
}

/// @notice upgrade the LightClient contract by deploying the new implementation using the deployer
/// and then
/// using the SAFE SDK to call the upgrade via the Safe Multisig wallet
contract LightClientContractUpgradeScript is Script {
    string internal originalContractName = "LightClient.sol";
    string internal upgradeContractName = vm.envString("LIGHT_CLIENT_CONTRACT_UPGRADE_NAME");

    function run() public returns (address implementationAddress, bytes memory result) {
        Options memory opts;
        opts.referenceContract = originalContractName;

        // validate that the new implementation contract is upgrade safe
        Upgrades.validateUpgrade(upgradeContractName, opts);

        uint256 deployerPrivateKey = vm.envUint("DEPLOYER_PRIVATE_KEY");
        vm.startBroadcast(deployerPrivateKey);

        // deploy the new implementation contract
        LCV2 implementationContract = new LCV2();

        vm.stopBroadcast();

        bytes memory initData = abi.encodeWithSignature("setNewField(uint256)", 2);

        // call upgradeToAndCall command so that the proxy can be upgraded to call from the new
        // implementation above and
        // execute the command via the Safe Multisig wallet
        string[] memory cmds = new string[](3);
        cmds[0] = "bash";
        cmds[1] = "-c";
        cmds[2] = string(
            abi.encodePacked(
                "source .env.contracts && ts-node contracts/script/multisigTransactionProposals/safeSDK/upgradeProxy.ts upgradeProxy ",
                vm.toString(vm.envAddress("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS")),
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

contract LightClientDefenderDeployScript is Script {
    string public contractName = "LightClient.sol";
    UtilsScript public utils = new UtilsScript();
    uint256 public contractSalt = uint256(vm.envInt("LIGHT_CLIENT_SALT"));

    function run()
        public
        returns (address proxy, address multisig, LC.LightClientState memory state)
    {
        // TODO for a production deployment provide the right genesis state and value
        uint32 numBlocksPerEpoch = 5;
        uint32 numInitValidators = 1;

        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = vm.toString(numBlocksPerEpoch);
        cmds[3] = vm.toString(uint256(numInitValidators));

        bytes memory result = vm.ffi(cmds);
        (state,,) = abi.decode(result, (LC.LightClientState, bytes32, bytes32));

        ApprovalProcessResponse memory upgradeApprovalProcess = Defender.getUpgradeApprovalProcess();
        multisig = upgradeApprovalProcess.via;

        if (upgradeApprovalProcess.via == address(0)) {
            revert(
                string.concat(
                    "Upgrade approval process with id ",
                    upgradeApprovalProcess.approvalProcessId,
                    " has no assigned address"
                )
            );
        }

        Options memory opts;
        opts.defender.useDefenderDeploy = true;
        opts.defender.salt = bytes32(abi.encodePacked(contractSalt));

        proxy = Upgrades.deployUUPSProxy(
            contractName, abi.encodeCall(LC.initialize, (state, numBlocksPerEpoch, multisig)), opts
        );

        //generate the file path, file output and write to the file
        (string memory filePath, string memory fileData) = utils.generateProxyDeploymentOutput(
            contractName,
            contractSalt,
            proxy,
            multisig,
            upgradeApprovalProcess.approvalProcessId,
            upgradeApprovalProcess.viaType
        );
        utils.writeJson(filePath, fileData);

        //generate the salt history file path,  output and write to the file
        (string memory saltFilePath, string memory saltFileData) =
            utils.generateSaltOutput(contractName, contractSalt);
        utils.writeJson(saltFilePath, saltFileData);

        return (proxy, multisig, state);
    }
}

contract LightClientDefenderUpgradeScript is Script {
    string public originalContractName = "LightClient.sol";
    string public upgradeContractName = vm.envString("LIGHT_CLIENT_UPGRADE_NAME");
    uint256 public contractSalt = uint256(vm.envInt("LIGHT_CLIENT_SALT"));
    UtilsScript public utils = new UtilsScript();

    function run() public returns (string memory proposalId, string memory proposalUrl) {
        //get the previous salt from the salt history - this assumes there was first a deployment
        (string memory saltFilePath,) = utils.generateSaltFilePath(originalContractName);
        (, string memory saltData) = utils.readFile(saltFilePath);
        uint256 prevContractSalt = vm.parseJsonUint(saltData, ".previousSalt");

        (string memory filePath,) =
            utils.generateDeploymentFilePath(originalContractName, prevContractSalt);

        //read the deployment file from the previous deployment to get the proxyAddress & multisig
        // used for deployment
        (, string memory result) = utils.readFile(filePath);
        address proxyAddress = vm.parseJsonAddress(result, ".proxyAddress");
        address multisig = vm.parseJsonAddress(result, ".multisig");

        //set openzeppelin defender options for the deployment
        Options memory opts;
        opts.defender.useDefenderDeploy = true;
        opts.defender.salt = bytes32(contractSalt);
        opts.referenceContract = originalContractName;

        // propose the upgrade via openzeppelin defender
        ProposeUpgradeResponse memory response =
            Defender.proposeUpgrade(proxyAddress, upgradeContractName, opts);
        string memory responseProposalId = response.proposalId;
        string memory responseProposalUrl = response.url;

        //generate the file path, file output (deployment info) and write to the file
        (string memory upgradeFilePath, string memory fileData) = utils.generateUpgradeOutput(
            originalContractName,
            contractSalt,
            upgradeContractName,
            proxyAddress,
            multisig,
            responseProposalId,
            responseProposalUrl
        );
        utils.writeJson(upgradeFilePath, fileData);

        //generate the salt history file path,  output and write to the file
        string memory saltFileData;
        (saltFilePath, saltFileData) = utils.generateSaltOutput(originalContractName, contractSalt);
        utils.writeJson(saltFilePath, saltFileData);

        return (responseProposalId, responseProposalUrl);
    }
}

contract DeployLightClientContractScriptWithoutMultiSig is Script {
    function run(uint32 numBlocksPerEpoch, uint32 numInitValidators)
        external
        returns (address payable proxyAddress, address admin, LC.LightClientState memory)
    {
        // TODO for a production deployment provide the right genesis state and value

        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = vm.toString(numBlocksPerEpoch);
        cmds[3] = vm.toString(uint256(numInitValidators));

        bytes memory result = vm.ffi(cmds);
        (LC.LightClientState memory state,,) =
            abi.decode(result, (LC.LightClientState, bytes32, bytes32));

        return deployContract(state, numBlocksPerEpoch);
    }

    function runDemo(uint32 numBlocksPerEpoch)
        external
        returns (address payable proxyAddress, address admin, LC.LightClientState memory)
    {
        string[] memory cmds = new string[](1);
        cmds[0] = "gen-demo-genesis";

        bytes memory result = vm.ffi(cmds);
        LC.LightClientState memory state = abi.decode(result, (LC.LightClientState));

        return deployContract(state, numBlocksPerEpoch);
    }

    /// @notice deploys the impl, proxy & initializes the impl
    /// @return proxyAddress The address of the proxy
    /// @return admin The address of the admin

    function deployContract(LC.LightClientState memory state, uint32 numBlocksPerEpoch)
        private
        returns (address payable proxyAddress, address admin, LC.LightClientState memory)
    {
        string memory seedPhrase = vm.envString("MNEMONIC");
        (admin,) = deriveRememberKey(seedPhrase, 0);
        vm.startBroadcast(admin);

        LC lightClientContract = new LC();

        // Encode the initializer function call
        bytes memory data = abi.encodeWithSignature(
            "initialize((uint64,uint64,uint256,uint256,uint256,uint256,uint256,uint256),uint32,address)",
            state,
            numBlocksPerEpoch,
            admin
        );

        // our proxy
        ERC1967Proxy proxy = new ERC1967Proxy(address(lightClientContract), data);
        vm.stopBroadcast();

        proxyAddress = payable(address(proxy));

        return (proxyAddress, admin, state);
    }
}
