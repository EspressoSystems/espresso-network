import { ethers } from "ethers";

export function decodeFunctionCall(contractAbi: any, encodedData: string) {
  const iface = new ethers.Interface(contractAbi);
  return iface.parseTransaction({ data: encodedData });
}

function decodeProposalData() {
  try {
    let contractName = process.argv[2];
    const encodedData = process.argv[3];

    if (!contractName || !encodedData) {
      throw new Error("Contract name and encoded data are required");
    }
    if (!ethers.isHexString(encodedData)) {
      throw new Error("Encoded data must be a hex string");
    }

    contractName = contractName.replace(".sol", "");

    const contractAbi = require(`../../../out/${contractName}.sol/${contractName}.json`).abi;

    const decodedData = decodeFunctionCall(contractAbi, encodedData);
    if (decodedData) {
      console.log("Function Name:", decodedData?.name);
      console.log("Arguments:", decodedData?.args.toString() || "No arguments");
      console.log("Signature:", decodedData?.signature);
      console.log("Selector:", decodedData?.selector);
    } else {
      console.log("No function call found in the encoded data");
    }
  } catch (error: any) {
    if (error.shortMessage) {
      console.error("Error Message:", error.shortMessage);
    } else {
      console.error("Error Message:", error);
    }
    console.error("Ensure the contract name is correct and the encoded data is valid e.g. it must start with 0x");
    process.exit(1);
  }
}

// Call the function with the encoded data
decodeProposalData();
