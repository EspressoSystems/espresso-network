//test processRustCommandLineArguments from upgradeProxy.ts
import { processRustCommandLineArguments } from "../safeSDK/upgradeProxy";
import { ethers } from "ethers";

describe("processRustCommandLineArguments", () => {
  it("should return the correct arguments", () => {
    // create a random ethereum address
    const proxyAddress = ethers.Wallet.createRandom().address;
    const implementationAddress = ethers.Wallet.createRandom().address;
    const initData = ethers.Wallet.createRandom().address;
    const rpcUrl = ethers.Wallet.createRandom().address;
    const safeAddress = ethers.Wallet.createRandom().address;
    const args = [
      "--proxy",
      proxyAddress,
      "--impl",
      implementationAddress,
      "--init-data",
      initData,
      "--rpc-url",
      rpcUrl,
      "--safe-address",
      safeAddress,
    ];
    const result = processRustCommandLineArguments(args);
    expect(result).toEqual({
      proxyAddress: proxyAddress,
      implementationAddress: implementationAddress,
      initData: initData,
      rpcUrl: rpcUrl,
      safeAddress: safeAddress,
      useHardwareWallet: false,
    });
  });

  it("should throw an error if the arguments are not provided", () => {
    const args = [];
    expect(() => processRustCommandLineArguments(args)).toThrow();
  });

  it("should throw an error if the arguments are not valid", () => {
    const args = [
      "--proxy",
      "0x123",
      "--impl",
      "0x456",
      "--init-data",
      "0x789",
      "--rpc-url",
      "0x100",
      "--safe-address",
      "0x101",
    ];
    expect(() => processRustCommandLineArguments(args)).toThrow();
  });

  it("should throw an error if some arguments are not provided", () => {
    const args = ["--proxy", "0x123", "--impl", "0x456", "--init-data", "0x789", "--rpc-url", "0x100"];
    expect(() => processRustCommandLineArguments(args)).toThrow();
  });
});
