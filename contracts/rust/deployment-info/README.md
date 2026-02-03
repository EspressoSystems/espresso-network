# Deployment Info Tool

Tool to collect and output Layer 1 deployment information for Espresso Network contracts.

## Usage

```bash
# Process all networks (decaf, hoodi, mainnet)
cargo run -p deployment-info

# Process a specific network
cargo run -p deployment-info -- --network <mainnet|decaf|hoodi>

# Print to stdout instead of file
cargo run -p deployment-info -- --network decaf --stdout

# Custom RPC URL
cargo run -p deployment-info -- --network mainnet --rpc-url https://eth.llamarpc.com
```

## Directory Structure

- **Input**: [addresses/](addresses/) - contract addresses per network
- **Output**: [deployments/](deployments/) - deployment info per network (YAML)

## Contract Information Collected

For each contract:

- **Address**: From environment variable
- **Owner**: Queried on-chain (for Ownable contracts)
- **Version**: Queried via `IVersioned.getVersion()` interface

For Safe multisigs:

- **Address**: From environment variable
- **Version**: Queried via `VERSION()`
- **Owners**: Queried via `getOwners()`
- **Threshold**: Queried via `getThreshold()`

For timelocks:

- **Address**: From environment variable
- **Min delay**: Queried via `getMinDelay()` (seconds before operations can execute)
