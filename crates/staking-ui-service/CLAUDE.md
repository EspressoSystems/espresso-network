# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a scalable backend service for the Espresso delegation UI, designed to support tens of thousands of simultaneous clients with real-time data updates. The service provides a polling-based REST API that streams blockchain state using a **snapshot + updates** pattern. State is organized into independent modules sourced from either Ethereum L1 (validator registrations, delegations, withdrawals) or Espresso (validator statistics, rewards).

## Build and Development Commands

This project uses `just` as the command runner. All commands should be run from the repository root.

### Essential Commands

- `just test` - Run all unit tests using nextest
- `just lint` - Run clippy lints (with warnings as errors)
- `just fmt` - Format all code with rustfmt
- `just build release` - Build release binaries
- `just run` - Run the service locally with a temporary database
- `just coverage` - Generate HTML code coverage report

### Specialized Commands

- `just run-decaf` - Run against the Decaf testnet
- `just run-local` - Run locally against Docker services (starts demo containers)
- `just build-docker` - Build Docker images (Linux only)

### Demo Commands

- `just demo::up` - Start all demo services (geth L1 devnet + espresso-dev-node + staking service)
- `just demo::logs <service>` - View logs for a specific demo service
- `just demo::down` - Shut down demo services and clean up
- `just demo::pull` - Pull latest Docker images from registry

### Development Notes

- Tests use `cargo nextest` rather than standard `cargo test`
- The project uses Nix for dependency management (`nix develop` for a dev shell)
- Minimum Rust version: 1.90
- Docker builds only work on Linux; Mac users should use published images

## Architecture Overview

### Core Components

**State Management**: The service maintains two independent state machines that run concurrently:

- **L1 State** (`src/input/l1.rs`): Tracks validator registrations, delegations, undelegations, exits, and withdrawals from Ethereum L1 smart contracts
- **Espresso State** (`src/input/espresso.rs`): Tracks active node statistics, participation metrics, and reward calculations from the Espresso network

**Persistence**: SQLite-based storage (`src/persistence/sql.rs`) maintains finalized snapshots. Each state machine keeps:

- One finalized snapshot in the database
- A chain of unfinalized updates in memory (since the finalized block)
- The ability to handle L1 reorgs by discarding and reapplying blocks

**HTTP API**: The REST API (`src/app.rs`) exposes endpoints defined in `api/api.toml` using the `tide-disco` framework. Most endpoints are pure functions of block hash, making them highly cacheable.

### Data Flow

1. **L1 Input Pipeline**:
   - `RpcStream` subscribes to new L1 blocks via WebSocket (or HTTP polling fallback)
   - `RpcCatchup` provides fast-forward mechanism for catching up to current L1 state
   - `MetadataFetcher` downloads validator metadata from off-chain URIs
   - Events from StakeTable and RewardClaim contracts are parsed and applied to state
   - Provider failover mechanism switches between multiple L1 RPC endpoints on errors

2. **Espresso Input Pipeline**:
   - `QueryServiceClient` streams leaves from Espresso query service
   - For each block, calculates leader elections and participation statistics
   - Tracks per-validator proposals, votes, missed slots, and eligible votes
   - Computes rewards distribution based on block production
   - Updates lifetime reward balances in persistent storage

3. **Client Update Pattern**:
   - Clients fetch initial snapshot by block hash (e.g., `/nodes/all/{hash}`)
   - Clients poll for sequential updates (e.g., `/nodes/all/updates/{hash}`)
   - Block hashes ensure consistency and enable aggressive caching
   - Updates contain only diffs since the previous block

### Key Modules

- `src/types/` - Public API types (common, global, wallet)
- `src/input/l1/` - L1 data ingestion (provider, rpc_stream, rpc_catchup, metadata, switching_transport)
- `src/input/espresso/` - Espresso data ingestion (client)
- `src/persistence/` - SQLite persistence layer
- `src/app.rs` - HTTP server and endpoint handlers
- `src/metrics.rs` - Prometheus metrics
- `src/error.rs` - Error types and result handling

### Testing Strategy

- Unit tests are co-located with source files
- Integration tests use mock clients (`testing` modules) to avoid external dependencies
- The `testing` feature flag enables test-only dependencies
- Mock L1: `l1::testing::VecStream` provides deterministic block sequences
- Mock Espresso: `espresso::testing::MockEspressoClient` simulates consensus
- End-to-end tests in `src/main.rs` spin up full Anvil + Espresso networks

