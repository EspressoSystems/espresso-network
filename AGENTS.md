# AGENTS.md

This file provides guidance to AI coding agents when working with code in this repository.

## Overview

Espresso Network is the **global confirmation layer** for Ethereum rollups. Rollups post their blocks to Espresso for
fast finality and cross-rollup composability. This repo contains:

- **Sequencer node** (`sequencer/`): Rust binary for running consensus and serving APIs
- **HotShot** (`crates/hotshot/`): BFT consensus library
- **Contracts** (`contracts/`): Solidity contracts for L1 integration (light client, staking, fees, rewards)
- **Types** (`types/`): Domain types shared across crates

## Critical Rules

**MUST:**

- Use `-p <package>` for all cargo commands during development (full workspace builds cause OOM)
- Update ALL THREE storage backends (PostgreSQL, SQLite, filesystem) when changing persistence
- Run `cargo test -p espresso-types reference` after modifying any serializable type
- Run `cargo fmt` and `cargo check -p <package>` after making changes
- Write tests that prove correctness, not just exercise code paths

**NEVER:**

- Modify V1 contract storage layout (V2 inherits V1; changing V1 breaks upgrades)
- Combine `embedded-db` feature with default features (sqlx feature conflict)
- Use `just test` during iteration (compiles everything; use `cargo test -p <package>`)
- Read from non-finalized L1 blocks (reorg risk)

## Commands

```bash
# Rust - ALWAYS use package-specific commands
cargo fmt
cargo check -p <package> --tests
cargo clippy -p <package> --tests
cargo test -p <package> -- <test_name>

# Full workspace (pre-commit only)
just check                            # Check postgres + embedded-db variants
just lint                             # Clippy with -D warnings
just test                             # Run nextest (skips slow tests)

# Solidity contracts
forge fmt
forge test                            # Unit tests only
just sol-test                         # Full test suite
just gen-bindings                     # Regenerate Rust bindings

# E2E integration tests
just test-demo base                   # Basic integration
just test-demo pos-base               # PoS integration

# Running locally
just demo-native                      # Full local network via process-compose
```

## Code Style

**Rust:**

- Imports: `use module::Type` for types, `module::func()` for function calls
- Errors: `anyhow` for binaries/applications, `thiserror` for libraries
- Async: Prefer `async_trait` for trait definitions with async methods
- Testing: `#[cfg(test)]` modules in same file, integration tests in `tests/`

**Solidity:**

- Use `forge fmt` before committing
- Upgradeable contracts: V2 extends V1, never modify V1 storage
- Events: Emit for all state changes that external systems need to track

## Architecture Overview

### HotShot vs Sequencer Separation

The codebase separates **consensus** (HotShot) from **application logic** (Sequencer):

- **HotShot** (`crates/hotshot/`): Generic BFT consensus library. Defines traits like `NodeType`, handles view-based
  voting, leader election, certificates, and network communication. Application-agnostic.

- **Sequencer** (`sequencer/`, `types/`): Espresso-specific application built on HotShot. Implements `NodeType` via
  `SeqTypes` in `types/src/v0/mod.rs`, defining concrete types for headers, payloads, transactions, and validated state.
  Handles L1 integration, namespaces, fees, and rollup-specific logic.

### Sequencer Internal Architecture

The sequencer is built around `SequencerContext` (`context.rs`), which wraps HotShot's `SystemContextHandle`. Key
components:

- **Node struct** (`lib.rs`): Generic over network (`N: ConnectedNetwork`) and persistence (`P: SequencerPersistence`)
- **ValidatedState** (`state.rs`): Manages three merkle trees - fee accounts, block commitments, and validator rewards
- **API** (`api/`): Tide-disco HTTP server with modular endpoints (query, submit, status, catchup, light_client,
  explorer)
- **Persistence** (`persistence/`): Pluggable storage backends implementing `SequencerPersistence`

### Transaction and Block Flow

**Transaction Submission:**

