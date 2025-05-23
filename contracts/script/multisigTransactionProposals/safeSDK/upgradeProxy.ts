import * as dotenv from "dotenv";
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
  rpcUrl: string;
  safeAddress: string;
  useHardwareWallet: boolean;
}

async function main() {
  dotenv.config();

  try {
    const [upgradeData, dryRun] = processCommandLineArguments();
    if (!upgradeData.rpcUrl) {
      upgradeData.rpcUrl = getEnvVar("RPC_URL");
    }
    if (!upgradeData.safeAddress) {
      upgradeData.safeAddress = getEnvVar("SAFE_MULTISIG_ADDRESS");
    }

    if (!dryRun) {
      // Initialize web3 provider using the RPC URL from environment variables
      const web3Provider = new ethers.JsonRpcProvider(upgradeData.rpcUrl);

      // Get the signer, this signer must be one of the signers on the Safe Multisig Wallet
      const orchestratorSigner = getSigner(web3Provider, upgradeData.useHardwareWallet);

      // Set up Eth Adapter with ethers and the signer
      const ethAdapter = new EthersAdapter({
        ethers,
        signerOrProvider: orchestratorSigner,
      });

      const chainId = await ethAdapter.getChainId();
      const safeService = new SafeApiKit({ chainId });
      validateEthereumAddress(upgradeData.safeAddress);
      const safeSdk = await Safe.create({ ethAdapter, safeAddress: upgradeData.safeAddress });
      const orchestratorSignerAddress = await orchestratorSigner.getAddress();

      await proposeUpgradeTransaction(safeSdk, safeService, orchestratorSignerAddress, upgradeData);

      console.log(
        `The other owners of the Safe Multisig wallet need to sign the transaction via the Safe UI https://app.safe.global/transactions/queue?safe=sep:${upgradeData.safeAddress}`,
      );
    }
  } catch (error) {
    throw new Error("An error occurred: " + error);
  }
}

export function processRustCommandLineArguments(args: string[]): [UpgradeData, boolean] {
  let proxyAddress = "";
  let implementationAddress = "";
  let initData = "";
  let rpcUrl = "";
  let safeAddress = "";
  let useHardwareWallet = false;
  let dryRun = false;
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
  implementationAddress = map["impl"];
  initData = map["init-data"];
  rpcUrl = map["rpc-url"];
  safeAddress = map["safe-address"];
  useHardwareWallet = map["use-hardware-wallet"] === "true";
  dryRun = map["dry-run"] === "true";
  // if any of the arguments are not provided, throw an error
  if (!proxyAddress || !implementationAddress || !initData || !rpcUrl || !safeAddress) {
    throw new Error("All arguments are required, --proxy, --impl, --init-data, --rpc-url, --safe-address");
  }
  validateEthereumAddress(proxyAddress);
  validateEthereumAddress(implementationAddress);
  validateEthereumAddress(safeAddress);

  let upgradeData: UpgradeData = {
    proxyAddress: proxyAddress,
    implementationAddress: implementationAddress,
    initData: initData,
    rpcUrl: rpcUrl,
    safeAddress: safeAddress,
    useHardwareWallet: useHardwareWallet,
  };

  return [upgradeData, dryRun];
}

function processCommandLineArguments(): [UpgradeData, boolean] {
  let proxyAddress = "";
  let implementationAddress = "";
  let initData = "";
  let rpcUrl = "";
  let safeAddress = "";
  let useHardwareWallet = false;
  let dryRun = false;
  const args = process.argv.slice(2); // Remove the first two args (node command and script name)
  if (args.length === 0 || args.length < 3) {
    throw new Error(
      `Incorrect number of arguments, enter ${UPGRADE_PROXY_CMD} followed by the new implementation contract address and its init data`,
    );
  } else if (args.includes("--from-rust")) {
    return processRustCommandLineArguments(args);
  } else {
    // it's being called from elsewhere, so we need to parse the arguments differently to maintain backwards compatibility
    const command = args[0];
    if (command !== UPGRADE_PROXY_CMD) {
      throw new Error(`Only ${UPGRADE_PROXY_CMD} command is supported.`);
    }
    proxyAddress = args[1];
    implementationAddress = args[2];
    validateEthereumAddress(implementationAddress);
    initData = args[3];

    try {
      useHardwareWallet = getEnvVar("USE_HARDWARE_WALLET") === "true";
    } catch (error) {
      console.error("USE_HARDWARE_WALLET is not set, defaulting to false");
    }
    dryRun = args[4] === "true";
  }

  let upgradeData: UpgradeData = {
    proxyAddress: proxyAddress,
    implementationAddress: implementationAddress,
    initData: initData,
    rpcUrl: rpcUrl,
    safeAddress: safeAddress,
    useHardwareWallet: useHardwareWallet,
  };

  return [upgradeData, dryRun];
}

/**
 * Function to propose the transaction data for upgrading the new implementation
 * @param {string} safeSDK - An instance of the Safe SDK
 * @param {string} safeService - An instance of the Safe Service
 * @param {string} signerAddress - The address of the address signing the transaction
 * @param {UpgradeData} upgradeData - The data for the upgrade
 */
async function proposeUpgradeTransaction(
  safeSDK: Safe,
  safeService: SafeApiKit,
  signerAddress: string,
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
    upgradeData.useHardwareWallet,
  );

  // Propose the transaction which can be signed by other owners via the Safe UI
  await safeService.proposeTransaction({
    safeAddress: upgradeData.safeAddress,
    safeTransactionData: safeTransaction.data,
    safeTxHash: safeTxHash,
    senderAddress: signerAddress,
    senderSignature: senderSignature.data,
  });
}

if (require.main === module) {
  main();
}
