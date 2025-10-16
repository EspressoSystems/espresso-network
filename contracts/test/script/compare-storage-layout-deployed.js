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
    // Use cast to get the storage layout from deployed contract
    const output = execSync(`cast storage-layout ${contractAddress} --rpc-url ${process.env.RPC_URL || 'http://localhost:8545'}`).toString();
    
    // Parse the output and convert to our format
    const lines = output.trim().split('\n');
    const storage = [];
    
    for (const line of lines) {
      if (line.includes('|')) {
        const parts = line.split('|').map(p => p.trim()).filter(p => p);
        if (parts.length >= 4 && parts[0] !== 'Slot') {
          storage.push({
            label: parts[1],
            slot: parts[0],
            offset: '0', // cast doesn't provide offset, assume 0
            type: parts[2] || 'unknown'
          });
        }
      }
    }
    
    return storage;
  } catch (error) {
    console.error(`Error getting deployed contract layout: ${error.message}`);
    throw error;
  }
}

// Extracts the storage layout using `forge inspect` and parses the JSON output
function extractLocalLayout(contractName) {
  const output = execSync(`forge inspect ${contractName} storageLayout --json`).toString();
  
  // Find the JSON part by looking for the first '{' and last '}'
  const startIndex = output.indexOf('{');
  const lastIndex = output.lastIndexOf('}');
  
  if (startIndex === -1 || lastIndex === -1) {
    throw new Error(`No valid JSON found in output: ${output}`);
  }
  
  const jsonOutput = output.substring(startIndex, lastIndex + 1);
  const layout = JSON.parse(jsonOutput);
  return layout.storage.map(({ label, slot, offset, type }) => ({
    label,
    slot,
    offset,
    type: normalizeType(type),
  }));
}

// Compare two storage layout arrays
// expects the first layout to be the deployed one and the second to be the new one
function compareLayouts(layoutA, layoutB) {
  if (layoutA.length > layoutB.length) {
    // the new layout should have same or more variables
    return false;
  }

  for (let i = 0; i < layoutA.length; i++) {
    const a = layoutA[i];
    const b = layoutB[i];

    if (a.label !== b.label || a.slot !== b.slot || a.offset !== b.offset || a.type !== b.type) {
      return false;
    }
  }

  return true;
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
  const isCompatible = compareLayouts(deployedLayout, newLayout);
  console.log(isCompatible);

  process.exit(0);
} catch (err) {
  console.error("Error comparing layouts:", err.message);
  process.exit(1);
}
