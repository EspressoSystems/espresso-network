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
- **Output**: [deployments/](deployments/) - deployment info per network (TOML)

## Contract Information Collected

For each contract:

- **Address**: From environment variable
- **Owner**: Queried on-chain via AccessControl (`DEFAULT_ADMIN_ROLE`) for StakeTable/RewardClaim, `Ownable.owner()` for
  others
- **Owner name**: Resolved from known addresses (multisigs + timelocks in the .env)
- **Version**: Queried via `IVersioned.getVersion()`
- **Pauser** (StakeTable, RewardClaim): Who holds `PAUSER_ROLE`, resolved to a name

All role holders must be known addresses from the .env config; the tool errors if an unknown address holds a role.

For Safe multisigs:

- **Version**: Queried via `VERSION()`
- **Owners**: Queried via `getOwners()`
- **Threshold**: Queried via `getThreshold()`

For timelocks:

- **Min delay**: Queried via `getMinDelay()`, displayed in human-readable format (e.g. `7days`)
