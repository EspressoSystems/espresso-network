const { ethers, upgrades } = require("hardhat");

async function main() {
  console.log("🔍 Checking upgrade safety...\n");

  // Get the deployed proxy address
  const PROXY_ADDRESS = process.env.PROXY_ADDRESS || "0x303872BB82a191771321d4828888920100d0b3e4";
  
  try {
    // Get the current implementation
    const currentImpl = await upgrades.erc1967.getImplementationAddress(PROXY_ADDRESS);
    console.log(`📍 Current implementation: ${currentImpl}`);
    
    // Get the new implementation
    const NewImplementation = await ethers.getContractFactory("LightClientV3");
    const newImpl = await NewImplementation.deploy();
    await newImpl.deployed();
    console.log(`🆕 New implementation: ${newImpl.address}`);
    
    // Check for storage layout conflicts
    console.log("\n🔍 Checking storage layout compatibility...");
    
    try {
      // This will throw if there are storage layout conflicts
      await upgrades.validateUpgrade(currentImpl, newImpl.address);
      console.log("✅ Storage layout is compatible!");
    } catch (error) {
      console.log("❌ Storage layout conflict detected:");
      console.log(error.message);
      process.exit(1);
    }
    
    // Check for initialization issues
    console.log("\n🔍 Checking initialization compatibility...");
    
    try {
      // This checks if the new implementation can be initialized properly
      await upgrades.validateImplementation(newImpl.address);
      console.log("✅ Implementation is valid!");
    } catch (error) {
      console.log("❌ Implementation validation failed:");
      console.log(error.message);
      process.exit(1);
    }
    
    console.log("\n🎉 All checks passed! Safe to upgrade.");
    
  } catch (error) {
    console.error("❌ Error during upgrade safety check:", error.message);
    process.exit(1);
  }
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
