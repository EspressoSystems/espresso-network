//! Times of the first blocks of the missing rows the page lists, read from block headers.

use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

use chrono::{DateTime, TimeDelta, Utc};
use serde::Deserialize;
use tokio::sync::watch;

use crate::kind::Height;

/// Time between poll rounds of `BlockTimePoller`.
pub const BLOCK_TIME_POLL_INTERVAL: Duration = Duration::from_secs(1);
/// Most headers fetched in one poll round, newest first.
pub const MAX_HEADERS_PER_POLL: usize = 20;
/// Time one poll round may start new header requests in.
pub const HEADER_POLL_BUDGET: Duration = Duration::from_secs(10);
/// Wait before a shown height no node served is asked again.
pub const HEADER_RETRY_INTERVAL: Duration = Duration::from_secs(300);

/// Block time per listed missing row's first block; written by the poller, read by the page.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockTimes(BTreeMap<Height, DateTime<Utc>>);

impl BlockTimes {
    /// The time of the block at `height`; `None` until fetched or while no node serves it.
    pub fn get(&self, height: Height) -> Option<DateTime<Utc>> {
        self.0.get(&height).copied()
    }

    /// Records the time of `height`.
    pub fn insert(&mut self, height: Height, time: DateTime<Utc>) {
        self.0.insert(height, time);
    }

    /// Keeps the heights in `shown`; whether any was dropped.
    pub fn retain_shown(&mut self, shown: &BTreeSet<Height>) -> bool {
        let before = self.0.len();
        self.0.retain(|height, _| shown.contains(height));
        self.0.len() != before
    }
}

/// Publishing side, held by `BlockTimePoller`.
pub type BlockTimesSender = watch::Sender<BlockTimes>;
/// Reading side, held by the page.
pub type BlockTimesReceiver = watch::Receiver<BlockTimes>;

/// A channel holding no time.
pub fn channel() -> (BlockTimesSender, BlockTimesReceiver) {
    watch::channel(BlockTimes::default())
}

/// Shown heights no node served, each with its retry time.
#[derive(Clone, Debug, Default)]
pub struct UnservedHeaders(BTreeMap<Height, DateTime<Utc>>);

impl UnservedHeaders {
    /// Marks `height` unserved until `now + HEADER_RETRY_INTERVAL`.
    pub fn record(&mut self, height: Height, now: DateTime<Utc>) {
        let retry = TimeDelta::from_std(HEADER_RETRY_INTERVAL).expect("a small constant");
        self.0.insert(height, now + retry);
    }

    /// Keeps the heights in `shown`.
    pub fn retain_shown(&mut self, shown: &BTreeSet<Height>) {
        self.0.retain(|height, _| shown.contains(height));
    }

    /// Heights of `shown` without a time in `times` and without a retry after `now`, newest first.
    pub fn to_fetch(
        &self,
        shown: &BTreeSet<Height>,
        times: &BlockTimes,
        now: DateTime<Utc>,
    ) -> Vec<Height> {
        shown
            .iter()
            .rev()
            .filter(|height| times.get(**height).is_none())
            .filter(|height| self.0.get(height).is_none_or(|retry_at| *retry_at <= now))
            .copied()
            .collect()
    }
}

/// `v1/availability/header/{height}`.
pub fn header_route(height: Height) -> String {
    format!("v1/availability/header/{}", height.0)
}

/// The part of a v0.2+ header the watchdog reads.
#[derive(Deserialize)]
pub struct HeaderJson {
    fields: HeaderFields,
}

/// `fields` of a header.
#[derive(Deserialize)]
struct HeaderFields {
    timestamp: u64,
}

impl HeaderJson {
    /// `fields.timestamp` as a time; `None` outside chrono's range.
    pub fn block_time(&self) -> Option<DateTime<Utc>> {
        let seconds = i64::try_from(self.fields.timestamp).ok()?;
        DateTime::from_timestamp(seconds, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn heights(heights: impl IntoIterator<Item = u64>) -> BTreeSet<Height> {
        heights.into_iter().map(Height).collect()
    }

    #[test]
    fn retain_shown_drops_heights_no_longer_shown() {
        let mut times = BlockTimes::default();
        let mut unserved = UnservedHeaders::default();
        for height in [1, 2, 3] {
            times.insert(Height(height), at(height as i64));
        }
        unserved.record(Height(5), at(0));
        unserved.record(Height(6), at(0));

        times.retain_shown(&heights([2, 3, 6]));
        unserved.retain_shown(&heights([2, 3, 6]));

        assert_eq!(times.get(Height(1)), None);
        assert_eq!(times.get(Height(2)), Some(at(2)));
        assert!(unserved.0.contains_key(&Height(6)));
        assert!(!unserved.0.contains_key(&Height(5)));
    }

    #[test]
    fn to_fetch_skips_unserved_heights_for_five_minutes_newest_first() {
        let shown = heights([1, 2, 3, 4]);
        let mut times = BlockTimes::default();
        times.insert(Height(2), at(2));
        let mut unserved = UnservedHeaders::default();
        unserved.record(Height(3), at(1_000));

        let fetch = |now| unserved.to_fetch(&shown, &times, at(now));
        assert_eq!(fetch(1_000), [Height(4), Height(1)]);
        assert_eq!(fetch(1_299), [Height(4), Height(1)]);
        assert_eq!(fetch(1_300), [Height(4), Height(3), Height(1)]);
    }

    #[test]
    fn header_time_decodes_from_fields() {
        let header: HeaderJson =
            serde_json::from_str(r#"{"version":{},"fields":{"timestamp":1727815494}}"#).unwrap();
        assert_eq!(header.block_time(), Some(at(1_727_815_494)));
        let huge = format!(r#"{{"fields":{{"timestamp":{}}}}}"#, u64::MAX);
        let header: HeaderJson = serde_json::from_str(&huge).unwrap();
        assert_eq!(header.block_time(), None);
        let beyond = r#"{"fields":{"timestamp":9223372036854775807}}"#;
        let header: HeaderJson = serde_json::from_str(beyond).unwrap();
        assert_eq!(header.block_time(), None);
    }

    #[test]
    fn a_flat_header_does_not_decode() {
        assert!(serde_json::from_str::<HeaderJson>(r#"{"timestamp":1727815494}"#).is_err());
    }

    #[test]
    fn the_route_names_the_height() {
        assert_eq!(header_route(Height(7)), "v1/availability/header/7");
    }
}
