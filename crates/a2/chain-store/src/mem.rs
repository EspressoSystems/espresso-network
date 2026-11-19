//! A backend in memory, for tests and for a single process that keeps nothing across restarts.

use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Mutex,
};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::watch;

use crate::{Backend, Column, Part, Row};

#[derive(Debug)]
pub struct MemBackend {
    state: Mutex<State>,
    changes: watch::Sender<u64>,
}

#[derive(Debug, Default)]
struct State {
    rows: BTreeMap<u64, Row>,
    parts: BTreeMap<(u64, Part), Bytes>,
    checked_epochs: BTreeSet<u64>,
    floor: u64,
}

impl Default for MemBackend {
    fn default() -> Self {
        Self {
            state: Mutex::default(),
            changes: watch::channel(0).0,
        }
    }
}

impl MemBackend {
    fn write(&self, f: impl FnOnce(&mut State)) {
        f(&mut self.state.lock().unwrap());
        self.changes.send_modify(|n| *n += 1);
    }

    fn read<R>(&self, f: impl FnOnce(&State) -> R) -> R {
        f(&self.state.lock().unwrap())
    }
}

#[async_trait]
impl Backend for MemBackend {
    async fn put(&self, row: Row, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()> {
        self.write(|state| {
            if row.height < state.floor {
                return;
            }
            let height = row.height;
            state.rows.entry(height).or_insert(row);
            for (part, data) in parts {
                state.parts.entry((height, part)).or_insert(data);
            }
        });
        Ok(())
    }

    async fn put_parts(&self, height: u64, parts: Vec<(Part, Bytes)>) -> anyhow::Result<()> {
        self.write(|state| {
            if height < state.floor {
                return;
            }
            for (part, data) in parts {
                state.parts.entry((height, part)).or_insert(data);
            }
        });
        Ok(())
    }

    async fn item(&self, height: u64) -> anyhow::Result<Option<Bytes>> {
        Ok(self.read(|state| state.rows.get(&height).map(|row| row.item.clone())))
    }

    async fn part(&self, height: u64, part: Part) -> anyhow::Result<Option<Bytes>> {
        Ok(self.read(|state| state.parts.get(&(height, part)).cloned()))
    }

    async fn parts(&self, height: u64) -> anyhow::Result<Vec<(Part, Bytes)>> {
        Ok(self.read(|state| {
            Part::ALL
                .into_iter()
                .filter_map(|part| Some((part, state.parts.get(&(height, part))?.clone())))
                .collect()
        }))
    }

    async fn height_of(&self, column: Column, value: &str) -> anyhow::Result<Option<u64>> {
        Ok(self.read(|state| {
            state
                .rows
                .values()
                .find(|row| {
                    let key = match column {
                        Column::Leaf => &row.leaf,
                        Column::Block => &row.block,
                        Column::Payload => &row.payload,
                    };
                    key == value
                })
                .map(|row| row.height)
        }))
    }

    async fn newest(&self) -> anyhow::Result<Option<u64>> {
        Ok(self.read(|state| state.rows.keys().next_back().copied()))
    }

    async fn floor(&self) -> anyhow::Result<u64> {
        Ok(self.read(|state| state.floor))
    }

    async fn missing_items(&self, from: u64, limit: usize) -> anyhow::Result<Vec<Range<u64>>> {
        Ok(self.read(|state| {
            let mut next = from.max(state.floor);
            let mut gaps = Vec::new();
            for &height in state.rows.range(next..).map(|(h, _)| h) {
                if gaps.len() == limit {
                    break;
                }
                if height > next {
                    gaps.push(next..height);
                }
                next = height + 1;
            }
            gaps
        }))
    }

    async fn newest_gap(&self) -> anyhow::Result<Option<Range<u64>>> {
        Ok(self.read(|state| {
            let mut heights = state.rows.range(state.floor..).map(|(&h, _)| h).rev();
            let mut end = heights.next()?;
            for height in heights {
                if height + 1 < end {
                    return Some(height + 1..end);
                }
                end = height;
            }
            (state.floor < end).then_some(state.floor..end)
        }))
    }

    async fn missing_parts(&self, part: Part, from: u64, limit: usize) -> anyhow::Result<Vec<u64>> {
        Ok(self.read(|state| {
            state
                .rows
                .range(from.max(state.floor)..)
                .map(|(&height, _)| height)
                .filter(|height| !state.parts.contains_key(&(*height, part)))
                .take(limit)
                .collect()
        }))
    }

    async fn certs_checked(&self, epoch: u64) -> anyhow::Result<bool> {
        Ok(self.read(|state| state.checked_epochs.contains(&epoch)))
    }

    async fn mark_certs_checked(&self, epoch: u64) -> anyhow::Result<()> {
        self.write(|state| {
            state.checked_epochs.insert(epoch);
        });
        Ok(())
    }

    async fn prune_below(&self, height: u64) -> anyhow::Result<()> {
        self.write(|state| {
            state.floor = state.floor.max(height);
            let floor = state.floor;
            state.rows = state.rows.split_off(&floor);
            state.parts = state.parts.split_off(&(floor, Part::Payload));
        });
        Ok(())
    }

    fn changes(&self) -> watch::Receiver<u64> {
        self.changes.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn conforms() {
        crate::testing::conformance(&MemBackend::default()).await;
    }
}
