//! What the retry step may probe: the representative, due confirming rows and the recovery queue.

use std::{collections::VecDeque, time::Duration};

use chrono::{DateTime, Utc};

use crate::{
    kind::{Epochs, Height, Probe},
    missing::{MissingRow, MissingSet, RowKey},
};

/// Time per retry step in which probes may start; follow and sweep keep the rest (HEAD's value).
pub const RETRY_BUDGET: Duration = Duration::from_secs(5);

/// Rows yielding probes of at most `CHUNK_LEN` heights, front first.
#[derive(Clone, Debug, Default)]
pub struct ChunkQueue(VecDeque<RowKey>);

impl ChunkQueue {
    /// Every confirmed row of `set` in `rows()` order: the recovery queue.
    pub fn confirmed(set: &MissingSet) -> Self {
        Self(
            set.rows()
                .filter(|row| row.state.is_confirmed())
                .map(|row| row.key)
                .collect(),
        )
    }

    /// `split_first_chunk` of the front row; the rest stays at the front. `None` when empty.
    pub fn next_probe(&mut self) -> Option<Probe> {
        let (probe, rest) = self.0.pop_front()?.split_first_chunk();
        if let Some(rest) = rest {
            self.0.push_front(rest);
        }
        Some(probe)
    }

    /// Drops what lies below `window_start`: ranges are clipped, points go by height, epoch rows
    /// by root block (rows without one stay), as `MissingUpdate::ForgetBelow`.
    pub fn forget_below(&mut self, window_start: Height, epochs: Epochs) {
        self.0.retain_mut(|key| match key {
            RowKey::Range { heights, .. } => {
                heights.start = heights.start.max(window_start);
                heights.start < heights.end
            },
            RowKey::Point { height, .. } => *height >= window_start,
            RowKey::Epoch { kind, epoch } => epochs
                .root_block(*kind, *epoch)
                .is_none_or(|block| block >= window_start),
        });
    }

    /// No row left.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// What one retry step may probe at one `now`.
#[derive(Clone, Debug)]
pub struct RetryQueue {
    /// First chunk of the oldest confirmed row by (`first_missing_at`, first unit, kind), when its
    /// `next_at <= now`.
    pub representative: Option<Probe>,
    /// Confirming rows with `next_at <= now`, in (`next_at`, kind, first unit) order.
    pub confirming: ChunkQueue,
}

impl RetryQueue {
    /// The representative and the due confirming rows of `set` at `now`.
    pub fn new(set: &MissingSet, now: DateTime<Utc>) -> Self {
        let (confirmed, mut confirming): (Vec<MissingRow>, Vec<MissingRow>) =
            set.rows().partition(|row| row.state.is_confirmed());
        let representative = confirmed
            .iter()
            .min_by_key(|row| row.age_order())
            .filter(|row| row.state.retry().next_at <= now)
            .map(|row| row.key.first_probe());
        confirming.retain(|row| row.state.retry().next_at <= now);
        confirming.sort_by_key(|row| {
            (
                row.state.retry().next_at,
                row.key.kind(),
                row.key.first_unit(),
            )
        });
        Self {
            representative,
            confirming: ChunkQueue(confirming.into_iter().map(|row| row.key).collect()),
        }
    }
}
