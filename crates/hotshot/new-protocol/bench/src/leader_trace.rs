//! Bench-side implementation of `LeaderTracer` writing to a per-node CSV.

use std::path::PathBuf;

use anyhow::Result;
use hotshot_new_protocol::leader_trace::{LeaderEvent, LeaderTracer};
use parking_lot::Mutex;
use serde::Serialize;

#[derive(Serialize)]
struct Row {
    view: u64,
    node_id: u64,
    event: &'static str,
    ts_ns: i128,
}

pub struct CsvLeaderTracer {
    node_id: u64,
    path: PathBuf,
    inner: Mutex<Vec<Row>>,
}

impl CsvLeaderTracer {
    pub fn new(node_id: u64, path: PathBuf) -> Self {
        Self {
            node_id,
            path,
            inner: Mutex::new(Vec::with_capacity(8192)),
        }
    }

    /// Flush all buffered rows to disk. Idempotent — leaves the buffer empty.
    pub fn flush(&self) -> Result<()> {
        let mut buf = self.inner.lock();
        let rows: Vec<Row> = std::mem::take(&mut *buf);
        drop(buf);
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut wtr = csv::Writer::from_path(&self.path)?;
        for r in rows {
            wtr.serialize(r)?;
        }
        wtr.flush()?;
        Ok(())
    }
}

impl LeaderTracer for CsvLeaderTracer {
    fn record(&self, view: u64, event: LeaderEvent, ts_ns: i128) {
        self.inner.lock().push(Row {
            view,
            node_id: self.node_id,
            event: event.name(),
            ts_ns,
        });
    }
}
