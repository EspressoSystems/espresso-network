# Architecture

## HotShot vs Espresso Network

- **HotShot** (`crates/hotshot/`): generic BFT consensus. Defines `NodeType`, view-based voting, leader election,
  certificates, networking. Application-agnostic.
- **Espresso Network** (`crates/espresso/node/`, `crates/espresso/types/`): application built on HotShot. Implements
  `NodeType` via `SeqTypes` in `crates/espresso/types/src/v0/mod.rs`. Handles L1 integration, namespaces, fees, rollup
  logic.

## Transaction and block flow

- **Submission:** Client POSTs `/v1/submit/submit`. `BlockBuilder::on_submit_transaction`
  (`crates/hotshot/new-protocol/src/block.rs`) forwards it to the leaders of upcoming views.
- **Proposal (leader):** `BlockBuilder::request_block` builds the block from the leader's pool; the leader proposes it.
- **Validation (all validators):** `ValidatedState::validate_and_apply_header()`
  (`crates/espresso/types/src/v0/impls/state.rs`) computes the state transition (fees, L1 deposits, rewards); validates
  timestamps, builder signature, height, chain config, size, fees; validates merkle roots (fee, block, reward);
  validates L1 references non-decreasing; if valid, votes.

## L1 integration

Reads **finalized L1 blocks** only, to avoid reorgs.

- Headers carry `l1_finalized` referencing the latest finalized L1 block. Proposal validation enforces non-decreasing.
- Data read from L1: fee deposits (FeeContract), stake table events (StakeTable).

## Stake table events

`StakeTableEvent` (`crates/espresso/types/src/v0/v0_3/stake_table.rs:439`) is the full set of `StakeTable` contract
events that affect consensus membership, including the V2/V3 event variants.

A fetcher polls finalized L1 blocks, validates signatures, builds a `ValidatorMap`; `select_active_validator_set()`
drops validators below `max_stake / VID_TARGET_TOTAL_STAKE` and truncates to `MAX_VALIDATORS` by descending stake.
Effective from the next epoch boundary.

## VID

AVID-M over GF(2^16) (`crates/vid/src/avidm_gf2.rs`), systematic: shard indices below the recovery threshold (a third of
total weight) are the payload's own chunks. `disperse_params()` hands out contiguous shard ranges in descending stake
order, so the highest-staked nodes hold the systematic shards. The reconstructor
(`crates/hotshot/new-protocol/src/vid/reconstruct.rs`) races two paths per view: concatenating the systematic shares
once they are all in hand, and erasure-decoding any threshold of shares. Both re-commit the result; the first to succeed
wins. `vid_payloads_*` and `vid_reconstruct_*` metrics show which path wins in practice.
`ESPRESSO_BENCH_VID_SYSTEMATIC_WAIT_MS` (bench knob, default 0) holds the decode back for that long after a threshold of
mixed shares arrives, giving the systematic set a chance to complete first.

## Fast finality

V0_6, `crates/hotshot/new-protocol/`, `doc/stake-table-fast-finality.md`. Replaces CDN + libp2p networking with
`crates/cliquenet/` (fully-connected mesh, x25519-encrypted). Validators register `x25519_key` and `p2p_addr` on the
StakeTable contract for peer discovery.

## Consensus upgrades

Mechanism and configuration: `doc/upgrades.md`.

1. `UpgradeProposal` broadcast several views before the upgrade
2. Validators vote; enough votes form an `UpgradeCertificate`
3. Certificate attached to subsequent `QuorumProposal`s until the network upgrades

Configured in the genesis TOML, view-based (`start_proposing_view`, `stop_proposing_view`, `start_voting_view`,
`stop_voting_view`) or time-based (same fields as Unix timestamps).
