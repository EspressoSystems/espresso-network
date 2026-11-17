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

use async_trait::async_trait;
use chrono::Utc;
use hotshot_types::traits::metrics::Metrics;

use crate::{
    QueryError, QueryResult,
    metrics::{MetricsError, PrometheusMetrics},
};

pub trait HasMetrics {
    fn metrics(&self) -> &PrometheusMetrics;
}

#[async_trait]
pub trait StatusDataSource: HasMetrics {
    async fn block_height(&self) -> QueryResult<usize>;

    fn consensus_metrics(&self) -> QueryResult<PrometheusMetrics> {
        self.metrics()
            .get_subgroup(["consensus"])
            .map_err(metrics_err)
    }

    /// Seconds since the newest decided block, or since the Unix epoch if there is none.
    async fn elapsed_time_since_last_decide(&self) -> QueryResult<u64>;
}

pub trait UpdateStatusData {
    fn populate_metrics(&self) -> Box<dyn Metrics>;
}

impl<T: StatusDataSource> UpdateStatusData for T {
    fn populate_metrics(&self) -> Box<dyn Metrics> {
        self.metrics().subgroup("consensus".into())
    }
}

/// Seconds from `timestamp`, in seconds since the Unix epoch, to now.
pub(crate) fn seconds_since(timestamp: u64) -> QueryResult<u64> {
    (Utc::now().timestamp() as u64)
        .checked_sub(timestamp)
        .ok_or_else(|| QueryError::Error {
            message: "the last decide is in the future".into(),
        })
}

pub(crate) fn metrics_err(err: MetricsError) -> QueryError {
    QueryError::Error {
        message: err.to_string(),
    }
}
