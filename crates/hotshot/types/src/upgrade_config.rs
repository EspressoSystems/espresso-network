// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

/// Constants associated with the upgrade process.
pub struct UpgradeConstants {
    /// The offset for how far in the future we will send out a `QuorumProposal` with an `UpgradeCertificate` we form. This is also how far in advance of sending a `QuorumProposal` we begin collecting votes on an `UpgradeProposal`.
    pub propose_offset: u64,

    /// The offset for how far in the future the upgrade certificate we attach should be decided on (or else discarded).
    pub decide_by_offset: u64,

    /// The offset for how far in the future the upgrade actually begins.
    pub begin_offset: u64,

    /// The offset for how far in the future the upgrade ends.
    pub finish_offset: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = ""))]
/// Holds configuration for the upgrade task.
pub struct UpgradeConfig {
    /// View to start proposing an upgrade
    pub start_proposing_view: u64,
    /// View to stop proposing an upgrade. To prevent proposing an upgrade, set stop_proposing_view <= start_proposing_view.
    pub stop_proposing_view: u64,
    /// View to start voting on an upgrade
    pub start_voting_view: u64,
    /// View to stop voting on an upgrade. To prevent voting on an upgrade, set stop_voting_view <= start_voting_view.
    pub stop_voting_view: u64,
    /// Unix time in seconds at which we start proposing an upgrade
    pub start_proposing_time: u64,
    /// Unix time in seconds at which we stop proposing an upgrade. To prevent proposing an upgrade, set stop_proposing_time <= start_proposing_time.
    pub stop_proposing_time: u64,
    /// Unix time in seconds at which we start voting on an upgrade
    pub start_voting_time: u64,
    /// Unix time in seconds at which we stop voting on an upgrade. To prevent voting on an upgrade, set stop_voting_time <= start_voting_time.
    pub stop_voting_time: u64,
    /// Override for `UpgradeConstants::propose_offset`. Falls back to `TYPES::UPGRADE_CONSTANTS.propose_offset` when `None`.
    #[serde(default)]
    pub propose_offset: Option<u64>,
    /// Override for `UpgradeConstants::decide_by_offset`. Falls back to `TYPES::UPGRADE_CONSTANTS.decide_by_offset` when `None`.
    #[serde(default)]
    pub decide_by_offset: Option<u64>,
    /// Override for `UpgradeConstants::begin_offset`. Falls back to `TYPES::UPGRADE_CONSTANTS.begin_offset` when `None`.
    #[serde(default)]
    pub begin_offset: Option<u64>,
    /// Override for `UpgradeConstants::finish_offset`. Falls back to `TYPES::UPGRADE_CONSTANTS.finish_offset` when `None`.
    /// For a zero-blackout upgrade (e.g. 0.4 → 0.8), set `finish_offset == begin_offset`.
    #[serde(default)]
    pub finish_offset: Option<u64>,
}

// Explicitly implementing `Default` for clarity.
#[allow(clippy::derivable_impls)]
impl Default for UpgradeConfig {
    fn default() -> Self {
        UpgradeConfig {
            start_proposing_view: u64::MAX,
            stop_proposing_view: 0,
            start_voting_view: u64::MAX,
            stop_voting_view: 0,
            start_proposing_time: u64::MAX,
            stop_proposing_time: 0,
            start_voting_time: u64::MAX,
            stop_voting_time: 0,
            propose_offset: None,
            decide_by_offset: None,
            begin_offset: None,
            finish_offset: None,
        }
    }
}
