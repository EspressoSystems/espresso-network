# AGENTS.md

## Overview

Espresso Network is a confirmation layer for Ethereum rollups.

- **Espresso node** (`crates/espresso/node/`): Rust binary running consensus and serving APIs
- **HotShot** (`crates/hotshot/`): BFT consensus library
- **Contracts** (`contracts/`): Solidity L1 integration (light client, staking, fees, rewards)
- **Types** (`crates/espresso/types/`): Domain types shared across crates

## Where to look

- `doc/agents/rust.md` - before writing Rust: commands, conventions, storage, tests, adding an API endpoint
- `doc/agents/solidity.md` - before touching `contracts/`
- `doc/agents/live-chains.md` - querying mainnet or decaf over the query-service API
- `doc/agents/rewards.md` - reward accumulation and the L1 claim path
- `doc/cargo-features.md` - feature gates for zkVM builds and which functions panic without them
- `doc/upgrades.md` - configuring and running a consensus upgrade
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

## Architecture

### HotShot vs Espresso Network

- **HotShot** (`crates/hotshot/`): generic BFT consensus. Defines `NodeType`, view-based voting, leader election,
  certificates, networking. Application-agnostic.
- **Espresso Network** (`crates/espresso/node/`, `crates/espresso/types/`): application built on HotShot. Implements
  `NodeType` via `SeqTypes` in `crates/espresso/types/src/v0/mod.rs`. Handles L1 integration, namespaces, fees, rollup
  logic.

### Transaction and block flow

- **Submission:** Client POSTs `/v1/submit/submit`. Node validates size, broadcasts to DA committee. Builders accumulate
  transactions.
- **Proposal (leader):** queries builder URLs, selects best block by fee, creates `QuorumProposal`, broadcasts.
- **Validation (all validators):** `ValidatedState::validate_and_apply_header()`
  (`crates/espresso/types/src/v0/impls/state.rs`) computes the state transition (fees, L1 deposits, rewards); validates
  timestamps, builder signature, height, chain config, size, fees; validates merkle roots (fee, block, reward);
  validates L1 references non-decreasing; if valid, votes.

### L1 integration

Uses **finalized L1 blocks** to avoid reorgs.

- Headers carry `l1_finalized` referencing latest finalized L1 block. Proposal validation enforces non-decreasing.
- Data read from L1: fee deposits (FeeContract), stake table events (StakeTable).

### Stake table events

`StakeTableEvent` (`crates/espresso/types/src/v0/v0_3/stake_table.rs:439`) is the full set of `StakeTable` contract
events that affect consensus membership, including the V2/V3 event variants.

A fetcher polls finalized L1 blocks, validates signatures, builds a `ValidatorMap`; `select_active_validator_set()`
drops validators below `max_stake / VID_TARGET_TOTAL_STAKE` and truncates to `MAX_VALIDATORS` (100) by descending stake.
Effective from the next epoch boundary.

### Protocol versions

`crates/versions/src/lib.rs` is the source of truth for version constants and for `Upgrade { base, target }`.
`crates/espresso/types/src/v0/mod.rs:161` re-declares 0.2-0.5 as `StaticVersion` aliases (`FeeVersion`, `EpochVersion`,
`DrbAndHeaderUpgradeVersion`, `EpochRewardVersion`). Per-version types live in `crates/espresso/types/src/v0/v0_*/`.

- V0_1: base Header, ChainConfig, Transaction, ADVZ VID proofs
- V0_2, `FEE_VERSION`: fee support
- V0_3, `EPOCH_VERSION`: PoS, stake_table_contract, reward_merkle_tree, AvidM VID proofs
- V0_4, `DRB_AND_HEADER_UPGRADE_VERSION`: header adds timestamp_millis, total_reward_distributed, RewardMerkleTreeV2
- V0_5, `EPOCH_REWARD_VERSION` (also `DRB_FIX_VERSION`): per-epoch rewards; header adds next_stake_table_hash,
  leader_counts
- V0_6, `NEW_PROTOCOL_VERSION` (also `MAX_SUPPORTED_VERSION`): AvidmGf2 VID proofs, cliquenet, DA upgrade; reuses the
  V0_5 header

Deployed versions are `base_version`/`upgrade_version` in `data/genesis/*.toml`: **mainnet V0_5, decaf V0_6.**

**Fast finality** (V0_6, see `crates/hotshot/new-protocol/` and `doc/stake-table-fast-finality.md`): replaces CDN +
libp2p networking with `crates/cliquenet/` (fully-connected mesh, x25519-encrypted). Validators register `x25519_key`
and `p2p_addr` on the StakeTable contract for peer discovery.

### Consensus upgrades

HotShot upgrades via `UpgradeProposal`. See `doc/upgrades.md`.

1. `UpgradeProposal` broadcast several views before upgrade
2. Validators vote; enough votes form an `UpgradeCertificate`
3. Certificate attached to subsequent `QuorumProposal`s until network upgrades

Configuration in genesis TOML, view-based (`start_proposing_view`, `stop_proposing_view`, `start_voting_view`,
`stop_voting_view`) or time-based (same fields as Unix timestamps).

## Key files

- `justfile` - build/test/deploy commands
- `data/genesis/*.toml` - genesis configurations
- `data/v1/` .. `data/v6/` - reference serialization test vectors
- `crates/versions/src/lib.rs` - protocol version constants
- `crates/espresso/api/src/axum/routes.rs` - every HTTP route path
- `crates/espresso/api/src/v1/`, `crates/espresso/api/src/v2/` - API trait definitions
