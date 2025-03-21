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
    metrics::{MetricsError, PrometheusMetrics},
    QueryError, QueryResult,
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

    async fn elapsed_time_since_last_decide(&self) -> QueryResult<u64> {
        let current_ts = Utc::now().timestamp() as u64;

        let last_decided_time = self
            .consensus_metrics()?
            .get_gauge("last_decided_time")
            .map_err(metrics_err)?
            .get() as u64;

        current_ts
            .checked_sub(last_decided_time)
            .ok_or_else(|| QueryError::Error {
                message: "last_decided_time is in future".into(),
            })
    }

    async fn success_rate(&self) -> QueryResult<f64> {
        let total_views = self
            .consensus_metrics()?
            .get_gauge("current_view")
            .map_err(metrics_err)?
            .get() as f64;
        // By definition, a successful view is any which committed a block.
        Ok(self.block_height().await? as f64 / total_views)
    }
}

pub trait UpdateStatusData {
    fn populate_metrics(&self) -> Box<dyn Metrics>;
}

impl<T: StatusDataSource> UpdateStatusData for T {
    fn populate_metrics(&self) -> Box<dyn Metrics> {
        self.metrics().subgroup("consensus".into())
    }
}

fn metrics_err(err: MetricsError) -> QueryError {
    QueryError::Error {
        message: err.to_string(),
    }
}
