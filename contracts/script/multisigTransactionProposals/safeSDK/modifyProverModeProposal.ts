import dotenv from "dotenv";
import { ethers } from "ethers";
import { EthersAdapter } from "@safe-global/protocol-kit";
import SafeApiKit from "@safe-global/api-kit";
import Safe from "@safe-global/protocol-kit";
import { getEnvVar, validateEthereumAddress, getSigner, createAndSignSafeTransaction } from "./utils";
const SET_PROVER_CMD = "setProver" as const;
const DISABLE_PROVER_CMD = "disableProver" as const;

async function main() {
  dotenv.config();

  try {
    const command = processCommandLineArguments();

    // Initialize web3 provider using the RPC URL from environment variables
    const web3Provider = new ethers.JsonRpcProvider(getEnvVar("RPC_URL"));
    let useHardwareWallet = false;
    try {
      useHardwareWallet = getEnvVar("USE_HARDWARE_WALLET") === "true";
    } catch (error) {
      console.error("USE_HARDWARE_WALLET is not set, defaulting to false");
    }
    // Get the signer, this signer must be one of the signers on the Safe Multisig Wallet
    const orchestratorSigner = getSigner(web3Provider, useHardwareWallet);

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

    if (command === SET_PROVER_CMD) {
      console.log(`${command}`);
      const permissionedProverAddress = getEnvVar("APPROVED_PROVER_ADDRESS");
      validateEthereumAddress(permissionedProverAddress);

      await proposeSetProverTransaction(
        safeSdk,
        safeService,
        orchestratorSignerAddress,
        safeAddress,
        permissionedProverAddress,
        useHardwareWallet,
      );
    } else if (command === DISABLE_PROVER_CMD) {
      console.log(`${command}`);
      await proposeDisableProverTransaction(
        safeSdk,
        safeService,
        orchestratorSignerAddress,
        safeAddress,
        useHardwareWallet,
      );
    }

    console.log(
      `Proposal created successfully. The other owners of the Safe Multisig wallet need to sign the transaction via the Safe UI https://app.safe.global/transactions/queue?safe=sep:${safeAddress}`,
    );
  } catch (error) {
    throw new Error("An error occurred: " + error);
  }
}

function processCommandLineArguments() {
  const args = process.argv.slice(2); // Remove the first two args (node command and script name)
  if (args.length === 0) {
    console.log("No commands provided.");
    throw new Error(`No commands provided, either ${SET_PROVER_CMD} or ${DISABLE_PROVER_CMD}`);
  }

  const command = args[0];
  if (command !== SET_PROVER_CMD && command !== DISABLE_PROVER_CMD) {
    throw new Error(`Unrecognized command ${command} provided, either ${SET_PROVER_CMD} or ${DISABLE_PROVER_CMD}`);
  }

  return command;
}

/**
 * Function to propose the transaction data for setting the permissioned prover
 * @param {string} safeSDK - An instance of the Safe SDK
 * @param {string} safeService - An instance of the Safe Service
 * @param {string} signerAddress - The address of the address signing the transaction
 * @param {string} safeAddress - The address of the Safe multisig wallet
 * @param {string} proverAddress - The address of the permissioned prover
 */
export async function proposeSetProverTransaction(
  safeSDK: Safe,
  safeService: SafeApiKit,
  signerAddress: string,
  safeAddress: string,
  proverAddress: string,
  useHardwareWallet: boolean,
) {
  // Prepare the transaction data to set the permissioned prover
  let data = createPermissionedProverTxData(proverAddress);

  const contractAddress = getEnvVar("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS");
  validateEthereumAddress(contractAddress);

  // Create & Sign the Safe Transaction Object
  const { safeTransaction, safeTxHash, senderSignature } = await createAndSignSafeTransaction(
    safeSDK,
    contractAddress,
    data,
    useHardwareWallet,
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

/**
 * Function to create the transaction data for setting the permissioned prover
 * @param {string} proverAddress - The address of the permissioned prover
 * @returns {string} - Encoded transaction data
 */
function createPermissionedProverTxData(proverAddress: string): string {
  // Define the ABI of the function to be called
  const abi = ["function setPermissionedProver(address)"];

  // Encode the function call with the provided prover address
  const data = new ethers.Interface(abi).encodeFunctionData("setPermissionedProver", [proverAddress]);
  return data; // Return the encoded transaction data
}

/**
 * Function to propose the transaction data for disabling permissioned prover mode
 * @param {Safe} safeSDK - An instance of the Safe SDK
 * @param {SafeApiKit} safeService - An instance of the Safe Service
 * @param {string} signerAddress - The address of the address signing the transaction
 * @param {string} safeAddress - The address of the Safe multisig wallet
 */
export async function proposeDisableProverTransaction(
  safeSDK: Safe,
  safeService: SafeApiKit,
  signerAddress: string,
  safeAddress: string,
  useHardwareWallet: boolean,
) {
  // Prepare the transaction data to disable permissioned prover mode
  // Define the ABI of the function to be called
  const abi = ["function disablePermissionedProverMode()"];

  // Encode the function call with the provided prover address
  const data = new ethers.Interface(abi).encodeFunctionData("disablePermissionedProverMode", []);

  const contractAddress = getEnvVar("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS");
  validateEthereumAddress(contractAddress);

  // Create & Sign the Safe Transaction Object
  const { safeTransaction, safeTxHash, senderSignature } = await createAndSignSafeTransaction(
    safeSDK,
    contractAddress,
    data,
    useHardwareWallet,
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
