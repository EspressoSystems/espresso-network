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
- V0_7, `TIMEOUT_EPOCH_VERSION` (also `MAX_SUPPORTED_VERSION`): timeout certificates bind the epoch they were collected
  in. `TimeoutData3` is a new voteable type whose commitment covers the epoch, so a stored certificate says by its own
  type what its signers covered and verifies without resolving a protocol version. `TimeoutEvidence` holds either form;
  which one a view admits is `UpgradeLock::timeout_epoch_bound`, keyed on the view the certificate justifies so the form
  matches the version of the block carrying it. The V3 wire messages (`TimeoutVote3`, `TimeoutCertificate3`,
  `CatchupEvidence::Tc3`) are appended variants and `TimeoutEvidence` encodes its two older states exactly as the
  `Option<TimeoutCertificate2>` it replaced, so a V0_6 node still decodes everything it is sent before the upgrade and
  the network need not restart. Reuses the V6 header

What a network runs: `base_version` and `upgrade_version` in `data/genesis/<network>.toml`. Live confirmation is
`consensus_genesis{base_version,upgrade_version}` from `/v1/status/metrics`, see `doc/agents/live-chains.md`.
