import * as dotenv from "dotenv";
import { ethers } from "ethers";
import { EthersAdapter } from "@safe-global/protocol-kit";
import SafeApiKit from "@safe-global/api-kit";
import Safe from "@safe-global/protocol-kit";
import {
  getEnvVar,
  validateEthereumAddress,
  getSigner,
  createAndSignSafeTransaction,
  LocalSafeTransaction,
  SafeSignature,
} from "./utils";

export interface GenericProposalData {
  target: string; // Contract address to call
  functionSignature: string; // e.g., "schedule(address,uint256,bytes,bytes32,bytes32,uint256)"
  functionArgs: string[]; // Array of argument values (as strings, will be parsed)
  value: string; // ETH value to send (default "0")
  rpcUrl: string;
  safeAddress: string;
  useHardwareWallet: boolean;
}

async function main() {
  dotenv.config();

  try {
    const [proposalData, dryRun] = processCommandLineArguments();

    console.log(JSON.stringify(proposalData));

    if (dryRun) {
      return;
    }

    // Initialize provider and signer
    const web3Provider = new ethers.JsonRpcProvider(proposalData.rpcUrl);
    const orchestratorSigner = getSigner(web3Provider, proposalData.useHardwareWallet);

    // Set up Safe SDK
    const ethAdapter = new EthersAdapter({
      ethers,
      signerOrProvider: orchestratorSigner,
    });

    const chainId = await ethAdapter.getChainId();
    const safeService = new SafeApiKit({ chainId });
    validateEthereumAddress(proposalData.safeAddress);
    const safeSdk = await Safe.create({ ethAdapter, safeAddress: proposalData.safeAddress });
    const orchestratorSignerAddress = await orchestratorSigner.getAddress();

    // Encode function call
    const abi = [`function ${proposalData.functionSignature}`];
    const iface = new ethers.Interface(abi);
    const functionName = proposalData.functionSignature.split("(")[0];

    // Parse arguments based on their types
    const parsedArgs = parseFunctionArgs(proposalData.functionSignature, proposalData.functionArgs);
    const encodedData = iface.encodeFunctionData(functionName, parsedArgs);

    // Create and sign Safe transaction - NOTE: createAndSignSafeTransaction hardcodes value to "0"
    // If you need non-zero value, you'll need to modify utils.ts or create transaction differently
    let safeTransaction: LocalSafeTransaction;
    let safeTxHash: string;
    let senderSignature: SafeSignature;
    try {
      const result = await createAndSignSafeTransaction(
        safeSdk,
        proposalData.target,
        encodedData,
        proposalData.useHardwareWallet,
      );
      safeTransaction = result.safeTransaction;
      safeTxHash = result.safeTxHash;
      senderSignature = result.senderSignature;
    } catch (error: unknown) {
      const errorMessage = (error as any)?.message || String(error);
      throw new Error(`Failed to create and sign Safe transaction: ${errorMessage}`);
    }

    // Propose transaction
    try {
      await safeService.proposeTransaction({
        safeAddress: proposalData.safeAddress,
        safeTransactionData: safeTransaction.data,
        safeTxHash,
        senderAddress: orchestratorSignerAddress,
        senderSignature: senderSignature.data,
      });
    } catch (apiError: unknown) {
      // Extract detailed error information
      const errorObj = apiError as any;
      const errorMessage = errorObj?.message || String(apiError);
      const statusCode = errorObj?.response?.status || errorObj?.status || "Unknown";
      const errorResponse = errorObj?.response?.data || errorObj?.response || errorObj?.data;

      console.error("\n=== Safe Transaction Service Error ===");
      console.error(`HTTP Status: ${statusCode}`);
      console.error(`Error Message: ${errorMessage}`);

      if (errorResponse) {
        console.error(`Full Error Response: ${JSON.stringify(errorResponse, null, 2)}`);
      }

      console.error(`\nContext:`);
      console.error(`  Chain ID: ${chainId}`);
      console.error(`  Safe Address: ${proposalData.safeAddress}`);
      console.error(`  Orchestrator Signer Address: ${orchestratorSignerAddress}`);
      console.error(`  RPC URL: ${proposalData.rpcUrl}`);
      console.error(`  Safe Tx Hash: ${safeTxHash}`);
      // Check if Safe exists on Sepolia (the real network, not the fork)
      console.error(`\nChecking if Safe exists on Sepolia (chain ${chainId})...`);
      try {
        const safeInfo = await safeService.getSafeInfo(proposalData.safeAddress);
        console.error(`✓ Safe exists on Sepolia`);
        console.error(`  Owners: ${safeInfo.owners.join(", ")}`);
        console.error(`  Threshold: ${safeInfo.threshold}`);
        console.error(`  Nonce: ${safeInfo.nonce}`);

        // Check if sender is an owner
        const isOwner = safeInfo.owners.some(
          (owner) => owner.toLowerCase() === orchestratorSignerAddress.toLowerCase(),
        );
        if (!isOwner) {
          console.error(`\n✗ ERROR: Orchestrator signer ${orchestratorSignerAddress} is NOT an owner on Sepolia!`);
          console.error(`  Safe owners on Sepolia: ${safeInfo.owners.join(", ")}`);
          console.error(
            `\nNote: Even if this address is an owner on your fork, it must also be an owner on real Sepolia.`,
          );
        } else {
          console.error(`✓ Orchestrator signer is an owner on Sepolia`);
          console.error(`\nSince the Safe exists and the signer is an owner, the error might be:`);
          console.error(`  1. Invalid transaction data format`);
          console.error(`  2. Invalid signature`);
          console.error(`  3. Nonce mismatch`);
          console.error(`  4. Transaction already exists`);
        }
      } catch (safeCheckError: unknown) {
        const safeCheckMessage = (safeCheckError as any)?.message || String(safeCheckError);
        console.error(`✗ Safe does NOT exist on Sepolia (chain ${chainId})`);
        console.error(`  Error: ${safeCheckMessage}`);
        console.error(`\nThis is the most likely cause of the error!`);
        console.error(`The Safe address ${proposalData.safeAddress} exists on your fork but NOT on real Sepolia.`);
        console.error(`\nThe Safe transaction service validates against the real network, not your fork.`);
        console.error(`\nTo fix:`);
        console.error(`  1. Deploy a Safe on Sepolia: https://app.safe.global/`);
        console.error(`  2. Use an existing Safe address that exists on Sepolia`);
        console.error(`  3. For localhost fork testing, skip the Safe transaction service and execute directly`);
      }

      throw new Error(`Safe transaction service error (HTTP ${statusCode}): ${errorMessage}`);
    }

    console.log(`Safe transaction proposal created`);
    console.log(`View at: https://app.safe.global/transactions/queue?safe=${proposalData.safeAddress}`);
  } catch (error: unknown) {
    const errorMessage = (error as any)?.message || String(error);
    const stack = (error as any)?.stack;
    throw new Error(`An error occurred in proposeTransactionGeneric: ${errorMessage}${stack ? "\n" + stack : ""}`);
  }
}

