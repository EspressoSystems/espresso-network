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

`StakeTableEvent` (`crates/espresso/types/src/v0/v0_3/stake_table.rs`) is the full set of `StakeTable` contract
events that affect consensus membership, including the V2/V3 event variants.

A fetcher polls finalized L1 blocks, validates signatures, builds a `ValidatorMap`; `select_active_validator_set()`
drops validators below `max_stake / VID_TARGET_TOTAL_STAKE` and truncates to `MAX_VALIDATORS` by descending stake.
Effective from the next epoch boundary.

## Fast finality

V0_6, `crates/hotshot/new-protocol/`, `doc/stake-table-fast-finality.md`. Replaces CDN + libp2p networking with
`crates/cliquenet/` (fully-connected mesh, x25519-encrypted). Validators register `x25519_key` and `p2p_addr` on the
StakeTable contract for peer discovery.

## Validator startup

`SequencerContext::init` applies the stored anchor's epoch to cliquenet and dials the active validators at the L1
finalized block (`Fetcher::validator_set_at`), bounded to 60 s. Their keys are extra request-response responders;
responders are the table keys and these keys that the network holds at request time. `state_peers` is optional for
validators.

The seed exists when the chain config names a stake table contract (the LC contract address is read from it);
without one the catchup walks only. The follower has no seed and walks at startup (`bootstrap_epoch_window`).

A node behind the stored window is caught up by `EpochMembershipCoordinator::catchup`. When the held pair is more than
one epoch below the target, the `CatchupSeedFn` hook (`crates/espresso/node/src/lc_contract_seed.rs`) runs first, one
seed at a time:

1. Reads `finalizedState` (`F`, `blockCommRoot`) of the LC contract at the L1 finalized block; `E* = epoch(F)`. The seed
   runs when `target > start + 1` and `E* > start + 1`, with `start` the top of the held pair. The window ends at
   `top = min(E*+1, target)`: only roots below `F` are provable, so never above `E*+1`. The DRB of a target at or
   below `E*+1` needs `ST(target-1)` and `ST(target-2)`, which are installed or held.
2. Fetches the header at `F` (sources: step 5) and accepts it iff its height is `F` and its `block_comm_root`
   matches.
3. `Request::HeaderProof { root: F, height }` proves `root(e-2)` for each table `ST(e)` of the window against it, and
   the preceding root where the reward of the lowest table reads its block time. The L1 read, each peer request (30 s)
   and each install are bounded.
4. Installs `ST(max(top-4, start+1, 3)) ..= ST(top)`, each with a block reward computed from the proven roots, each
   root bounded by the catchup timeout. A stake table hash mismatch ends the process; a timeout or another error keeps
   the installed tables and the catchup retries above them. The walk then runs from `top` up to the target (nothing
   for a target at or below `E*+1`); `drb_ready` supplies the target's DRB.
5. Requests go over request-response. With `--state-peers`, they also go over HTTP, eligible `--http-catchup-delay`
   after the fetch starts (one delay for the whole fetch), at once when the protocol is not available. The state peers
   are asked in order; a peer that fails or answers wrongly moves to the back for the rest of the process. When both
   sides fail, the error carries both reasons. The delay is at most a third of the 30 s request bound, and the timeout
   error of a request carries the last state peer error.

## Consensus upgrades

Mechanism and configuration: `doc/upgrades.md`.

1. `UpgradeProposal` broadcast several views before the upgrade
2. Validators vote; enough votes form an `UpgradeCertificate`
3. Certificate attached to subsequent `QuorumProposal`s until the network upgrades

Configured in the genesis TOML, view-based (`start_proposing_view`, `stop_proposing_view`, `start_voting_view`,
`stop_voting_view`) or time-based (same fields as Unix timestamps).