1. Client submits via HTTP POST to `/submit/submit`
2. Sequencer validates size, broadcasts to DA committee via P2P network
3. Builders listen to the network and accumulate transactions

**Block Proposal (Leader only):**

1. Leader queries configured builder URLs for available blocks
2. Selects best block (by fee), creates `QuorumProposal`, broadcasts to validators

**Block Validation (All validators):**

1. `ValidatedState::validate_and_apply_header()` performs application-level validation
2. Computes state transition: fee charges, L1 deposits, block rewards
3. Validates: timestamps, builder signature, height, chain config, block size, fees
4. Validates merkle roots: fee tree, block tree, reward tree
5. Validates L1 references are non-decreasing
6. If valid, validator votes on the proposal

### HotShot Internal Architecture

`SystemContext` (`crates/hotshot/hotshot/src/lib.rs`) is the main entry point. Key abstractions:

- **NodeType trait**: Defines types for View, Epoch, BlockHeader, BlockPayload, SignatureKey, Transaction,
  ValidatedState, Membership
- **Tasks**: Spawned via `ConsensusTaskRegistry`, communicate through broadcast channels using `HotShotEvent` variants
- **View-based consensus**: Each view has a deterministic leader. Leader collects QC, fetches transactions via builder,
  creates DA/Quorum proposals. Replicas validate and vote.
- **Epoch membership**: `EpochMembershipCoordinator` manages stake tables per epoch. Epoch transitions occur at block
  boundaries.

### L1 Integration

The sequencer uses **only finalized L1 blocks** to avoid reorg issues:

- **L1Client** (`types/src/v0/impls/l1.rs`): Tracks L1 `head` and `finalized` block numbers. Uses `BlockId::finalized()`
  for all reads.
- **Block headers**: Every Espresso header contains `l1_finalized` referencing the latest finalized L1 block. Proposal
  validation enforces this is non-decreasing.
- **Data read from L1**: Fee deposits (FeeContract), stake table events (ValidatorRegistered, Delegated, etc.)

### Stake Table Events

The StakeTable contract emits events that affect consensus membership:

- `ValidatorRegistered/Exit` - Validator registration/deregistration
- `Delegated/Undelegated` - Stake delegation changes
- `ConsensusKeysUpdated` - Key rotation

The `Fetcher` (`types/src/v0/impls/stake_table.rs`) polls finalized L1 blocks, fetches events, validates signatures.
Events transform into a `ValidatorMap`, then `select_active_validator_set()` picks top 100 validators by stake. Changes
affect consensus starting from the next epoch boundary.

### Reward Claims

Rewards accumulate in a `RewardMerkleTreeV2` (160-level binary tree keyed by Ethereum address). The tree root is
committed in each block header as part of `auth_root`.

**Claim flow:**

1. User queries API at `reward-state-v2/reward-claim-input/{block_height}/{address}` for merkle proof
2. Calls `RewardClaim.claimRewards(lifetimeRewards, authData)` on L1
3. Contract verifies merkle proof matches `lightClient.authRoot()`
4. On success, mints `lifetimeRewards - alreadyClaimed` ESP tokens

### Catchup

Nodes use sparse merkle trees (storing only necessary paths). When validating blocks, missing proofs are fetched
on-demand.

**Triggered during** `ValidatedState::apply_header()` when fee accounts or block frontier entries are missing.

**Providers** (`sequencer/src/catchup.rs`):

- `SqlStateCatchup` - Local database lookup
- `StatePeers` - Remote peer HTTP fetch with reliability scoring
- `ParallelStateCatchup` - Tries local first, falls back to peers

**Data fetched:** Fee account proofs, reward account proofs, block merkle frontier, chain config, leaf chain for stake
table sync.

**API:** Endpoints under `/catchup/` serve proof data to peers (schema in `sequencer/api/catchup.toml`).

### Key Crates

