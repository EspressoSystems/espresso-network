// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use std::{fmt::Debug, time::Duration};

use anyhow::bail;
use async_trait::async_trait;

#[derive(Clone, Debug)]
pub struct PrunerCfg {
    pruning_threshold: Option<u64>,
    minimum_retention: Duration,
    target_retention: Duration,
    batch_size: u64,
    max_usage: u16,
    interval: Duration,
    incremental_vacuum_pages: u64,
    state_tables: Vec<String>,
}

#[async_trait]
pub trait PruneStorage: PrunerConfig {
    type Pruner: Default + Send;

    async fn get_disk_usage(&self) -> anyhow::Result<u64> {
        Ok(0)
    }

    async fn prune(&self, _pruner: &mut Self::Pruner) -> anyhow::Result<Option<u64>> {
        Ok(None)
    }

    async fn vacuum(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait]
pub trait PrunedHeightStorage: Sized {
    async fn load_pruned_height(&mut self) -> anyhow::Result<Option<u64>> {
        Ok(None)
    }
}

#[async_trait]
pub trait PrunedHeightDataSource: Sized {
    async fn load_pruned_height(&self) -> anyhow::Result<Option<u64>> {
        Ok(None)
    }
}

pub trait PrunerConfig {
    fn set_pruning_config(&mut self, _cfg: PrunerCfg) {}
    fn get_pruning_config(&self) -> Option<PrunerCfg> {
        None
    }
}

impl PrunerCfg {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(pruning_threshold) = self.pruning_threshold {
            if pruning_threshold == 0 {
                bail!("pruning_threshold must be greater than 0 or set to None")
            }
        }

        if self.max_usage > 10000 {
            bail!("max_usage must be less than or equal to 10000")
        }

        Ok(())
    }

    pub fn with_state_tables(mut self, state_tables: Vec<String>) -> Self {
        self.state_tables = state_tables;
        self
    }

    pub fn with_pruning_threshold(mut self, pruning_threshold: u64) -> Self {
        self.pruning_threshold = Some(pruning_threshold);
        self
    }

    pub fn with_minimum_retention(mut self, minimum_retention: Duration) -> Self {
        self.minimum_retention = minimum_retention;
        self
    }

    pub fn with_target_retention(mut self, target_retention: Duration) -> Self {
        self.target_retention = target_retention;
        self
    }

    pub fn with_batch_size(mut self, batch_size: u64) -> Self {
        self.batch_size = batch_size;
        self
    }

    pub fn with_max_usage(mut self, max_usage: u16) -> Self {
        self.max_usage = max_usage;
        self
    }

    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn with_incremental_vacuum_pages(mut self, pages: u64) -> Self {
        self.incremental_vacuum_pages = pages;
        self
    }

    /// Disk space threshold (in bytes).
    ///
    /// If the disk usage exceeds this threshold, pruning of data starts from
    /// the oldest data and continues until the disk usage falls below `MAX_USAGE
    /// or until the oldest data is younger than `MINIMUM_RETENTION`
    pub fn pruning_threshold(&self) -> Option<u64> {
        self.pruning_threshold
    }

    /// Minimum data retention period
    ///
    /// Data younger than this is never pruned, regardless of disk usage.
    pub fn minimum_retention(&self) -> Duration {
        self.minimum_retention
    }

    /// Target data retention period
    ///
    /// This is the ideal period for which data should be retained
    /// data younger than this and older than `MINIMUM_RETENTION` may be pruned if disk usage exceeds the `pruning_threshold`.
    pub fn target_retention(&self) -> Duration {
        self.target_retention
    }

    /// Number of blocks to remove in a single pruning operation.
    pub fn batch_size(&self) -> u64 {
        self.batch_size
    }

    /// Maximum disk usage (in basis points).
    ///
    /// Pruning stops once the disk usage falls below this value, even if
    /// some data older than the `MINIMUM_RETENTION` remains. Values range
    /// from 0 (0%) to 10000 (100%).
    pub fn max_usage(&self) -> u16 {
        self.max_usage
    }

    /// Pruning interval
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// pages to remove from freelist during SQLite vacuuming
    pub fn incremental_vacuum_pages(&self) -> u64 {
        self.incremental_vacuum_pages
    }

    /// State tables to prune
    pub fn state_tables(&self) -> Vec<String> {
        self.state_tables.clone()
    }
}

impl Default for PrunerCfg {
    fn default() -> Self {
        Self {
            // 3 TB
            pruning_threshold: Some(3 * 10_u64.pow(12)),
            // 1 day
            minimum_retention: Duration::from_secs(24 * 3600),
            // 7 days
            target_retention: Duration::from_secs(7 * 24 * 3600),
            batch_size: 30000,
            // 80%
            max_usage: 8000,
            // 1.5 hour
            interval: Duration::from_secs(5400),
            // 8000 pages
            incremental_vacuum_pages: 8000,
            state_tables: Vec::new(),
        }
    }
}
