# AGENTS.md

Guidance for AI coding agents working in this repository.

## Overview

Espresso Network is a confirmation layer for Ethereum rollups, providing fast finality and cross-rollup composability.

- **Espresso node** (`crates/espresso/node/`): Rust binary running consensus and serving APIs
- **HotShot** (`crates/hotshot/`): BFT consensus library
- **Contracts** (`contracts/`): Solidity L1 integration (light client, staking, fees, rewards)
- **Types** (`crates/espresso/types/`): Domain types shared across crates

## Where to look

- `doc/agents/rust.md`
- `doc/agents/solidity.md`

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

- **Submission:** Client POSTs `/submit/submit`. Node validates size, broadcasts to DA committee. Builders accumulate
  transactions.
- **Proposal (leader):** queries builder URLs, selects best block by fee, creates `QuorumProposal`, broadcasts.
- **Validation (all validators):** `ValidatedState::validate_and_apply_header()` computes state transition (fees, L1
  deposits, rewards); validates timestamps, builder signature, height, chain config, size, fees; validates merkle roots
  (fee, block, reward); validates L1 references non-decreasing; if valid, votes.

### L1 integration

Uses **finalized L1 blocks** to avoid reorgs.

- Headers carry `l1_finalized` referencing latest finalized L1 block. Proposal validation enforces non-decreasing.
- Data read from L1: fee deposits (FeeContract), stake table events (ValidatorRegistered, Delegated, etc.).

### Stake table events

`StakeTable` contract events that affect consensus membership:

- `ValidatorRegistered`/`Exit`, `Delegated`/`Undelegated`, `ConsensusKeysUpdated`

A fetcher polls finalized L1 blocks, validates signatures, builds a `ValidatorMap`; `select_active_validator_set()`
picks top 100 by stake. Effective from the next epoch boundary.

### Reward claims

Rewards accumulate in `RewardMerkleTreeV2` (160-level binary tree keyed by Ethereum address). Root is committed in each
header as part of `auth_root`.

1. Query `reward-state-v2/reward-claim-input/{block_height}/{address}` for merkle proof
2. Call `RewardClaim.claimRewards(lifetimeRewards, authData)` on L1
3. Contract verifies proof against `lightClient.authRoot()`
4. Mints `lifetimeRewards - alreadyClaimed` ESP tokens

### Protocol versions

Defined in `crates/espresso/types/src/v0/mod.rs`. `SequencerVersions<Base, Upgrade>` pairs versions for network
operation. **Mainnet currently runs V0_4.**

- V0_1: base Header, ChainConfig, Transaction, ADVZ VID proofs (shipped)
- V0_2, `FeeVersion`: fee support (shipped)
- V0_3, `EpochVersion`: PoS, stake_table_contract, reward_merkle_tree, AvidM VID proofs (shipped)
- V0_4, `DrbAndHeaderUpgradeVersion`: header adds timestamp_millis, total_reward_distributed, RewardMerkleTreeV2
  (**mainnet**)
- V0_5, `EpochRewardVersion`: per-epoch rewards (**next upgrade**)
- V0_6, `NEW_PROTOCOL_VERSION`: DA upgrade + VID2 (AvidmGf2) proofs + cliquenet + new protocol (bundled at 0.6)

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

## Inspecting live chains

Public query-service base URLs:

- Mainnet: `https://query.main.net.espresso.network`
- Decaf testnet: `https://query.decaf.testnet.espresso.network`

Useful paths (append to either base URL):

- `/status/block-height` - current block height
- `/status/version` - running protocol version
- `/availability/header/{height}` - block header (check `version`, `l1_finalized`, `timestamp_millis`)
- `/availability/leaf/{height}` - leaf at height
- `/node/transactions/count` - total tx count
- `/v0/config/hotshot` - HotShot config including `libp2p_config.bootstrap_nodes`
- `/catchup/{height}/...` - state proofs (schema: `crates/espresso/node/api/catchup.toml`)

## Logs

See ./nix/pup/README.md

## Key files

- `justfile` - build/test/deploy commands
- `data/genesis/*.toml` - genesis configurations
- `data/v1/`, `data/v2/`, etc. - reference serialization test vectors
- `doc/upgrades.md` - upgrade mechanism
- `crates/espresso/node/api/*.toml` - API schemas
