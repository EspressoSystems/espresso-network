# Governance script test

For testing purposes, use this to deploy POS contracts, with timelock ownership, and execute various timelock flows.

## Pre-requisites

- `nix` installed
- in the root of the directory, enter `nix develop`

### Build Optimization

To avoid rebuilds during script execution, pre-build the deploy binary:

```bash
cargo build --bin deploy
```

## Deploying the contracts

1. Copy the env file

```bash
export ENV_FILE={YOUR_ENV_FILE}
cp .env $ENV_FILE
```

- and replace the following fields in the `$ENV_FILE` if not deploying to a local network via anvil.
  - `ESPRESSO_SEQUENCER_ETH_MNEMONIC`
  - `ESPRESSO_SEQUENCER_ETH_MULTISIG_ADDRESS`

- The default ops and safe exit timelock delays in this script is 30 and 60 seconds respectively. If you want to change
  it then also add the following fields to the ENV_FILE:
  - `OPS_DELAY` (in seconds)
  - `SAFE_EXIT_DELAY` (in seconds)

2. set the RPC_URL, ACCOUNT_INDEX and OUTPUT_FILE

```bash
export RPC_URL={YOUR_RPC_URL}
export ACCOUNT_INDEX={YOUR_ACCOUNT_INDEX} # Optional:  default value is zero
export OUTPUT_FILE={YOUR_OUTPUT_FILE}  # Optional: customize output file
```

3. Run the script

```bash
./contracts/rust/deployer/scripts/testnet-governance-deploy.sh --env-file $ENV_FILE
```

## Running the test flow

1. Assuming the contracts are deployed and their proxy addresses are found `$OUTPUT_FILE`
2. Ensure that you have an RPC URL for the network the contracts are deployed to
3. Have your ledger connected (assumes account index = 0 otherwise set `export ACCOUNT_INDEX=YOUR_ACCOUNT_INDEX`)

```bash
export RPC_URL={YOUR_RPC_URL}
./contracts/rust/deployer/scripts/testnet-governance-flows.sh --ledger --env-file $OUTPUT_FILE
```

**Note**: The `$OUTPUT_FILE` from the deploy script contains the deployed contract addresses and should be used as
`--env-file` for the flows script.

## Notes

- The script will prompt for confirmation before each operation
- Operations use a 30-second delay by default (configurable via OPS_DELAY env var)
- For non-localhost RPCs, you'll be prompted to confirm before proceeding
- to use a ledger with any command, use `--ledger`
