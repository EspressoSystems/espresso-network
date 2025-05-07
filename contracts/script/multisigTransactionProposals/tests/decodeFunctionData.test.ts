import { decodeFunctionCall } from "../safeSDK/decodeFunctionData";
import { ethers } from "ethers";

const abi = [
  {
    inputs: [{ internalType: "uint64", name: "_startBlock", type: "uint64" }],
    name: "updateEpochStartBlock",
    outputs: [],
    stateMutability: "nonpayable",
    type: "function",
  },
];

describe("decodeFunctionCall", () => {
  it("decodes updateEpochStartBlock(uint64) correctly", () => {
    const iface = new ethers.Interface(abi);
    const encodedData = iface.encodeFunctionData("updateEpochStartBlock", [3160636]);

    const decoded = decodeFunctionCall(abi, encodedData);

    expect(decoded?.name).toBe("updateEpochStartBlock");
    expect(decoded?.args[0].toString()).toBe("3160636");
    expect(decoded?.signature).toBe("updateEpochStartBlock(uint64)");
  });

  it("throws on invalid data", () => {
    // invalid data returns null
    const decoded = decodeFunctionCall(abi, "0x1234");
    expect(decoded).toBeNull();
  });

  it("throws on invalid abi", () => {
    const invalidAbi = [
      {
        inputs: [],
        name: "invalidFunction",
        outputs: [],
        stateMutability: "nonpayable",
        type: "function",
      },
    ];
    // invalid abi returns null and logs an error
    const decoded = decodeFunctionCall(invalidAbi, "0x1234");
    expect(decoded).toBeNull();
  });
});
