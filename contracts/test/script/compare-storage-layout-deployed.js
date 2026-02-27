const { execSync } = require("child_process");

// Different types of types have different IDs, so we need to strip the ID from the type
// (e.g., t_contract(LightClient)44013 → t_contract(LightClient))
function normalizeType(type) {
  const end = type.indexOf(")");

  if (end !== -1) {
    return type.slice(0, end + 1);
  }

  return type;
}

// Extracts the storage layout from a deployed contract using cast
function extractDeployedLayout(contractAddress) {
  try {
    // Use cast with --json flag to get structured storage layout
    const result = execSync(
      `cast storage ${contractAddress} --rpc-url ${process.env.RPC_URL || "http://localhost:8545"} --json`,
      {
        encoding: "utf8",
        stdio: ["pipe", "pipe", "pipe"],
      },
    );

    const output = result.toString().trim();

    // Handle empty output
    if (!output) {
      return [];
    }

    const data = JSON.parse(output);

    // Handle empty storage array
    if (!data.storage || data.storage.length === 0) {
      return [];
    }

    // Convert to our format
    return data.storage.map(({ label, slot, offset, type }) => ({
      label,
      slot: parseInt(slot),
      offset: parseInt(offset),
      type: normalizeType(type),
    }));
  } catch (error) {
    // Check if stderr contains "Storage layout is empty" warning
    if (error.stderr && error.stderr.toString().includes("Storage layout is empty")) {
      return [];
    }
    console.error(`Error getting deployed contract layout: ${error.message}`);
    throw error;
  }
}

// Extracts the storage layout using `forge inspect` and parses the JSON output
function extractLocalLayout(contractName) {
  const output = execSync(`forge inspect ${contractName} storageLayout --json`).toString();

  // Find the JSON part by looking for the first '{' and last '}'
  const startIndex = output.indexOf("{");
  const lastIndex = output.lastIndexOf("}");

  if (startIndex === -1 || lastIndex === -1) {
    throw new Error(`No valid JSON found in output: ${output}`);
  }

  const jsonOutput = output.substring(startIndex, lastIndex + 1);
  const layout = JSON.parse(jsonOutput);
  return layout.storage.map(({ label, slot, offset, type }) => ({
    label,
    slot: parseInt(slot),
    offset: parseInt(offset),
    type: normalizeType(type),
  }));
}

// Compare two storage layout arrays
// expects the first layout to be the deployed one and the second to be the new one
function compareLayouts(layoutA, layoutB) {
  const errors = [];

  if (layoutA.length > layoutB.length) {
    // the new layout should have same or more variables
    errors.push(
      `Deployed contract has ${layoutA.length} storage variables but new contract has only ${layoutB.length}`,
    );
    return { compatible: false, errors };
  }

  for (let i = 0; i < layoutA.length; i++) {
    const a = layoutA[i];
    const b = layoutB[i];

    if (a.label !== b.label) {
      errors.push(`Position ${i}: label mismatch - deployed: "${a.label}", local: "${b.label}"`);
    }
    if (a.slot !== b.slot) {
      errors.push(`Position ${i} (${a.label}): slot mismatch - deployed: ${a.slot}, local: ${b.slot}`);
    }
    if (a.offset !== b.offset) {
      errors.push(`Position ${i} (${a.label}): offset mismatch - deployed: ${a.offset}, local: ${b.offset}`);
    }
    // Compare types, but allow contract/interface substitution
    if (a.type !== b.type) {
      // Extract base type for contracts and interfaces
      const aIsContract = a.type.startsWith("t_contract(");
      const bIsContract = b.type.startsWith("t_contract(");

      // If both are contracts/interfaces, they're compatible (both store as address)
      if (!(aIsContract && bIsContract)) {
        errors.push(`Position ${i} (${a.label}): type mismatch - deployed: ${a.type}, local: ${b.type}`);
      }
    }
  }

  return { compatible: errors.length === 0, errors };
}

const [deployedAddress, newContractName] = process.argv.slice(2);

if (!deployedAddress || !newContractName) {
  console.error("Usage: node compare-storage-layout-deployed.js deployedAddress newContractName");
  console.error("Example: node compare-storage-layout-deployed.js 0x123...abc LightClientV3");
  process.exit(1);
}

try {
  const deployedLayout = extractDeployedLayout(deployedAddress);
  const newLayout = extractLocalLayout(newContractName);

  // If deployed layout is empty (e.g., OZ V5 namespaced storage), skip checks
  if (deployedLayout.length === 0) {
    console.log(true);
    process.exit(0);
  }

  const { compatible, errors } = compareLayouts(deployedLayout, newLayout);

  if (!compatible) {
    console.error(`\nStorage layout mismatch between deployed contract (${deployedAddress}) and ${newContractName}:`);
    console.error("\nErrors:");
    errors.forEach((err) => console.error(`  - ${err}`));

    console.error("\nDeployed layout:");
    deployedLayout.forEach((item, i) => {
      console.error(`  [${i}] ${item.label}: slot=${item.slot}, offset=${item.offset}, type=${item.type}`);
    });

    console.error(`\nLocal layout (${newContractName}):`);
    newLayout.forEach((item, i) => {
      console.error(`  [${i}] ${item.label}: slot=${item.slot}, offset=${item.offset}, type=${item.type}`);
    });
    console.error("");
  }

  console.log(compatible);
  process.exit(0);
} catch (err) {
  console.error("Error comparing layouts:", err.message);
  process.exit(1);
}
