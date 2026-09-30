# Protocol versions

`crates/versions/src/lib.rs` is the source of truth for version constants and for `Upgrade { base, target }`.
`crates/espresso/types/src/v0/mod.rs:161` re-declares 0.2-0.5 as `StaticVersion` aliases (`FeeVersion`, `EpochVersion`,
`DrbAndHeaderUpgradeVersion`, `EpochRewardVersion`). Per-version types live in `crates/espresso/types/src/v0/v0_*/`.

- V0_1: base Header, ChainConfig, Transaction, ADVZ VID proofs
- V0_2, `FEE_VERSION`: fee support
- V0_3, `EPOCH_VERSION`: PoS, stake_table_contract, reward_merkle_tree, AvidM VID proofs
- V0_4, `DRB_AND_HEADER_UPGRADE_VERSION`: header adds timestamp_millis, total_reward_distributed, RewardMerkleTreeV2
- V0_5, `EPOCH_REWARD_VERSION` (also `DRB_FIX_VERSION`): per-epoch rewards; header adds next_stake_table_hash,
  leader_counts
- V0_6, `NEW_PROTOCOL_VERSION`: AvidmGf2 VID proofs, cliquenet, DA upgrade; reuses the V0_5 header
- V0_7, `TIMEOUT_EPOCH_VERSION` (also `LARGE_BLOCK_VERSION`): timeout certificates bind the epoch they were collected
  in. `TimeoutData3` is a new voteable type whose commitment covers the epoch, so a stored certificate says by its own
  type what its signers covered and verifies without resolving a protocol version. `TimeoutEvidence` holds either form;
  which one a view admits is `UpgradeLock::timeout_epoch_bound`, keyed on the view the certificate justifies so the form
  matches the version of the block carrying it. The V3 wire messages (`TimeoutVote3`, `TimeoutCertificate3`,
  `CatchupEvidence::Tc3`) are appended variants and `TimeoutEvidence` encodes its two older states exactly as the
  `Option<TimeoutCertificate2>` it replaced, so a V0_6 node still decodes everything it is sent before the upgrade and
  the network need not restart. `Header::V7` reuses the V0_6 header's fields, but commits its own version, so header
  commitments differ across the boundary as they do at every version bump. One consequence to expect in operation: a
  node signs its timeout vote under the epoch it is in, so at an epoch boundary, where nodes disagree about that until
  the boundary certificate has spread, the votes for a view are collected under two epochs and neither may reach a
  threshold. The view times out again and the votes converge as the nodes do. Before V0_7 they are pooled instead, since
  the epoch is then covered by no signature and names no committee. The same upgrade carries the `LargeBlock`
  chain-config change raising `max_block_size`; it is the first upgrade run by the new protocol's own upgrade
  sub-protocol (`crates/hotshot/new-protocol/src/upgrade.rs`).
- V0_8, `ASYNC_VID_VERSION` (also `MAX_SUPPORTED_VERSION`): votes no longer wait for the block payload. Before it a node
  votes only on a block it holds in full: vote1 needs the parent block reconstructed from VID shares and vote2 the block
  itself, and a node stuck without one fetches the whole payload from a peer. From V0_8 a vote attests to the node's own
  verified VID share; vote2 still waits for that share to be persisted, so a decided block has a quorum of durably
  stored shares and reconstruction happens outside consensus. The gate is `UpgradeLock::payload_gates_votes`, keyed on
  the view the vote is cast in. Consequence: a leader that disperses a non-codeword can now get its block decided; the
  payload is then provably undecodable and the block must be treated as empty, so no network should be upgraded to V0_8
  before the query service can produce that proof for AvidmGf2. `Header::V8` reuses the V0_6 header's fields and commits
  its own version. Reconstruction, the payload fetch and its server still run so nodes on either side of the upgrade
  interoperate; they can be removed once no network runs below V0_8.

What a network runs: `base_version` and `upgrade_version` in `data/genesis/<network>.toml`. Live confirmation is
`consensus_genesis{base_version,upgrade_version}` from `/v1/status/metrics`, see `doc/agents/live-chains.md`.
