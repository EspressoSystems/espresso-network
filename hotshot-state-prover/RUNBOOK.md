# HotShot State Prover Runbook

This runbook describes how to operate, configure, and troubleshoot the HotShot State Prover. The implementation details can be found in [`src/service.rs`](./src/service.rs).

## [Overview](README.md)

## Configuration

All configuration is managed via the `StateProverConfig` struct:

- `relay_server` (URL): The relay server endpoint for validators' signatures.
- `sequencer_url` (URL): The sequencer endpoint for fetching consensus related states.
- `provider` (L1 Provider): RPC endpoint to interact with the L1 network.
- `light_client_address` (Address): The deployed Light Client contract address.
- Other parameters: See [`StateProverConfig`](./src/service.rs) for all options.

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

## Main Operations

- **Fetching Latest State:** Uses `fetch_latest_state` to get the current state and signatures from the relay server.
- **Reading Contract State:** Uses `read_contract_state` to get the on-chain state.
- **Proof Generation:** Calls `generate_proof` to create a Plonk proof for the state update.
- **Submitting Proof:** Uses `submit_state_and_proof` to send the proof and state to the contract.
- **Epoch Advancement:** For cross-epoch updates, uses `advance_epoch` to update the contract to a target epoch.

## Health & Monitoring

- The HTTP server (see `start_http_server`) exposes endpoints for health checks and status.
- Check the loggings for troubleshooting.

## Troubleshooting

- **Invalid State or Signatures:** Check logs for `ProverError::InvalidState`. Check if the stake table and `stake_table_capacity` are configured correctly across the sequencers, relay server, and the prover.
- **Contract Error:** Check logs for `ProverError::ContractError`. Ensure the provider urls are valid and the contract address is correct. If there's an error code, search it in the [bindings](../contracts/rust/adapter/src/bindings/lightclientv2.rs) for further debugging information.
- **Gas Price Too High:** Check logs for `ProverError::GasPriceTooHigh`. Adjust the `max_gas_price` in the configuration or wait for the gas price to drop.
- **Proof Generation Failure, Epoch Already Started:** These usually indicate a configuration issue.
- **Network Error:** Ensure that the urls are configured correctly.

## Key References in `service.rs`

- `StateProverConfig`: All configuration parameters.
- `ProverServiceState`: Holds prover state across runs.
- `run_prover_service`, `run_prover_once`: Entrypoints for daemon and one-shot modes.
- `sync_state`: Main loop for fetching, proving, and submitting.
- `generate_proof`, `submit_state_and_proof`, `advance_epoch`: Core logic for proof lifecycle.
