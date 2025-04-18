const { execSync } = require("child_process");

// Different types of types have different IDs, so we need to strip the ID from the type
// (e.g., t_contract(LightClient)44013 → t_contract(LightClient))
function normalizeType(type) {
  const end = type.indexOf(')');

  if (end !== -1) {
    return type.slice(0, end + 1); 
  }

    return type; 
}

// Extracts the storage layout using `forge inspect` and parses the JSON output
function extractLayout(contractName) {
  const output = execSync(`forge inspect ${contractName} storageLayout --json`).toString();
  const layout = JSON.parse(output);
  return layout.storage.map(({ label, slot, offset, type }) => ({
    label,
    slot,
    offset,
    type: normalizeType(type),
  }));
}

// Compare two storage layout arrays
// expects the first layout to be the old one and the second to be the new one
function compareLayouts(layoutA, layoutB) {
  if (layoutA.length > layoutB.length) { // the new layout should have same or more variables
    console.log("false");
    return false;
  }

  for (let i = 0; i < layoutA.length; i++) {
    const a = layoutA[i];
    const b = layoutB[i];

    if (
      a.label !== b.label ||
      a.slot !== b.slot ||
      a.offset !== b.offset ||
      a.type !== b.type
    ) {
      console.log("false");
      return false;
    }
  }

  console.log("true");
  return true;
}

const [contractA, contractB] = process.argv.slice(2);

if (!contractA || !contractB) {
  console.error("Usage: node compare-storage-layout.js oldContractName newContractName");
  process.exit(1);
}

try {
  const layoutA = extractLayout(contractA);
  const layoutB = extractLayout(contractB);
  const success = compareLayouts(layoutA, layoutB);

  process.exit(success ? 0 : 1);
} catch (err) {
  console.error("Error comparing layouts:", err.message);
  process.exit(1);
}
