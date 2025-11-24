# Deployment Info Tool

Tool to collect and output deployment information for Espresso Network contracts.

## Usage

```bash
cargo run -p deployment-info -- --network <mainnet|decaf> \
  [--rpc-url <RPC_URL>] \
  [--env-file <PATH_TO_ENV_FILE>] \
  [--output <OUTPUT_PATH>]
```

Examples:

```bash
# Uses default publicnode RPC for decaf
cargo run -p deployment-info -- --network decaf --env-file decaf.env

# Uses default publicnode RPC for mainnet
cargo run -p deployment-info -- --network mainnet --env-file mainnet.env

# Custom RPC URL
cargo run -p deployment-info -- \
  --network mainnet \
  --rpc-url https://eth.llamarpc.com \
  --env-file mainnet.env
```

If no `--output` is specified, the tool prints JSON to stdout.

## Environment Variables

The tool reads contract addresses from environment variables (or from a specified env file):

- `ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS` - Safe multisig wallet
- `ESPRESSO_SEQUENCER_OPS_TIMELOCK_PROXY_ADDRESS` - Operations timelock
- `ESPRESSO_SEQUENCER_SAFE_EXIT_TIMELOCK_PROXY_ADDRESS` - Safe exit timelock
- `ESPRESSO_SEQUENCER_STAKE_TABLE_PROXY_ADDRESS`
- `ESPRESSO_SEQUENCER_ESP_TOKEN_PROXY_ADDRESS`
- `ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS`
- `ESPRESSO_SEQUENCER_FEE_CONTRACT_PROXY_ADDRESS`
- `ESPRESSO_SEQUENCER_REWARD_CLAIM_PROXY_ADDRESS`

If addresses are not set the tool assumes the contracts are not deployed yet.

## Output

By default, prints to stdout. If `--output` is specified, writes to that file.

Example output:

```json
{
  "network": "decaf",
  "multisig": {
    "status": "deployed",
    "address": "0xB76834E371B666feEe48e5d7d9A97CA08b5a0620",
    "version": "1.3.0",
    "owners": ["0x1234...", "0x5678..."],
    "threshold": 2
  },
  "ops_timelock": {
    "status": "not-yet-deployed"
  },
  "safe_exit_timelock": {
    "status": "not-yet-deployed"
  },
  "light_client_proxy": {
    "status": "deployed",
    "proxy_address": "0x303872BB82a191771321d4828888920100d0b3e4",
    "owner": "0x...",
    "version": "3.0.0"
  },
  "stake_table_proxy": {
    "status": "not-yet-deployed"
  },
  "esp_token_proxy": {
    "status": "not-yet-deployed"
  },
  "fee_contract_proxy": {
    "status": "deployed",
    "proxy_address": "0x9fce21c3f7600aa63392a5f5713986b39bb98884",
    "owner": "0x...",
    "version": "1.0.0"
  },
  "reward_claim_proxy": {
    "status": "not-yet-deployed"
  }
}
```

## Contract Information Collected

For each contract:
- **Proxy address**: From environment variable
- **Owner**: Queried on-chain (for Ownable contracts)
- **Version**: Queried via `IVersioned.getVersion()` interface

For the Safe multisig:
- **Address**: From environment variable
- **Version**: Queried via `VERSION()`
- **Owners**: Queried via `getOwners()`
- **Threshold**: Queried via `getThreshold()`

For timelocks (OpsTimelock, SafeExitTimelock):
- **Address**: From environment variable
- **Min delay**: Queried via `getMinDelay()` (delay in seconds before operations can execute)
