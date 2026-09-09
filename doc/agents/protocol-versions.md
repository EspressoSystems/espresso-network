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
- V0_6, `NEW_PROTOCOL_VERSION` (also `MAX_SUPPORTED_VERSION`): AvidmGf2 VID proofs, cliquenet, DA upgrade; reuses the
  V0_5 header

What a network runs: `base_version` and `upgrade_version` in `data/genesis/<network>.toml`. Live confirmation is
`consensus_genesis{base_version,upgrade_version}` from `/v1/status/metrics`, see `doc/agents/live-chains.md`.
