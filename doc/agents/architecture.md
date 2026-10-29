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
- Data read from L1: fee deposits (FeeContract, below V0_6 only), stake table events (StakeTable).

## Stake table events

`StakeTableEvent` (`crates/espresso/types/src/v0/v0_3/stake_table.rs`) is the full set of `StakeTable` contract events
that affect consensus membership, including the V2/V3 event variants.

A fetcher polls finalized L1 blocks, validates signatures, builds a `ValidatorMap`; `select_active_validator_set()`
drops validators below `max_stake / VID_TARGET_TOTAL_STAKE` and truncates to `MAX_VALIDATORS` by descending stake.
Effective from the next epoch boundary.

## Fast finality

V0_6, `crates/hotshot/new-protocol/`, `doc/stake-table-fast-finality.md`. Replaces CDN + libp2p networking with
`crates/cliquenet/` (fully-connected mesh, x25519-encrypted). Validators register `x25519_key` and `p2p_addr` on the
StakeTable contract for peer discovery.

## Validator startup

`crates/espresso/node/src/startup_catchup.rs`. `catch_up_over_cliquenet` runs in `SequencerContext::init`, before the
`Coordinator` owns the cliquenet instance.

1. L1 phase: reads `finalizedState` and `votingStakeTableState` of the LC contract at the L1 finalized block `b_fin`.
   `E'` is the epoch whose table that commitment is. Recent storage (anchor epoch `>= E' - 1`, tables of
   `anchor - 1 ..= anchor + 1` held) starts as a restart does.
2. Otherwise the stake table events up to `b_fin` give a candidate `ST(E')` whose commitment equals the LC contract's
   (`LcContractCandidateStakeTable::find`) and `ST(now)`. Both only dial peers and verify the anchor leaf. For
   `E' <= first + 1` the seeded genesis table is the candidate.
3. Fill over the node's cliquenet instance with `unicast` and `receive()`: a verified certificate fixes the current
   epoch `C`, a cert2-verified leaf of `E'` anchors, `Request::HeaderProof` proves `root(e-2)` against it, and
   `add_epoch_root` installs the window `max(first + 2, C - 3) ..= C + 1`. DRB leaves of `C` and `C + 1` are fetched in
   the same batch. A lagging prover is walked epoch by epoch.
4. `apply_epoch(C)` dials the exact window and drops the candidate peers.
5. Exits with `NoLcContractCommitmentMatch`, `OwnKeyNotRegistered`, `InstalledCommitmentMismatch` or
   `DeadlineExceeded { stage }`; no HTTP fallback. Header proofs are served only from SQL storage.

## Consensus upgrades

Mechanism and configuration: `doc/upgrades.md`.

1. `UpgradeProposal` broadcast several views before the upgrade
2. Validators vote; enough votes form an `UpgradeCertificate`
3. Certificate attached to subsequent `QuorumProposal`s until the network upgrades

Configured in the genesis TOML, view-based (`start_proposing_view`, `stop_proposing_view`, `start_voting_view`,
`stop_voting_view`) or time-based (same fields as Unix timestamps).
