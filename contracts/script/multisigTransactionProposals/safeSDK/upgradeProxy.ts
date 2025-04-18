import dotenv from "dotenv";
import { ethers } from "ethers";
import { EthersAdapter } from "@safe-global/protocol-kit";
import SafeApiKit from "@safe-global/api-kit";
import Safe from "@safe-global/protocol-kit";
import { getEnvVar, validateEthereumAddress, getSigner, createAndSignSafeTransaction } from "./utils";
const UPGRADE_PROXY_CMD = "upgradeProxy" as const;

export interface UpgradeData {
  proxyAddress: string;
  implementationAddress: string;
  initData: string;
}

async function main() {
  dotenv.config();

  try {
    const upgradeData: UpgradeData = processCommandLineArguments();

    // Initialize web3 provider using the RPC URL from environment variables
    const web3Provider = new ethers.JsonRpcProvider(getEnvVar("RPC_URL"));

    // Get the signer, this signer must be one of the signers on the Safe Multisig Wallet
    const orchestratorSigner = getSigner(web3Provider);

    // Set up Eth Adapter with ethers and the signer
    const ethAdapter = new EthersAdapter({
      ethers,
      signerOrProvider: orchestratorSigner,
    });

    const chainId = await ethAdapter.getChainId();
    const safeService = new SafeApiKit({ chainId });
    const safeAddress = getEnvVar("SAFE_MULTISIG_ADDRESS");
    validateEthereumAddress(safeAddress);
    const safeSdk = await Safe.create({ ethAdapter, safeAddress });
    const orchestratorSignerAddress = await orchestratorSigner.getAddress();

    await proposeUpgradeTransaction(safeSdk, safeService, orchestratorSignerAddress, safeAddress, upgradeData);

    console.log(
      `The other owners of the Safe Multisig wallet need to sign the transaction via the Safe UI https://app.safe.global/transactions/queue?safe=sep:${safeAddress}`,
    );
  } catch (error) {
    throw new Error("An error occurred: " + error);
  }
}

function processCommandLineArguments(): UpgradeData {
  const args = process.argv.slice(2); // Remove the first two args (node command and script name)
  if (args.length === 0) {
    console.log("No commands provided.");
    throw new Error(
      `No commands provided, enter ${UPGRADE_PROXY_CMD} followed by the new implementation contract address and its init data`,
    );
  } else if (args.length < 3) {
    throw new Error(
      `Incorrect number of arguments, enter ${UPGRADE_PROXY_CMD} followed by the new implementation contract address and its init data`,
    );
  }

  const command = args[0];
  if (command !== UPGRADE_PROXY_CMD) {
    throw new Error(`Only ${UPGRADE_PROXY_CMD} command is supported.`);
  }
  const proxyAddress = args[1];
  const implementationAddress = args[2];
  validateEthereumAddress(implementationAddress);
  const initData = args[3];

  return { proxyAddress: proxyAddress, implementationAddress: implementationAddress, initData: initData };
}

/**
 * Function to propose the transaction data for upgrading the new implementation
 * @param {string} safeSDK - An instance of the Safe SDK
 * @param {string} safeService - An instance of the Safe Service
 * @param {string} signerAddress - The address of the address signing the transaction
 * @param {string} safeAddress - The address of the Safe multisig wallet
 * @param {string} newContractAddress - The address of the new implementation
 * @param {string} initData - The initialization data for the new implementation, may be '0x' if there is not initialization required
 */
export async function proposeUpgradeTransaction(
  safeSDK: Safe,
  safeService: SafeApiKit,
  signerAddress: string,
  safeAddress: string,
  upgradeData: UpgradeData,
) {
  // Prepare the transaction data to upgrade the proxy
  const abi = ["function upgradeToAndCall(address,bytes)"];
  // Encode the function call with the new implementation address and its init data
  const data = new ethers.Interface(abi).encodeFunctionData("upgradeToAndCall", [
    upgradeData.implementationAddress,
    upgradeData.initData,
  ]);

  // Create & Sign the Safe Transaction Object
  const { safeTransaction, safeTxHash, senderSignature } = await createAndSignSafeTransaction(
    safeSDK,
    upgradeData.proxyAddress,
    data,
  );

  // Propose the transaction which can be signed by other owners via the Safe UI
  await safeService.proposeTransaction({
    safeAddress: safeAddress,
    safeTransactionData: safeTransaction.data,
    safeTxHash: safeTxHash,
    senderAddress: signerAddress,
    senderSignature: senderSignature.data,
  });
}

main();
