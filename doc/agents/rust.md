# Rust

## Critical rules

**MUST:**

- Use `-p <package>` for all cargo commands (full workspace builds OOM)
- Run `cargo fmt` and `cargo check -p <package>` after changes
- Run `cargo test -p espresso-types reference` after modifying any serializable type
- Update all three storage backends (PostgreSQL, SQLite, filesystem) when changing persistence

**NEVER:**

- Use `just test` during iteration (use `cargo test -p <package>`)

## Commands

```bash
cargo nextest run -p <package> -- <test_name>

just check                            # postgres + embedded-db variants (pre-commit only)
just lint                             # clippy with -D warnings
just hotshot::test <test_name>        # HotShot consensus tests
just test-demo base                   # basic E2E
just test-demo pos-base               # PoS E2E
just test-slow                        # long-running tests
just demo-native                      # local network via process-compose
```

## Project conventions

- Errors: `anyhow` for binaries, `thiserror` for libraries
- HTTP API: `tide-disco` with TOML schemas (`crates/espresso/node/api/*.toml`, `hotshot-query-service/api/*.toml`)

## Type-driven design

- **Parse, don't validate**: convert inputs to refined types at module boundaries; downstream code takes only refined
  types and does not re-check.
- **Newtype** every domain ID and unit (`BlockHeight`, `ValidatorAddress`, `FeeAmount`). Never pass raw `u64` or
  `[u8; 32]` across module boundaries.
- Replace `bool` parameters, return values and stringly-typed APIs with two-variant enums.
- Make invalid states unrepresentable: enums with associated data over `struct`-of-`Option`; `NonEmpty` over
  `Vec`-that-must-not-be-empty.
- Typestate for multi-step protocols (`Proposal<Unverified>` -> `Proposal<Verified>`); the compiler enforces ordering,
  not runtime asserts.
- Smart constructors return `Result`; keep raw constructors private.
- Push partiality (`panic`, `unwrap`) and Result handling to module edges; interior functions stay total.

## Architecture pointers

- **SequencerContext** (`crates/espresso/node/src/context.rs`): wraps HotShot's `SystemContextHandle`.
- **Node** (`crates/espresso/node/src/lib.rs`): generic over `N: ConnectedNetwork`, `P: SequencerPersistence`.
- **ValidatedState** (`crates/espresso/node/src/state.rs`): three merkle trees (fee accounts, blocks, rewards).
- **HotShot SystemContext** (`crates/hotshot/hotshot/src/lib.rs`): tasks via `ConsensusTaskRegistry`, broadcast channels
  with `HotShotEvent` variants. `EpochMembershipCoordinator` manages per-epoch stake tables.
- **L1Client** (`crates/espresso/types/src/v0/impls/l1.rs`): tracks `head` and `finalized`; reads use
  `BlockId::finalized()`.
- **Stake table fetcher** (`crates/espresso/types/src/v0/impls/stake_table.rs`): polls finalized L1, builds
  `ValidatorMap`; `select_active_validator_set()` picks top 100 by stake.
- **Catchup** (`crates/espresso/node/src/catchup.rs`): `SqlStateCatchup` (local DB), `StatePeers` (HTTP, reliability
  scored), `ParallelStateCatchup` (local first, peers fallback). Fetches fee/reward proofs, block frontier, chain
  config, leaf chain, and `LightClientStateUpdateCertificateV2` state certs per epoch.

## Key crates

- `espresso-node`: main node binary, API, persistence
- `espresso-types`: domain types (Header, Payload, Transaction, ValidatedState)
- `hotshot`: BFT consensus implementation
- `hotshot-query-service`: query APIs for blocks/availability
- `hotshot-state-prover`: ZK proof generation for light client updates
- `hotshot-contract-adapter`: Rust <-> Solidity type bridge
- `staking-cli`: stake table contract interaction
- `cliquenet`: fully-connected mesh network (fast finality)

## Feature flags

- `embedded-db` (off by default): SQLite. Requires different sqlx features than PostgreSQL; features are additive
  globally, so use `espresso-node-sqlite` for SQLite builds.

## Testing

**Blockchain infrastructure: bugs can cause irreversible financial losses.** Write tests that prove correctness, not
just exercise code paths. Regression test first when fixing bugs.

Test layers:

- Unit (`cargo nextest -p <crate>`): individual functions/modules
- Reference (`cargo nextest -p espresso-types reference`): serialization compatibility, in
  `crates/espresso/types/src/reference_tests.rs`
- HotShot (`just hotshot::test <test_name>`): consensus tasks, network sims, in `crates/hotshot/testing/tests/`
- Integration (`cargo nextest run -p tests`): full system E2E in `tests/`
- Slow (`just test-slow`): in `slow-tests/`

When a `reference` test fails after a type change: if intentional, regenerate `/data/` reference files and commitment
constants. If unintentional, revert.

## Storage

Backends:

- PostgreSQL (`sql.rs`): production DA/archival, merklized, pruning supported
- Filesystem (`fs.rs`): production non-DA validators, not merklized, limited pruning
- SQLite (`sql.rs` + `embedded-db`): not yet production, merklized, pruning supported

Migrations (all three backends required when adding storage):

- SQL via Refinery. Naming `V{n}__{name}.sql`.
- Locations: `crates/espresso/node/api/migrations/{postgres,sqlite}/`,
  `hotshot-query-service/migrations/{postgres,sqlite}/`.
- hotshot-query-service uses multiples of 100 (V100, V200...) leaving gaps for applications.
- Filesystem (`crates/espresso/node/src/persistence/fs.rs`): code-based, tracked via `migrated` HashSet. Must be
  recoverable and atomic.
- Update `SequencerPersistence` for all backends; test with `cargo test -p espresso-node persistence`.

## Adding an API endpoint

1. Add route to a `.toml` schema with `PATH`, parameter types, `METHOD`, `DOC`.
2. Implement handler in the corresponding Rust module (e.g., `crates/espresso/node/src/api/endpoints.rs`).
3. Register with `.get("route_name", handler)` or `.at("route_name", handler)`.
4. Add the method to the data source trait.
