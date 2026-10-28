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
- V0_7, `TIMEOUT_EPOCH_VERSION` (also `LARGE_BLOCK_VERSION`, `NO_BUILDER_COMMITMENT_VERSION`, `MAX_SUPPORTED_VERSION`):
  timeout votes and certificates bind the epoch (`TimeoutData3`, `TimeoutVote3`, `TimeoutCertificate3`,
  `CatchupEvidence::Tc3`; form chosen by `UpgradeLock::timeout_epoch_bound`); `Header::V7` is the V0_6 header without
  `builder_commitment`; `LargeBlock` chain-config change raises `max_block_size`, run by the new protocol's upgrade
  sub-protocol (`crates/hotshot/new-protocol/src/upgrade.rs`)

What a network runs: `consensus_genesis{base_version,upgrade_version}` from `/v1/status/metrics`, see
`doc/agents/live-chains.md`. `base_version` and `upgrade_version` in `data/genesis/<network>.toml` are what the next
release configures and can be ahead of the deployed network.
