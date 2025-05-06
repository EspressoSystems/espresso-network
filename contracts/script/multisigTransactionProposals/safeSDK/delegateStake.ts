import dotenv from "dotenv";
import { ethers } from "ethers";
import { EthersAdapter } from "@safe-global/protocol-kit";
import SafeApiKit from "@safe-global/api-kit";
import Safe from "@safe-global/protocol-kit";
import { getEnvVar, validateEthereumAddress, createAndSignSafeTransaction, getSigner } from "./utils";

async function main() {
  dotenv.config();

  try {
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

    // get stake amount and validator address from command line arguments
    const validator = process.argv[2];
    if (!validator) {
      throw new Error("Validator address is required");
    }
    validateEthereumAddress(validator);

    const stakeAmount = process.argv[3];
    if (!stakeAmount) {
      throw new Error("Stake amount is required");
    }
    const stakeAmountInWei = ethers.parseEther(stakeAmount.toString());

    const tokenAddress = getEnvVar("TOKEN_CONTRACT_PROXY_ADDRESS");
    validateEthereumAddress(tokenAddress);

    const stakeTableAddress = getEnvVar("STAKE_TABLE_CONTRACT_PROXY_ADDRESS");
    validateEthereumAddress(stakeTableAddress);

    const tokenApprovalAmount = await getTokenApprovalAmount(
      tokenAddress,
      stakeTableAddress,
      web3Provider,
      safeAddress,
    );
    if (tokenApprovalAmount < stakeAmountInWei) {
      throw new Error("Token approval amount is less than the stake amount, propose approve transaction instead");
    }
    console.log("Stake ", stakeAmount, " ESP to ", validator);
    console.log("Token approval amount remaining: ", ethers.formatEther(tokenApprovalAmount));

    await proposeStakeTransaction(
      safeSdk,
      safeService,
      orchestratorSignerAddress,
      safeAddress,
      stakeTableAddress,
      tokenAddress,
      validator,
      stakeAmountInWei,
    );

    console.log(
      `The other owners of the Safe Multisig wallet need to sign the transaction via the Safe UI https://app.safe.global/transactions/queue?safe=sep:${safeAddress}`,
    );
  } catch (error) {
    throw new Error("An error occurred: " + error);
  }
}

/**
 * Function to propose the transaction data for setting the state history retention period
 * @param {string} safeSDK - An instance of the Safe SDK
 * @param {string} safeService - An instance of the Safe Service
 * @param {string} signerAddress - The address of the address signing the transaction
 * @param {string} safeAddress - The address of the Safe multisig wallet
 * @param {string} validator - The address of the validator
 * @param {bigint} stakeAmountInWei - The amount of ESP (in wei) to stake
 */
export async function proposeStakeTransaction(
  safeSDK: Safe,
  safeService: SafeApiKit,
  signerAddress: string,
  safeAddress: string,
  stakeTableAddress: string,
  tokenAddress: string,
  validator: string,
  stakeAmountInWei: bigint,
) {
  // Define the ABI of the function to be called
  const stakeAbi = ["function delegate(address,uint256)"];

  // Encode the function call with the provided stateHistoryRetentionPeriod
  const stakeData = new ethers.Interface(stakeAbi).encodeFunctionData("delegate", [validator, stakeAmountInWei]);

  // Create & Sign the Safe Transaction Object
  const { safeTransaction, safeTxHash, senderSignature } = await createAndSignSafeTransaction(
    safeSDK,
    stakeTableAddress,
    stakeData,
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

async function getTokenApprovalAmount(
  tokenAddress: string,
  stakeTableAddress: string,
  web3Provider: ethers.JsonRpcProvider,
  multisigAddress: string,
): Promise<bigint> {
  const tokenApprovalAbi = ["function allowance(address,address) view returns (uint256)"];
  const tokenContract = new ethers.Contract(tokenAddress, tokenApprovalAbi, web3Provider);
  const tokenApprovalAmount = await tokenContract.allowance(multisigAddress, stakeTableAddress);
  return tokenApprovalAmount;
}

main();
