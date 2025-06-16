import * as dotenv from "dotenv";
import { ethers } from "ethers";
import { EthersAdapter } from "@safe-global/protocol-kit";
import SafeApiKit from "@safe-global/api-kit";
import Safe from "@safe-global/protocol-kit";
import { getEnvVar, validateEthereumAddress, getSigner, createAndSignSafeTransaction } from "./utils";
const TRANSFER_OWNERSHIP_CMD = "transferOwnership" as const;

export interface TransferOwnershipData {
  proxyAddress: string;
  initData: string;
  rpcUrl: string;
  newOwner: string;
  safeAddress: string;
  useHardwareWallet: boolean;
}

async function main() {
  dotenv.config();

  try {
    const [transferOwnershipData, dryRun] = processCommandLineArguments();
    if (dryRun) {
      // Prepare the transaction data to upgrade the proxy
      const abi = ["function transferOwnership(address)"];
      // Encode the function call with the new implementation address and its init data
      transferOwnershipData.initData = new ethers.Interface(abi).encodeFunctionData("transferOwnership", [
        transferOwnershipData.newOwner,
      ]);

      console.log(JSON.stringify(transferOwnershipData));
      return;
    }

    if (!dryRun) {
      // Initialize web3 provider using the RPC URL from environment variables
      const web3Provider = new ethers.JsonRpcProvider(transferOwnershipData.rpcUrl);

      // Get the signer, this signer must be one of the signers on the Safe Multisig Wallet
      const orchestratorSigner = getSigner(web3Provider, transferOwnershipData.useHardwareWallet);

      // Set up Eth Adapter with ethers and the signer
      const ethAdapter = new EthersAdapter({
        ethers,
        signerOrProvider: orchestratorSigner,
      });

      const chainId = await ethAdapter.getChainId();
      const safeService = new SafeApiKit({ chainId });
      validateEthereumAddress(transferOwnershipData.safeAddress);
      const safeSdk = await Safe.create({ ethAdapter, safeAddress: transferOwnershipData.safeAddress });
      const orchestratorSignerAddress = await orchestratorSigner.getAddress();

      await proposeTransferOwnershipTransaction(
        safeSdk,
        safeService,
        orchestratorSignerAddress,
        transferOwnershipData.safeAddress,
        transferOwnershipData.newOwner,
        transferOwnershipData.useHardwareWallet,
      );

      console.log(
        `The other owners of the Safe Multisig wallet need to sign the transaction via the Safe UI https://app.safe.global/transactions/queue?safe=sep:${transferOwnershipData.safeAddress}`,
      );
    }
  } catch (error) {
    throw new Error("An error occurred in transferOwnership: " + error);
  }
}

export function processRustCommandLineArguments(args: string[]): [TransferOwnershipData, boolean] {
  let proxyAddress = "";
  let rpcUrl = "";
  let newOwner = "";
  let safeAddress = "";
  let useHardwareWallet = false;
  let dryRun = false;
  let initData = "";
  // Parse named flags like --proxy, --impl, --init
  const map: Record<string, string> = {};
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith("--")) {
      const key = arg.slice(2);
      if (key === "from-rust") {
        // the value is true by default and not followed by a value, thus proceed to the next iteration
        continue;
      }
      const value = args[i + 1];
      map[key] = value;
      i++; // skip next since it's the value
    }
  }

  proxyAddress = map["proxy"];
  rpcUrl = map["rpc-url"];
  newOwner = map["new-owner"];
  safeAddress = map["safe-address"];
  useHardwareWallet = map["use-hardware-wallet"] === "true";
  dryRun = map["dry-run"] === "true";
  // if any of the arguments are not provided, throw an error
  if (!newOwner || !safeAddress) {
    throw new Error("All arguments are required, --new-owner, --safe-address " + JSON.stringify(map));
  }
  validateEthereumAddress(newOwner);
  validateEthereumAddress(safeAddress);

  let transferOwnershipData: TransferOwnershipData = {
    proxyAddress: proxyAddress,
    initData: initData,
    rpcUrl: rpcUrl,
    newOwner: newOwner,
    safeAddress: safeAddress,
    useHardwareWallet: useHardwareWallet,
  };

  return [transferOwnershipData, dryRun];
}

function processCommandLineArguments(): [TransferOwnershipData, boolean] {
  let proxyAddress = "";
  let rpcUrl = "";
  let initData = "";
  let newOwner = "";
  let safeAddress = "";
  let useHardwareWallet = false;
  let dryRun = false;
  const args = process.argv.slice(2); // Remove the first two args (node command and script name)
  if (args.length === 0 || args.length < 3) {
    throw new Error(
      `Incorrect number of arguments, enter ${TRANSFER_OWNERSHIP_CMD} followed by the new implementation contract address and its init data`,
    );
  } else if (args.includes("--from-rust")) {
    return processRustCommandLineArguments(args);
  } else {
    // it's being called from elsewhere, so we need to parse the arguments differently to maintain backwards compatibility
    const command = args[0];
    if (command !== TRANSFER_OWNERSHIP_CMD) {
      throw new Error(`Only ${TRANSFER_OWNERSHIP_CMD} command is supported.`);
    }

    try {
      useHardwareWallet = getEnvVar("USE_HARDWARE_WALLET") === "true";
    } catch (error) {
      console.error("USE_HARDWARE_WALLET is not set, defaulting to false");
    }
    dryRun = args[4] === "true";
  }

  let transferOwnershipData: TransferOwnershipData = {
    proxyAddress: proxyAddress,
    initData: initData,
    rpcUrl: rpcUrl,
    newOwner: newOwner,
    safeAddress: safeAddress,
    useHardwareWallet: useHardwareWallet,
  };

  return [transferOwnershipData, dryRun];
}

/**
 * Function to propose the transaction data for upgrading the new implementation
 * @param {string} safeSDK - An instance of the Safe SDK
 * @param {string} safeService - An instance of the Safe Service
 * @param {string} signerAddress - The address of the address signing the transaction
 * @param {UpgradeData} upgradeData - The data for the upgrade
 */
async function proposeTransferOwnershipTransaction(
  safeSDK: Safe,
  safeService: SafeApiKit,
  signerAddress: string,
  safeAddress: string,
  newOwner: string,
  useHardwareWallet: boolean,
) {
  // Prepare the transaction data to upgrade the proxy
  const abi = ["function transferOwnership(address)"];
  // Encode the function call with the new implementation address and its init data
  const data = new ethers.Interface(abi).encodeFunctionData("transferOwnership", [newOwner]);

  // Create & Sign the Safe Transaction Object
  const { safeTransaction, safeTxHash, senderSignature } = await createAndSignSafeTransaction(
    safeSDK,
    safeAddress,
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

if (require.main === module) {
  main();
}
