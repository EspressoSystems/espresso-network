# Protocol versions

`crates/versions/src/lib.rs` is the source of truth for version constants and for `Upgrade { base, target }`.
`crates/espresso/types/src/v0/mod.rs` re-declares 0.2-0.5 as `StaticVersion` aliases (`FeeVersion`, `EpochVersion`,
`DrbAndHeaderUpgradeVersion`, `EpochRewardVersion`). Per-version types live in `crates/espresso/types/src/v0/v0_*/`.

- V0_1: base Header, ChainConfig, Transaction, ADVZ VID proofs
- V0_2, `FEE_VERSION`: fee support
- V0_3, `EPOCH_VERSION`: PoS, stake_table_contract, reward_merkle_tree, AvidM VID proofs
- V0_4, `DRB_AND_HEADER_UPGRADE_VERSION`: header adds timestamp_millis, total_reward_distributed, RewardMerkleTreeV2
- V0_5, `EPOCH_REWARD_VERSION` (also `DRB_FIX_VERSION`): per-epoch rewards; header adds next_stake_table_hash,
  leader_counts
- V0_6, `NEW_PROTOCOL_VERSION`: AvidmGf2 VID proofs, cliquenet, DA upgrade; reuses the V0_5 header; no fees, no L1
  deposit reads, fee tree untouched
- V0_7, `TIMEOUT_EPOCH_VERSION` (also `LARGE_BLOCK_VERSION`, `NO_BUILDER_COMMITMENT_VERSION`):
  timeout votes and certificates bind the epoch (`TimeoutData3`, `TimeoutVote3`, `TimeoutCertificate3`,
  `CatchupEvidence::Tc3`; form chosen by `UpgradeLock::timeout_epoch_bound`); `Header::V7` is the V0_6 header without
  `builder_commitment`; `LargeBlock` chain-config change raises `max_block_size`, run by the new protocol's upgrade
  sub-protocol (`crates/hotshot/new-protocol/src/upgrade.rs`)
- V0_8, `ASYNC_VID_VERSION` (also `MAX_SUPPORTED_VERSION`): votes no longer wait for the block payload. Before it a node
  votes only on a block it holds in full: vote1 needs the parent block reconstructed from VID shares and vote2 the block
  itself, and a node stuck without one fetches the whole payload from a peer. From V0_8 a vote attests to the node's own
  verified VID share; vote2 still waits for that share to be persisted, so a decided block has a quorum of durably
  stored shares and reconstruction runs off the vote path. The gate is `UpgradeLock::payload_gates_votes`, keyed on the
  view the vote is cast in. Consequence: a leader that disperses a non-codeword can now get its block decided; the
  payload is then provably undecodable and the block must be treated as empty, so no network should be upgraded to V0_8
  before the query service can produce that proof for AvidmGf2. `Header::V8` reuses the V0_7 header's fields and commits
  its own version. Reconstruction, the payload fetch and its server still run so nodes on either side of the upgrade
  interoperate; they can be removed once no network runs below V0_8.

What a network runs: `base_version` and `upgrade_version` in `data/genesis/<network>.toml`. Live confirmation is
`consensus_genesis{base_version,upgrade_version}` from `/v1/status/metrics`, see `doc/agents/live-chains.md`.
