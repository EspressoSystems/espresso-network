# Running the flow

1. Assuming the contracts are deployed and their proxy addresses are found `$ENV_FILE`
2. Ensure that you have an RPC URL for the network the contracts are deployed to
3. Have your ledger connected (assumes account index = 0 otherwise set `export ACCOUNT_INDEX=YOUR_ACCOUNT_INDEX`)

## Build Optimization

To avoid rebuilds during script execution, pre-build the deploy binary:

# Debug build (faster compilation, slower execution)

cargo build --bin deploy

```bash
export RPC_URL=
./contracts/rust/deployer/scripts/testnet-governance-flows.sh --ledger --env-file $ENV_FILE
```

# Notes

- The script will prompt for confirmation before each operation
- Operations use a 30-second delay by default (configurable via OPS_DELAY env var)
- For non-localhost RPCs, you'll be prompted to confirm before proceeding
