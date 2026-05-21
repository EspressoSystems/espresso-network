# Rust guidance for agents

Read [`../../AGENTS.md`](../../AGENTS.md) first for overview and cross-cutting rules.

## Critical rules

**MUST:**

- Use `-p <package>` for all cargo commands during development (full workspace builds OOM)
- Run `cargo fmt` and `cargo check -p <package>` after changes
- Run `cargo test -p espresso-types reference` after modifying any serializable type
- Update ALL THREE storage backends (PostgreSQL, SQLite, filesystem) when changing persistence
- Write tests that prove correctness, not just exercise code paths

**NEVER:**

- Read from non-finalized L1 blocks (reorg risk)
- Combine `embedded-db` feature with default features (sqlx feature conflict)
- Use `just test` during iteration (compiles everything; use `cargo test -p <package>`)

## Commands

```bash
cargo fmt
cargo check -p <package> --tests
cargo clippy -p <package> --tests
cargo nextest run -p <package> -- <test_name>

just check                            # postgres + embedded-db variants (pre-commit only)
just lint                             # clippy with -D warnings

just test-demo base                   # basic E2E
just test-demo pos-base               # PoS E2E
just demo-native                      # local network via process-compose
```

## Code style

- Imports: `use module::Type` for types, `module::func()` for function calls
- Errors: `anyhow` for binaries, `thiserror` for libraries
- Async: `async_trait` for trait definitions with async methods
- Testing: `#[cfg(test)]` modules in same file; integration tests in `tests/`

## Type-driven design

Use the type system to guide reviewers and the compiler.

- **Parse, don't validate**: convert inputs to refined types at module boundaries; downstream code takes only refined
  types and does not re-check.
- **Newtype** every domain ID and unit (`BlockHeight`, `ValidatorAddress`, `FeeAmount`). Never pass raw `u64` or
  `[u8; 32]` across module boundaries.
- Replace `bool` parameters and stringly-typed APIs with two-variant enums so call sites read self-documenting.
- Make invalid states unrepresentable: enums with associated data over `struct`-of-`Option`; `NonEmpty` over
  `Vec`-that-must-not-be-empty.
- Typestate for multi-step protocols (e.g., `Proposal<Unverified>` -> `Proposal<Verified>`); the compiler enforces
  ordering, not runtime asserts.
- Smart constructors return `Result`; keep raw constructors private so every instance is valid by construction.
- Push partiality (`panic`, `unwrap`, `Result`) to module edges; interior functions stay total.

## Architecture details

### Espresso node

Built around `SequencerContext` (`crates/espresso/node/src/context.rs`) wrapping HotShot's `SystemContextHandle`:

- **Node** (`lib.rs`): generic over network (`N: ConnectedNetwork`) and persistence (`P: SequencerPersistence`)
- **ValidatedState** (`state.rs`): three merkle trees - fee accounts, block commitments, validator rewards
- **API** (`api/`): tide-disco HTTP server with modular endpoints (query, submit, status, catchup, light_client,
  explorer)
- **Persistence** (`persistence/`): pluggable backends implementing `SequencerPersistence`

### HotShot

`SystemContext` (`crates/hotshot/hotshot/src/lib.rs`) is the entry point:

- **NodeType** trait: types for View, Epoch, BlockHeader, BlockPayload, SignatureKey, Transaction, ValidatedState,
  Membership
- **Tasks**: spawned via `ConsensusTaskRegistry`, communicate via broadcast channels using `HotShotEvent` variants
- **View-based consensus**: each view has a deterministic leader. Leader collects QC, fetches transactions from builder,
  creates DA/Quorum proposals. Replicas validate and vote.
- **Epoch membership**: `EpochMembershipCoordinator` manages per-epoch stake tables. Transitions occur at block
  boundaries.

### L1Client

`crates/espresso/types/src/v0/impls/l1.rs`. Tracks L1 `head` and `finalized`; all reads use `BlockId::finalized()`.

### Stake table fetcher

`crates/espresso/types/src/v0/impls/stake_table.rs`. Polls finalized L1 blocks, fetches stake-table events, validates
signatures, builds `ValidatorMap`. `select_active_validator_set()` picks top 100 by stake.

### Catchup

Sparse merkle trees store only necessary paths; missing proofs fetched on-demand during
`ValidatedState::apply_header()`.

Providers in `crates/espresso/node/src/catchup.rs`:

- `SqlStateCatchup` - local DB lookup
- `StatePeers` - remote peer HTTP fetch with reliability scoring
- `ParallelStateCatchup` - local first, falls back to peers