| Crate                      | Purpose                                                    |
| -------------------------- | ---------------------------------------------------------- |
| `sequencer`                | Main node binary, API, persistence                         |
| `espresso-types` (types/)  | Domain types: Header, Payload, Transaction, ValidatedState |
| `hotshot`                  | BFT consensus implementation                               |
| `hotshot-query-service`    | Query APIs for blocks and availability data                |
| `hotshot-state-prover`     | ZK proof generation for light client updates               |
| `hotshot-contract-adapter` | Rust-Solidity type bridge                                  |
| `staking-cli`              | CLI for stake table contract interaction                   |

### Key Contracts (`contracts/src/`)

| Contract                              | Purpose                                                 |
| ------------------------------------- | ------------------------------------------------------- |
| `LightClient.sol`                     | Verifies HotShot state proofs, stores block commitments |
| `StakeTable.sol` / `StakeTableV2.sol` | Validator staking, delegations, withdrawals             |
| `FeeContract.sol`                     | Builder fee deposits                                    |
| `EspToken.sol`                        | ESP token (ERC20)                                       |
| `RewardClaim.sol`                     | Validator reward distribution                           |

### Protocol Versions

Versions in `types/src/v0/mod.rs`. `SequencerVersions<Base, Upgrade>` defines version pairs for network operation.

| Version | Alias                        | Key Changes                                                                |
| ------- | ---------------------------- | -------------------------------------------------------------------------- |
| V0_1    | -                            | Base types: Header, ChainConfig, Transaction, ADVZ VID proofs              |
| V0_2    | `FeeVersion`                 | Fee support (version marker)                                               |
| V0_3    | `EpochVersion`               | PoS: stake_table_contract, reward_merkle_tree, AvidM VID proofs            |
| V0_4    | `DrbAndHeaderUpgradeVersion` | Header adds timestamp_millis, total_reward_distributed, RewardMerkleTreeV2 |
| V0_5    | `DaUpgradeVersion`           | DA upgrade (version marker)                                                |
| V0_6    | `Vid2UpgradeVersion`         | VID2 (AvidmGf2) proofs                                                     |

## Consensus Upgrades

HotShot supports protocol upgrades via an `UpgradeProposal` mechanism. See `doc/upgrades.md` for full details.

**How upgrades work:**

1. An `UpgradeProposal` is broadcast several views before the upgrade
2. Validators vote on the proposal; once enough votes are collected, an `UpgradeCertificate` is formed
3. The certificate is attached to subsequent `QuorumProposal`s until the network upgrades

**Configuration** (in genesis TOML):

- View-based: `start_proposing_view`, `stop_proposing_view`, `start_voting_view`, `stop_voting_view`
- Time-based: Same parameters but with Unix timestamps

## Feature Flags

| Feature          | Default | Purpose                  |
| ---------------- | ------- | ------------------------ |
| `fee`            | Yes     | Fee contract integration |
| `pos`            | Yes     | Proof of stake           |
| `drb-and-header` | Yes     | DRB and header upgrades  |
| `da-upgrade`     | Yes     | DA committee upgrades    |
| `embedded-db`    | No      | SQLite backend           |

**IMPORTANT:** `embedded-db` requires sqlx with different features than PostgreSQL. Since Rust features are additive and
global to compilation, use `sequencer-sqlite` crate for SQLite builds.

## Testing

**This is blockchain infrastructure. Bugs can cause irreversible financial losses.**

### Testing Philosophy

- **Correctness over coverage**: Tests must prove the code is correct, not just hit line counts
- **Requirements traceability**: Each requirement should have corresponding test(s)
- **Edge cases are mandatory**: Boundary conditions, error paths, adversarial inputs
- **Regression tests first**: When fixing bugs, write a failing test before the fix

Agents make writing tests fast. There is no excuse for untested code.

### Test Structure

