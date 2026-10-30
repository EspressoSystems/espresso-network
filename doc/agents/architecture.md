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

`SequencerContext::init` does no epoch walk. It applies the stored anchor's epoch to cliquenet and dials `ST(now)`
(`NodeState::l1_finalized_peers`: the active set from stake table events up to the L1 finalized block). `ST(now)` keys
stay extra request-response responders. `state_peers` is optional for validators.

A node behind the stored window is caught up by `EpochMembershipCoordinator::catchup`. When the held pair is more than
one epoch below the target, the `LcContractSeedFn` hook (`crates/espresso/node/src/lc_contract_seed.rs`) runs first, one
seed at a time:

1. Reads `finalizedState` and `votingStakeTableState` of the LC contract at the L1 finalized block; `E'` is the epoch
   whose table that commitment is. A target below `E'` fails fast.
2. Fetches the epoch root leaf `root(E'-2)` unverified and accepts it if its L1-derived table commits to
   `votingStakeTableState`; that table is only the quorum for the anchor.
3. A cert2 under that quorum plus parent links give the anchor `H*(E'-1)+1`; `Request::HeaderProof` proves
   `root(E'-5) ..= root(E'-2)` against it.
4. Installs the tables above the held pair within `ST(E'-3) ..= ST(E'+1)`, and `DRB(E')`; a hash mismatch evicts what it
   installed and errors. The existing walk continues from `E'+1`.

## Consensus upgrades

Mechanism and configuration: `doc/upgrades.md`.

1. `UpgradeProposal` broadcast several views before the upgrade
2. Validators vote; enough votes form an `UpgradeCertificate`
3. Certificate attached to subsequent `QuorumProposal`s until the network upgrades

Configured in the genesis TOML, view-based (`start_proposing_view`, `stop_proposing_view`, `start_voting_view`,
`stop_voting_view`) or time-based (same fields as Unix timestamps).