Data fetched: fee proofs, reward proofs, block frontier, chain config, leaf chain for stake table sync. API at
`/catchup/` (schema: `crates/espresso/node/api/catchup.toml`).

## Key crates

- `espresso-node`: main node binary, API, persistence
- `espresso-types`: domain types (Header, Payload, Transaction, ValidatedState)
- `hotshot`: BFT consensus implementation
- `hotshot-query-service`: query APIs for blocks/availability
- `hotshot-state-prover`: ZK proof generation for light client updates
- `hotshot-contract-adapter`: Rust <-> Solidity type bridge
- `staking-cli`: CLI for stake table contract interaction
- `cliquenet`: fully-connected mesh network (fast finality)

## Feature flags

- `embedded-db` (off by default): SQLite backend. Requires different sqlx features than PostgreSQL; since features are
  additive globally, use the `espresso-node-sqlite` crate for SQLite builds.

## Testing

**Blockchain infrastructure: bugs can cause irreversible financial losses.**

- **Correctness over coverage**: tests must prove correctness, not hit line counts
- **Requirements traceability**: each requirement has corresponding test(s)
- **Edge cases mandatory**: boundary conditions, error paths, adversarial inputs
- **Regression test first**: write a failing test before the fix

Agents make writing tests fast. There is no excuse for untested code.

### Test layers

- Unit (`cargo test -p <crate>`): individual functions/modules within crate
- Reference (`cargo test -p espresso-types reference`): serialization compatibility, in
  `crates/espresso/types/src/reference_tests.rs`
- HotShot (`just hotshot::test <test_name>`): consensus tasks, network sims, in `crates/hotshot/testing/tests/`
- Integration (`cargo nextest run -p tests`): full system E2E in `tests/`
- Slow (`just test-slow`): long-running, in `slow-tests/`

### Serialization compatibility

When changing a serializable type:

1. Run `cargo test -p espresso-types reference`
2. If failures are intentional, update reference files in `/data/` and commitment constants
3. If unintentional, revert

## Storage

Node operator docs: <https://docs.espressosys.com/network/guides/node-operators/running-a-sequencer-node>

### Backends

- PostgreSQL (`sql.rs`): production DA/archival, merklized, pruning supported
- Filesystem (`fs.rs`): production non-DA validators, not merklized, limited pruning
- SQLite (`sql.rs` + `embedded-db`): not yet production, merklized, pruning supported

### Migrations

All three backends must be updated when adding storage functionality.

**SQL (PostgreSQL and SQLite)** uses [Refinery](https://github.com/rust-db/refinery):

- Naming: `V{version}__{description}.sql` (e.g., `V501__epoch_tables.sql`)
- Locations: `crates/espresso/node/api/migrations/{postgres,sqlite}/`,
  `hotshot-query-service/migrations/{postgres,sqlite}/`
- hotshot-query-service uses multiples of 100 (V100, V200...) leaving gaps for applications

**Filesystem** (`crates/espresso/node/src/persistence/fs.rs`): code-based migrations tracked via `migrated` HashSet.
Must be recoverable, atomic, tested.

Checklist:

1. Add PostgreSQL migration: `crates/espresso/node/api/migrations/postgres/V{next}__{name}.sql`
2. Add SQLite migration: `crates/espresso/node/api/migrations/sqlite/V{next}__{name}.sql`
3. Update filesystem persistence if data format changes
4. Update `SequencerPersistence` for all backends
5. Test: `cargo test -p espresso-node persistence`

## APIs

[tide-disco](https://github.com/EspressoSystems/tide-disco) with TOML schemas.

Schemas: `crates/espresso/node/api/*.toml`, `hotshot-query-service/api/*.toml`.

Adding an endpoint:

1. Add route to `.toml` with `PATH`, parameter types, `METHOD`, `DOC`
2. Implement handler in the corresponding Rust module (e.g., `crates/espresso/node/src/api/endpoints.rs`)
3. Register with `.get("route_name", handler)` or `.at("route_name", handler)`
4. Ensure data source trait has the required method

## Debugging

- **Compile slow / OOM**: use `-p <package>`. For HotShot tests, `just hotshot::test <name>`.
- **Tests fail after type changes**: `cargo test -p espresso-types reference`. Update `/data/` if intentional.
- **Storage migration failures**: verify all three backends; check version numbers don't conflict.
- **Datadog logs/metrics**: use `pup` (in dev shell). See `nix/pup/README.md`.
- **Live chain state**: see "Inspecting live chains" in [`../../AGENTS.md`](../../AGENTS.md).