## Configuration

Required environment variables:

- `ESPRESSO_STAKING_SERVICE_L1_HTTP` - Comma-separated L1 HTTP RPC endpoints
- `ESPRESSO_STAKING_SERVICE_STAKE_TABLE_ADDRESS` - Stake table contract address
- `ESPRESSO_STAKING_SERVICE_REWARD_CONTRACT_ADDRESS` - Reward claim contract address
- `ESPRESSO_STAKING_SERVICE_ESPRESSO_URL` - Espresso query service URL
- `ESPRESSO_STAKING_SERVICE_DB_PATH` - SQLite database file path

Optional variables for L1 reliability:

- `ESPRESSO_STAKING_SERVICE_L1_WS` - WebSocket RPC endpoints (reduces polling overhead)
- `ESPRESSO_STAKING_SERVICE_L1_RETRY_DELAY` - Delay before retrying failed requests (default: 1s)
- `ESPRESSO_STAKING_SERVICE_L1_POLLING_INTERVAL` - HTTP polling interval (default: 7s)
- `ESPRESSO_STAKING_SERVICE_L1_FREQUENT_FAILURE_TOLERANCE` - Failover threshold for frequent failures (default: 1m window)
- `ESPRESSO_STAKING_SERVICE_L1_CONSECUTIVE_FAILURE_TOLERANCE` - Failover after N consecutive failures (default: 10)

## Important Implementation Details

### L1 State Consistency

The L1 state maintains a vector of `BlockData` where `blocks[i]` corresponds to block `finalized + i`. This ensures:

- `blocks[0]` is always the finalized snapshot (persisted to SQL)
- Subsequent blocks contain only updates (diffs)
- The `blocks_by_hash` map enables O(1) lookup by block hash
- On finalization, old blocks are dropped and `blocks[0]` is updated

### Reward Calculation

Rewards are computed per-block based on:

- The active stake table for the current epoch
- Block production success (leader proposed a block)
- Validator commission rates (from L1 stake table)
- The `RewardDistributor` trait calculates splits between operators and delegators
- Lifetime rewards are accumulated in SQL and exposed via the `/wallet/{address}/rewards/{block}` endpoint

### Active Node Set Statistics

For each Espresso block, the service tracks:

- `proposals`: Number of blocks successfully proposed by this node
- `slots`: Total number of leader slots assigned to this node
- `votes`: Number of votes cast by this node
- `eligible_votes`: Number of blocks where this node was eligible to vote

These are computed using HotShot's DRB-based leader election algorithm (`select_randomized_leader` and stake CDFs).

### Metadata Fetching

Validators can specify a `metadataUri` in their registration. The service:

- Fetches metadata from this URI on registration
- Retries failed fetches every `METADATA_REFRESH_BLOCKS` (currently 10) blocks
- Uses `HttpMetadataFetcher` which supports HTTP/HTTPS URIs
- Gracefully handles missing or invalid metadata

### Provider Failover

The L1 provider uses a sophisticated failover mechanism:

- Multiple providers can be configured (HTTP and/or WebSocket)
- Switches providers on frequent failures (2 failures within 1 minute window)
- Switches on consecutive failures (10 in a row)
- Reverts to first provider after 30 minutes of stability
- Rate limiting support (respects 429 responses)

## Code Patterns and Conventions

- Use `Result<T>` (aliased to `crate::Result<T>`) for all fallible operations
- Errors use `.context()` to add contextual information (via `ResultExt` trait)
- Async state is protected by `async_lock::RwLock` (not `tokio::sync::RwLock`)
- Use `tracing` for logging, not `log` macros
- Metrics are updated via `PrometheusMetrics` struct (thread-safe gauges)
- All persistent state uses SQLx with SQLite backend
- Background tasks should run forever; termination indicates an error
- The `derive_more` crate is used extensively for `Deref`, `DerefMut`, etc.

## Common Gotchas

- The service expects L1 and Espresso to be running before startup. Genesis block loading retries indefinitely.
- Docker builds require Linux (cross-compilation from Mac is not supported)
- Tests using `sequencer` crate require the `testing` feature flag
- The demo uses hardcoded contract addresses (see `justfile`)
- Reward contract address is often set to zero address in test environments
- Metrics endpoint is at `/v0/staking/metrics`, not `/metrics`
- Block hashes in API paths must be lowercase hex without `0x` prefix (handled by alloy's display formatting)
