use serde::{Deserialize, Serialize};
use std::fmt::Debug;

use crate::{v0::utils::Timestamp, v0_3::ChainConfig};

/// Represents the specific type of upgrade.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeType {
    Fee { chain_config: ChainConfig },
    Epoch { chain_config: ChainConfig },
}

impl UpgradeType {
    /// Get the upgrade data from `UpgradeType`. As of this writing,
    /// we are only concerned w/ `ChainConfig`.
    pub fn chain_config(&self) -> Option<ChainConfig> {
        match self {
            UpgradeType::Fee { chain_config } => Some(*chain_config),
            UpgradeType::Epoch { chain_config } => Some(*chain_config),
        }
    }
}

/// Represents an upgrade based on time (unix timestamp).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TimeBasedUpgrade {
    /// the earliest unix timestamp in which the node can propose an upgrade
    pub start_proposing_time: Timestamp,
    /// timestamp after which the node stops proposing an upgrade
    pub stop_proposing_time: Timestamp,
    /// The timestamp at which voting for the upgrade proposal starts
    pub start_voting_time: Option<Timestamp>,
    /// The timestamp at which voting for the upgrade proposal stops
    pub stop_voting_time: Option<Timestamp>,
}

/// Represents an upgrade based on view.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ViewBasedUpgrade {
    /// the earliest view in which the node can propose an upgrade
    pub start_proposing_view: u64,
    /// view after which the node stops proposing an upgrade
    pub stop_proposing_view: u64,
    /// The view at which voting for the upgrade proposal starts
    pub start_voting_view: Option<u64>,
    /// The view at which voting for the upgrade proposal stops
    pub stop_voting_view: Option<u64>,
}

/// Represents the specific type of upgrade.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum UpgradeMode {
    /// Upgrade based on unix timestamp.
    Time(TimeBasedUpgrade),
    /// Upgrade based on view.
    View(ViewBasedUpgrade),
}

/// Represents a general upgrade with mode and type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upgrade {
    /// The mode of the upgrade (time-based or view-based).
    pub mode: UpgradeMode,
    /// The type of the upgrade.
    pub upgrade_type: UpgradeType,
}

#[derive(Clone, Copy, Debug)]
pub struct NoStorage;