function parseFunctionArgs(signature: string, args: string[]): any[] {
  // Extract parameter types from signature
  const paramsMatch = signature.match(/\(([^)]*)\)/);
  if (!paramsMatch || paramsMatch[1].trim() === "") {
    // No parameters or empty parameter list
    if (args.length > 0) {
      throw new Error(`Function signature has no parameters but ${args.length} arguments provided`);
    }
    return [];
  }

  const paramTypes = paramsMatch[1]
    .split(",")
    .map((t) => t.trim())
    .filter((t) => t);

  if (args.length !== paramTypes.length) {
    throw new Error(`Argument count mismatch: expected ${paramTypes.length}, got ${args.length}`);
  }

  return args.map((arg, i) => {
    const type = paramTypes[i];
    if (!type) return arg;

    // Handle common types
    if (type.startsWith("uint") || type.startsWith("int")) {
      return BigInt(arg);
    } else if (type === "bool") {
      return arg === "true" || arg === "1";
    } else if (type === "address") {
      validateEthereumAddress(arg);
      return arg;
    } else if (type === "bytes" || type === "bytes32") {
      // Ensure bytes are properly formatted
      return arg.startsWith("0x") ? arg : `0x${arg}`;
    } else if (type.startsWith("bytes")) {
      return arg.startsWith("0x") ? arg : `0x${arg}`;
    }
    // For other types (arrays, tuples, etc.), return as-is and let ethers handle it
    return arg;
  });
}

function processCommandLineArguments(): [GenericProposalData, boolean] {
  const args = process.argv.slice(2);

  if (args.includes("--from-rust")) {
    return processRustCommandLineArguments(args);
  }

  // For non-Rust usage, you could add alternative parsing here
  // For now, just use Rust parsing
  return processRustCommandLineArguments(args);
}

export function processRustCommandLineArguments(args: string[]): [GenericProposalData, boolean] {
  const map: Record<string, string> = {};
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg.startsWith("--")) {
      const key = arg.slice(2); // Remove "--" prefix
      if (key === "from-rust") continue;

      // Handle array arguments (function-args)
      if (key === "function-args") {
        // Collect all remaining arguments until next flag
        const functionArgs: string[] = [];
        let j = i + 1;
        while (j < args.length && !args[j].startsWith("--")) {
          functionArgs.push(args[j]);
          j++;
        }
        map[key] = JSON.stringify(functionArgs);
        i = j - 1;
        continue;
      }

      const value = args[i + 1];
      if (value === undefined) {
        throw new Error(`Missing value for argument: --${key}`);
      }
      map[key] = value;
      i++;
    }
  }

  const target = map["target"] || getEnvVar("TARGET_ADDRESS");
  const functionSignature = map["function-signature"] || map["functionSignature"];
  const functionArgsStr = map["function-args"] || map["functionArgs"] || "[]";
  const value = map["value"] || "0";
  const rpcUrl = map["rpc-url"] || map["rpcUrl"] || getEnvVar("RPC_URL");
  const safeAddress = map["safe-address"] || map["safeAddress"] || getEnvVar("SAFE_MULTISIG_ADDRESS");
  const useHardwareWallet = map["use-hardware-wallet"] === "true";
  const dryRun = map["dry-run"] === "true";

  if (!target || !functionSignature) {
    throw new Error(`Missing required arguments: target=${target}, functionSignature=${functionSignature}`);
  }

  validateEthereumAddress(target);
  validateEthereumAddress(safeAddress);

  let functionArgs: string[];
  try {
    functionArgs = JSON.parse(functionArgsStr);
  } catch (e) {
    throw new Error(`Failed to parse function-args as JSON: ${functionArgsStr}`);
  }

  return [
    {
      target,
      functionSignature,
      functionArgs,
      value,
      rpcUrl,
      safeAddress,
      useHardwareWallet,
    },
    dryRun,
  ];
}

if (require.main === module) {
  main().catch((error) => {
    console.error("Unhandled error:", error);
    process.exit(1);
  });
}
