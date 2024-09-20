import dotenv from "dotenv";
import { ethers } from "ethers";
import { EthersAdapter } from "@safe-global/protocol-kit";
import SafeApiKit from "@safe-global/api-kit";
import Safe from "@safe-global/protocol-kit";
import { getEnvVar, createSafeTransactionData, validateEthereumAddress, getSigner } from "./utils";
const SET_PROVER_CMD = "setProver" as const;
const DISABLE_PROVER_CMD = "disableProver" as const;

// declaring the type returned by the createTransaction method in the safe package locally (since the return type isn't exposed) so that if it's updated, it's reflected here too
type LocalSafeTransaction = Awaited<ReturnType<Safe["createTransaction"]>>;

async function main() {
  dotenv.config();

  try {
    const command = processCommandLineArguments();

    /**TODO
     * change from RPC_URL to production URL when deploying to production
     */
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
      );
    } else if (command === DISABLE_PROVER_CMD) {
      console.log(`${command}`);
      await proposeDisableProverTransaction(safeSdk, safeService, orchestratorSignerAddress, safeAddress);
    }

    console.log(
      `The other owners of the Safe Multisig wallet need to sign the transaction via the Safe UI https://app.safe.global/transactions/queue?safe=sep:${safeAddress}`,
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
) {
  // Prepare the transaction data to set the permissioned prover
  let data = createPermissionedProverTxData(proverAddress);

  const contractAddress = getEnvVar("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS");
  validateEthereumAddress(contractAddress);

  // Create the Safe Transaction Object
  const safeTransaction = await createSafeTransaction(safeSDK, contractAddress, data, "0");

  // Get the transaction hash and sign the transaction
  const safeTxHash = await safeSDK.getTransactionHash(safeTransaction);

  // Sign the transaction with orchestrator signer that was specified when we created the safeSDK
  const senderSignature = await safeSDK.signHash(safeTxHash);

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
) {
  // Prepare the transaction data to disable permissioned prover mode
  let data = createDisablePermissionedProverTxData();

  const contractAddress = getEnvVar("LIGHT_CLIENT_CONTRACT_PROXY_ADDRESS");
  validateEthereumAddress(contractAddress);

  // Create the Safe Transaction Object
  const safeTransaction = await createSafeTransaction(safeSDK, contractAddress, data, "0");

  // Get the transaction hash and sign the transaction
  const safeTxHash = await safeSDK.getTransactionHash(safeTransaction);

  // Sign the transaction with orchestrator signer that was specified when we created the safeSDK
  const senderSignature = await safeSDK.signHash(safeTxHash);

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
 * Function to create the transaction data for disabling permissioned prover mode
 * @returns {string} - Encoded transaction data
 */
function createDisablePermissionedProverTxData(): string {
  // Define the ABI of the function to be called
  const abi = ["function disablePermissionedProverMode()"];

  // Encode the function call with the provided prover address
  const data = new ethers.Interface(abi).encodeFunctionData("disablePermissionedProverMode", []);
  return data; // Return the encoded transaction data
}

/**
 * Creates a Safe transaction object
 *
 * @param {Safe} safeSDK - An instance of the Safe SDK
 * @param {string} contractAddress - The address of the contract to interact with
 * @param {string} data - The data payload for the transaction
 * @param {string} value - The value to be sent with the transaction
 * @returns {Promise<any>} - A promise that resolves to the Safe transaction object
 */
async function createSafeTransaction(
  safeSDK: Safe,
  contractAddress: string,
  data: string,
  value: string,
): Promise<LocalSafeTransaction> {
  // Prepare the safe transaction data with the contract address, data, and value
  let safeTransactionData = createSafeTransactionData(contractAddress, data, value);

  if (getEnvVar("USE_HARDWARE_WALLET")) {
    console.log(`Please sign the message on your connected Ledger device`);
  }

  // Create the safe transaction using the Safe SDK
  const safeTransaction = await safeSDK.createTransaction({ transactions: [safeTransactionData] });
  return safeTransaction;
}

main();
