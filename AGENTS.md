# AGENTS.md

Espresso Network is a confirmation layer for Ethereum rollups. `crates/espresso/` is the application, `crates/hotshot/`
the generic BFT consensus library it is built on, `contracts/` the Solidity L1 side.

## Commands

- Every `cargo` command needs `-p <package>`; a full workspace build OOMs.
- `cargo test -p espresso-types reference` after changing any serializable type.
- `just lint` runs clippy with `-D warnings`. `just --list` for the rest.

## Where to look

- `doc/agents/rust.md` - before writing Rust: commands, conventions, storage, tests, adding an API endpoint
- `doc/agents/solidity.md` - before touching `contracts/`
- `API.md` - the v1 and v2 APIs, and how to add a v2 endpoint
- `doc/agents/architecture.md` - transaction and block flow, L1 reads, stake table, consensus upgrades
- `doc/agents/protocol-versions.md` - what each version changed, and what a network runs
- `doc/agents/live-chains.md` - querying mainnet or decaf over the query-service API
- `doc/agents/rewards.md` - reward accumulation and the L1 claim path
- `doc/cargo-features.md` - feature gates for zkVM builds and which functions panic without them
- `doc/upgrades.md` - configuring and running a consensus upgrade
- `doc/espresso-dev-node.md` - single-process dev node, for rollup integration work
- `doc/pup.md` - `pup` Datadog CLI, for logs and metrics of Espresso's own infrastructure

## Writing Reviewable Code

Reviewing is the bottleneck. Default to changes that minimize reviewer time.

### Diff shape

- Each PR is one self-contained change. Split larger work into a stack.
- Separate refactors and renames from behavior changes. Every commit compiles and passes tests.
- Delete code in its own commit.
- Don't spread small edits across many files when one file would do.

### Reading flow

- Order code top-down: public API first, helpers below. Reviewers read in declaration order.
- Each function fits on one screen. Extract named sub-steps; don't use comments to mark sections.
- Comment the _why_ only when it isn't visible from types or code. Don't narrate the _what_.

### Commits and PRs

- Subject: imperative, scoped, under 70 chars (`feat(stake-table): ...`).
- When work is ready, suggest an updated PR description capturing the final changeset.
- Tell the reviewer where to focus.
- Link the regression test or `reference` test when touching serializable types.

## Key files

- `justfile` - build/test/deploy commands
- `data/genesis/*.toml` - genesis configs, including the protocol version each network runs
- `data/v1/` .. `data/v6/` - reference serialization test vectors
