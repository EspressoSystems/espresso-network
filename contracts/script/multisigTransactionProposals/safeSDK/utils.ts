import { ethers } from "ethers"; // Import ethers from the ethers library
import { LedgerSigner } from "@ethers-ext/signer-ledger";
import HIDTransport from "@ledgerhq/hw-transport-node-hid";
import Safe from "@safe-global/protocol-kit";
// declaring types locally (since the return type isn't exposed) so that if it's updated, it's reflected here too
type LocalSafeTransaction = Awaited<ReturnType<Safe["createTransaction"]>>;
type SafeSignature = Awaited<ReturnType<Safe["signHash"]>>;

/**
 * Function to check if a given string is a valid Ethereum address
 * @param {string} address - The Ethereum address to validate
 * @throws {Error} - Throws an error if the address is invalid and doesn't follow Ethereum address standards
 */
export function validateEthereumAddress(address: string) {
  if (!ethers.isAddress(address)) {
    throw new Error(`Invalid Ethereum address format: ${address}`); // Throw an error if the address is invalid
  }
}

/**
 * Function to get the value of an environment variable from the .env file
 * @param {string} name - The name of the environment variable to retrieve
 * @returns {string} - Returns the value of the environment variable
 */
export function getEnvVar(name: string): string {
  const value = process.env[name]; // Retrieve the environment variable value
  // Check if the environment variable is undefined or empty
  if (value === undefined || value === "") {
    throw new Error(`Environment variable ${name} is not set`); // Throw an error if the environment variable is not set
  }
  return value; // Return the value of the environment variable
}

/**
 * Function to create safe transaction data to be used with the safe SDK
 * @param {string} to - The destination address for the transaction
 * @param {string} data - The contract data to be sent
 * @param {number} value - The value to be sent
 * @returns {object} - Returns the safe transaction data object
 */
export function createSafeTransactionData(to: string, data: string, value: string) {
  // Check if the destination address is specified
  if (to == "") {
    throw new Error("must specify destination address"); // Throw an error if the destination address is not specified
  }
  // Check if both data and value are empty
  if (data == "" && value == "") {
    throw new Error("Either the contract data or value to be sent must be specified"); // Throw an error if both data and value are empty
  }
  validateEthereumAddress(to); // Validate the destination address
  // Create the safe transaction data object
  const safeTransactionData = {
    to: to,
    data: data,
    value: value,
  };
  return safeTransactionData; // Return the safe transaction data object
}

/**
 * Function to check if a given string is a valid Ethereum address
 * @param {string} address - The Ethereum address to validate
 * @throws {Error} - Throws an error if the address is invalid and doesn't follow Ethereum address standards
 */
export function getSigner(web3Provider: ethers.Provider): ethers.Signer {
  let orchestratorSigner;
  const use_hardware_wallet = getEnvVar("USE_HARDWARE_WALLET");
  if (use_hardware_wallet == "true") {
    // Create a signer using the ledger
    orchestratorSigner = new LedgerSigner(HIDTransport, web3Provider);
  } else {
    // Create a signer using the orchestrator's private key and the web3 provider
    orchestratorSigner = new ethers.Wallet(getEnvVar("SAFE_ORCHESTRATOR_PRIVATE_KEY"), web3Provider);
  }

  return orchestratorSigner;
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
export async function createSafeTransaction(
  safeSDK: Safe,
  contractAddress: string,
  data: string,
  value: string,
): Promise<LocalSafeTransaction> {
  // Prepare the safe transaction data with the contract address, data, and value
  let safeTransactionData = createSafeTransactionData(contractAddress, data, value);
  if (getEnvVar("USE_HARDWARE_WALLET") == "true") {
    console.log(`Please sign the message on your connected Ledger device`);
  }

  // Create the safe transaction using the Safe SDK
  const safeTransaction = await safeSDK.createTransaction({ transactions: [safeTransactionData] });
  console.log("Safe Transaction Data hex: ", data);

  return safeTransaction;
}

/**
 * Creates and signs a Safe transaction object
 *
 * @param {Safe} safeSDK - An instance of the Safe SDK used to interact with the Gnosis Safe
 * @param {string} contractAddress - The address of the contract to interact with
 * @param {string} data - The data payload encoded with the ABI for the transaction
 * @returns {Promise<{ safeTransaction: SafeTransaction; safeTxHash: string; senderSignature: SafeSignature }>} -
 *          A promise that resolves to an object containing the Safe transaction, transaction hash, and the signature
 */
export async function createAndSignSafeTransaction(
  safeSDK: Safe,
  contractAddress: string,
  data: string,
): Promise<{ safeTransaction: LocalSafeTransaction; safeTxHash: string; senderSignature: SafeSignature }> {
  validateEthereumAddress(contractAddress);

  // Create the Safe Transaction Object
  const safeTransaction = await createSafeTransaction(safeSDK, contractAddress, data, "0");

  // Get the transaction hash
  const safeTxHash = await safeSDK.getTransactionHash(safeTransaction);

  // Sign the transaction
  const senderSignature = await safeSDK.signHash(safeTxHash);

  return { safeTransaction, safeTxHash, senderSignature };
}
