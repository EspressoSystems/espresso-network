import dotenv from "dotenv";
import { ethers } from "ethers";
import { EthersAdapter } from "@safe-global/protocol-kit";
import SafeApiKit from "@safe-global/api-kit";
import Safe from "@safe-global/protocol-kit";
import { getEnvVar, validateEthereumAddress, createAndSignSafeTransaction, getSigner } from "./utils";

async function main() {
  dotenv.config();

  try {
    const approveAmount = process.argv[2];
    if (!approveAmount) {
      throw new Error("Approval amount (in ether) is required");
    }
    const approveAmountInWei = ethers.parseEther(approveAmount.toString());

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

    const tokenAddress = getEnvVar("TOKEN_CONTRACT_PROXY_ADDRESS");
    validateEthereumAddress(tokenAddress);

    const stakeTableAddress = getEnvVar("STAKE_TABLE_CONTRACT_PROXY_ADDRESS");
    validateEthereumAddress(stakeTableAddress);

    console.log("Approving ", approveAmount, " ESP to ", stakeTableAddress);
    await proposeApproveTransaction(
      safeSdk,
      safeService,
      orchestratorSignerAddress,
      safeAddress,
      tokenAddress,
      stakeTableAddress,
      approveAmountInWei,
    );

    console.log(
      `The other owners of the Safe Multisig wallet need to sign the transaction via the Safe UI https://app.safe.global/transactions/queue?safe=sep:${safeAddress}`,
    );
  } catch (error) {
    throw new Error("An error occurred: " + error);
  }
}

export async function proposeApproveTransaction(
  safeSDK: Safe,
  safeService: SafeApiKit,
  signerAddress: string,
  safeAddress: string,
  tokenAddress: string,
  stakeTableAddress: string,
  approveAmountWei: bigint,
) {
  // approve the stake table to spend the tokens
  const approveAbi = ["function approve(address,uint256)"];
  const approveData = new ethers.Interface(approveAbi).encodeFunctionData("approve", [
    stakeTableAddress,
    approveAmountWei,
  ]);

  // Create & Sign the Safe Transaction Object
  const { safeTransaction, safeTxHash, senderSignature } = await createAndSignSafeTransaction(
    safeSDK,
    tokenAddress,
    approveData,
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
