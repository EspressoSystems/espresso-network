# HotShot State Prover Runbook

This runbook describes how to operate, configure, and troubleshoot the HotShot State Prover. The implementation details
can be found in [`src/service.rs`](./src/service.rs).

## [Overview](README.md)

## Configuration

All configuration is managed via the `StateProverConfig` struct and can be set using command-line arguments or
environment variables:

- `relay_server` (URL): The relay server endpoint for validators' signatures.
  - `--relay-server <URL>`
  - `ESPRESSO_STATE_RELAY_SERVER_URL`
- `sequencer_url` (URL): The sequencer endpoint for fetching consensus related states.
  - `--sequencer-url <URL>`
  - `ESPRESSO_SEQUENCER_URL`
- `l1_provider_url` (Vec<Url>): RPC endpoint to interact with the L1 network.
  - `--l1-provider-url <URL>`
  - `ESPRESSO_SEQUENCER_L1_PROVIDER`
- `light_client_address` (Address): The deployed Light Client contract address.
  - `--light-client-address <ADDRESS>`
  - `ESPRESSO_SEQUENCER_LIGHT_CLIENT_PROXY_ADDRESS`
- `eth_mnemonic` (String): Mnemonic phrase for a funded Ethereum wallet.
  - `--eth-mnemonic <MNEMONIC>`
  - `ESPRESSO_SEQUENCER_ETH_MNEMONIC`
- `eth_account_index` (u32): Index of a funded account derived from `eth_mnemonic`.
  - `--eth-account-index <INDEX>`
  - `ESPRESSO_SEQUENCER_STATE_PROVER_ACCOUNT_INDEX`
- `port` (Option<u16>): Port for the HTTP server.
  - `--port <PORT>`
  - `ESPRESSO_PROVER_SERVICE_PORT`
- `stake_table_capacity` (usize): Stake table capacity for the prover circuit.
  - `--stake-table-capacity <CAPACITY>`
  - `ESPRESSO_SEQUENCER_STAKE_TABLE_CAPACITY`
- `max_gas_price` (Option<String>): Max acceptable gas price in Gwei.
  - `--max-gas-price <PRICE>`
  - `ESPRESSO_STATE_PROVER_MAX_GAS_PRICE_IN_GWEI`
- `update_interval` (Duration): The frequency of updating the light client state.
  - `--freq <DURATION>`
  - `ESPRESSO_STATE_PROVER_UPDATE_INTERVAL`
- `retry_interval` (Duration): Interval between retries if a state update fails.
  - `--retry-freq <DURATION>`
  - `ESPRESSO_STATE_PROVER_RETRY_INTERVAL`
- `max_retries` (u64): Maximum number of retries for one-shot prover.
  - `--retries <RETRIES>`
  - `ESPRESSO_STATE_PROVER_ONESHOT_RETRIES`

## Running the Prover

There are two main modes:

### 1. Daemon Mode

Continuously syncs and submits proofs.

```sh
RUST_LOG=info cargo run --release --bin hotshot-state-prover -- --daemon
```

This will invoke `run_prover_service`, which:

- Initializes `ProverServiceState`
- Periodically calls `sync_state` to fetch signatures, generate proofs, and submit to the contract
- Runs an HTTP server for health checks and metrics (see `start_http_server`)

### 2. One-Shot Mode

Runs the prover once for a single state update.

```sh
RUST_LOG=info cargo run --release --bin hotshot-state-prover
```

This will invoke `run_prover_once` and call `sync_state` once.

## Docker

A Docker image is available for the state prover. You can run it using the following command:

```bash
docker run --env-file .env.docker ghcr.io/espressosystems/espresso-sequencer-prover-service:<TAG> [FLAGS/OPTIONS]
```

Make sure to replace `<TAG>` with the desired version.

## Main Operations

- **Fetching Latest State:** Uses `fetch_latest_state` to get the current state and signatures from the relay server.
- **Reading Contract State:** Uses `read_contract_state` to get the on-chain state.
- **Proof Generation:** Calls `generate_proof` to create a Plonk proof for the state update.
- **Submitting Proof:** Uses `submit_state_and_proof` to send the proof and state to the contract.
- **Epoch Advancement:** For cross-epoch updates, uses `advance_epoch` to update the contract to a target epoch.

## Legacy Service

The prover can also run in a legacy mode for older contract versions. The application automatically detects the contract
version and runs the appropriate service. The legacy service implementation can be found in `src/legacy/service.rs`.

## Health & Monitoring

- The HTTP server (see `start_http_server`) exposes endpoints for health checks and status.
- Check the loggings for troubleshooting.

## Troubleshooting

- **Invalid State or Signatures:** Check logs for `ProverError::InvalidState`. Check if the stake table and
  `stake_table_capacity` are configured correctly across the sequencers, relay server, and the prover.
- **Contract Error:** Check logs for `ProverError::ContractError`. Ensure the provider urls are valid and the contract
  address is correct. If there's an error code, search it in the
  [bindings](../contracts/rust/adapter/src/bindings/lightclientv3.rs) for further debugging information.
- **Gas Price Too High:** Check logs for `ProverError::GasPriceTooHigh`. Adjust the `max_gas_price` in the configuration
  or wait for the gas price to drop.
- **Proof Generation Failure, Epoch Already Started:** These usually indicate a configuration issue.
- **Network Error:** Ensure that the urls are configured correctly.

## Key References in `service.rs`

- `StateProverConfig`: All configuration parameters.
- `ProverServiceState`: Holds prover state across runs.
- `run_prover_service`, `run_prover_once`: Entrypoints for daemon and one-shot modes.
- `sync_state`: Main loop for fetching, proving, and submitting.
- `generate_proof`, `submit_state_and_proof`, `advance_epoch`: Core logic for proof lifecycle.