| Layer                   | Location                        | Purpose                                   | Command                                  |
| ----------------------- | ------------------------------- | ----------------------------------------- | ---------------------------------------- |
| Unit tests              | Within crate modules            | Test individual functions/modules         | `cargo test -p <crate>`                  |
| Reference/Serialization | `types/src/reference_tests.rs`  | Verify serialization compatibility        | `cargo test -p espresso-types reference` |
| HotShot tests           | `crates/hotshot/testing/tests/` | Consensus task tests, network simulations | `just hotshot::test <test_name>`         |
| Integration (E2E)       | `tests/`                        | Full system tests                         | `cargo nextest run -p tests`             |
| Slow tests              | `slow-tests/`                   | Long-running tests                        | `just test-slow`                         |
| Contract tests          | `contracts/test/`               | Solidity unit/fuzz/invariant tests        | `just sol-test`                          |

### Serialization Compatibility Tests

**IMPORTANT:** The `types/src/reference_tests.rs` module ensures backward compatibility. If you change a serializable
type:

1. Run `cargo test -p espresso-types reference`
2. If tests fail and change is intentional, update reference files in `/data/` and commitment constants
3. If unintentional, revert your changes

## Storage Layer

For node operator details, see https://docs.espressosys.com/network/guides/node-operators/running-a-sequencer-node

### Storage Backends

| Backend    | Module                   | Production Use          | Merklized State | Pruning |
| ---------- | ------------------------ | ----------------------- | --------------- | ------- |
| PostgreSQL | `sql.rs`                 | Yes (DA/Archival)       | Yes             | Yes     |
| Filesystem | `fs.rs`                  | Yes (non-DA validators) | No              | Limited |
| SQLite     | `sql.rs` + `embedded-db` | Not yet                 | Yes             | Yes     |

### Storage Migrations

**IMPORTANT:** When adding storage functionality, ALL THREE backends must be updated.

**SQL Migrations (PostgreSQL and SQLite):**

- Uses [Refinery](https://github.com/rust-db/refinery) migration framework
- Naming: `V{version}__{description}.sql` (e.g., `V501__epoch_tables.sql`)
- Locations: `sequencer/api/migrations/{postgres,sqlite}/`, `hotshot-query-service/migrations/{postgres,sqlite}/`
- Version numbering: hotshot-query-service uses multiples of 100 (V100, V200...) leaving gaps for applications

**Filesystem Migrations (`sequencer/src/persistence/fs.rs`):**

- Code-based migrations tracked via `migrated` HashSet
- Requirements: Must be recoverable, use atomic file operations, be tested

**Adding storage functionality checklist:**

1. Add PostgreSQL migration: `sequencer/api/migrations/postgres/V{next}__{name}.sql`
2. Add SQLite migration: `sequencer/api/migrations/sqlite/V{next}__{name}.sql`
3. Update filesystem persistence if data format changes
4. Update `SequencerPersistence` trait implementation for all backends
5. Test: `cargo test -p sequencer persistence`

## API Development

APIs use [tide-disco](https://github.com/EspressoSystems/tide-disco) with TOML schema definitions.

**Schema files:** `sequencer/api/*.toml`, `hotshot-query-service/api/*.toml`

**Adding a new endpoint:**

1. Add route to `.toml` file with `PATH`, parameter types, `METHOD`, and `DOC`
2. Implement handler in corresponding Rust module (e.g., `sequencer/src/api/endpoints.rs`)
3. Register handler with `.get("route_name", handler)` or `.at("route_name", handler)`
4. Ensure data source trait has required method

## Debugging

**Compilation slow or OOM:** Use `-p <package>` for all cargo commands. For HotShot tests, use
`just hotshot::test <name>`.

**Tests fail after type changes:** Run `cargo test -p espresso-types reference`. Update `/data/` if change is
intentional.

**Storage migration failures:** Verify all three backends updated. Check version numbers don't conflict.

## Key Files

- `justfile` - All build/test/deploy commands
- `Cargo.toml` - Workspace definition and default members
- `data/genesis/*.toml` - Genesis configurations
- `data/v1/`, `data/v2/`, etc. - Reference serialization test vectors
- `doc/upgrades.md` - Upgrade mechanism documentation
- `sequencer/api/*.toml` - API schema definitions

## Maintaining This Document

**Keep this file up to date.** When making changes that affect build/test commands, architecture, storage backends,
feature flags, or API definitions, update the relevant section.
