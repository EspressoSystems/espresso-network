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

### Control lane

Opt-in with `--cliquenet-control-lane` (`ESPRESSO_NODE_CLIQUENET_CONTROL_LANE`), off by default; `ControlLane` in
`crates/hotshot/new-protocol/src/network.rs`. cliquenet writes one message at a time on each connection, lowest view
first, so a proposal or vote waits for every VID share queued ahead of it. With the lane on, `Cliquenet` runs a second
cliquenet network with the same key and peers, and `Lane::of` sends everything on it except the bulk, which stays on the
main network: VID shares and fragments, transaction forwards, payload fetch responses and external (request-response)
messages. `receive` reads the lane first. Off, the node runs the single network it always did.

- Port: the lane binds one port above `--cliquenet-bind-address` and dials each peer one port above its registered
  `p2p_addr` (or orchestrator advertise address). Peers learn one address per validator, so the offset is fixed, not
  configured. Main port 0 or 65535 is refused; nodes sharing a host need cliquenet ports at least two apart.
- All or none: a node without the lane never receives the lane's messages. While between one and two thirds of the stake
  has it on, no leader can gather a quorum, so turn it on everywhere in one coordinated restart.
- Failure: each lane connects, retries and queues on its own. A peer whose lane is unreachable gets its shares but no
  proposals, votes or certificates (they wait in the lane's queue until the view is garbage collected; nothing falls
  back to the main network), so it cannot vote. Failing to bind the lane's port fails startup.
- Operations: open the port above the cliquenet port between validators and map it next to the main one behind NAT. The
  lane's cliquenet metrics are in a `control_lane` subgroup.

## Consensus upgrades

Mechanism and configuration: `doc/upgrades.md`.

1. `UpgradeProposal` broadcast several views before the upgrade
2. Validators vote; enough votes form an `UpgradeCertificate`
3. Certificate attached to subsequent `QuorumProposal`s until the network upgrades

Configured in the genesis TOML, view-based (`start_proposing_view`, `stop_proposing_view`, `start_voting_view`,
`stop_voting_view`) or time-based (same fields as Unix timestamps).
