//! The missing data of one node: rows, updates and the queries the loops and the page read.

use std::{collections::BTreeMap, ops::Range, time::Duration};

use anyhow::ensure;
use chrono::{DateTime, TimeDelta, Utc};
use rangemap::RangeMap;
use strum::VariantArray;

use crate::{
    gaps::Known,
    kind::{
        DataKind, Epoch, EpochKind, Epochs, Height, PointKind, Probe, RangeKind, height_count,
        next_chunk,
    },
    outcome::{Observation, ProbeResult, RangeFindings},
    state::{UnitState, transition},
};

/// The smallest range holding both.
fn hull_of<T: Ord + Copy>(a: &Range<T>, b: &Range<T>) -> Range<T> {
    a.start.min(b.start)..a.end.max(b.end)
}

/// Rows of `map` overlapping `heights`, whole.
fn overlapping_rows(
    map: &RangeMap<Height, UnitState>,
    heights: &Range<Height>,
) -> Vec<(Range<Height>, UnitState)> {
    map.overlapping(heights)
        .map(|(range, state)| (range.clone(), *state))
        .collect()
}

/// Applies `observation` at `now` to the unit `key` of `rows`; whether the row changed.
fn observe_unit<K: Ord + Copy, U: Ord + Copy>(
    rows: &mut BTreeMap<(K, U), UnitState>,
    key: (K, U),
    observation: Observation,
    now: DateTime<Utc>,
) -> bool {
    let old = rows.get(&key).copied();
    let new = transition(old, observation, now);
    match new {
        Some(state) => rows.insert(key, state),
        None => rows.remove(&key),
    };
    new != old
}

/// Applies `observation` at `now` to every unit of `piece`: the part of each row inside `piece`
/// and the heights no row covers (read as `None`) go through `transition` and are written back.
fn observe_piece(
    map: &mut RangeMap<Height, UnitState>,
    piece: Range<Height>,
    observation: Observation,
    now: DateTime<Utc>,
) {
    let mut parts: Vec<(Range<Height>, Option<UnitState>)> = map
        .overlapping(&piece)
        .map(|(row, state)| {
            let inside = row.start.max(piece.start)..row.end.min(piece.end);
            (inside, Some(*state))
        })
        .chain(map.gaps(&piece).map(|gap| (gap, None)))
        .collect();
    parts.sort_by_key(|(range, _)| range.start);
    map.remove(piece);
    for (range, old) in parts {
        if let Some(new) = transition(old, observation, now) {
            map.insert(range, new);
        }
    }
}

/// Removes the rows of `kind` whose unit satisfies `dropped`; the units they covered, `None` when
/// there were none.
fn drop_units<K: Ord + Copy, U: Ord + Copy>(
    rows: &mut BTreeMap<(K, U), UnitState>,
    kind: K,
    dropped: impl Fn(U) -> bool,
    next: impl Fn(U) -> U,
) -> Option<Range<U>> {
    let units: Vec<U> = rows
        .keys()
        .filter(|&&(row_kind, unit)| row_kind == kind && dropped(unit))
        .map(|&(_, unit)| unit)
        .collect();
    let (first, last) = (*units.first()?, *units.last()?);
    for unit in units {
        rows.remove(&(kind, unit));
    }
    Some(first..next(last))
}

/// Units one row covers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKey {
    Range {
        kind: RangeKind,
        heights: Range<Height>,
    },
    Point {
        kind: PointKind,
        height: Height,
    },
    Epoch {
        kind: EpochKind,
        epoch: Epoch,
    },
}

impl RowKey {
    /// The kind label.
    pub fn kind(&self) -> DataKind {
        match self {
            Self::Range { kind, .. } => (*kind).into(),
            Self::Point { kind, .. } => (*kind).into(),
            Self::Epoch { kind, .. } => (*kind).into(),
        }
    }

    /// Heights or epochs covered: range length, or 1.
    pub fn unit_count(&self) -> u64 {
        match self {
            Self::Range { heights, .. } => height_count(heights),
            Self::Point { .. } | Self::Epoch { .. } => 1,
        }
    }

    /// Probe of the row's first `CHUNK_LEN` heights (`kind::next_chunk` from the start), or of the
    /// point or epoch; the page link.
    pub fn first_probe(&self) -> Probe {
        match self {
            Self::Range { kind, heights } => Probe::Range {
                kind: *kind,
                heights: next_chunk(heights.start, heights.end)
                    .expect("a row covers at least one height"),
            },
            Self::Point { kind, height } => Probe::Point {
                kind: *kind,
                height: *height,
            },
            Self::Epoch { kind, epoch } => Probe::Epoch {
                kind: *kind,
                epoch: *epoch,
            },
        }
    }

    /// First unit, for ordering: range start, height or epoch number.
    pub fn first_unit(&self) -> u64 {
        match self {
            Self::Range { heights, .. } => heights.start.0,
            Self::Point { height, .. } => height.0,
            Self::Epoch { epoch, .. } => epoch.0,
        }
    }

    /// The units covered: heights or epoch numbers.
    pub fn units(&self) -> Range<u64> {
        let first = self.first_unit();
        first..first + self.unit_count()
    }

    /// The probe of the first `CHUNK_LEN` heights and the rest of a range key; point and epoch
    /// keys have no rest.
    pub fn split_first_chunk(self) -> (Probe, Option<RowKey>) {
        let probe = self.first_probe();
        match (self, &probe) {
            (Self::Range { kind, heights }, Probe::Range { heights: chunk, .. })
                if chunk.end < heights.end =>
            {
                (
                    probe.clone(),
                    Some(Self::Range {
                        kind,
                        heights: chunk.end..heights.end,
                    }),
                )
            },
            _ => (probe, None),
        }
    }
}

/// A key and its state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingRow {
    pub key: RowKey,
    pub state: UnitState,
}

impl MissingRow {
    /// Orders rows by (`first_missing_at`, first unit, kind): oldest first.
    pub fn age_order(&self) -> (DateTime<Utc>, u64, DataKind) {
        (
            self.state.first_missing_at(),
            self.key.first_unit(),
            self.key.kind(),
        )
    }
}

/// The only kinds of change to a missing set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissingUpdate {
    /// A probe result.
    Observed(ProbeResult),
    /// Drops heights below the window start: range rows are trimmed, point rows below go, epoch
    /// rows whose root block is below go. `ForgetBelow(Height(0))` changes nothing and returns no
    /// region (`RangeMap::remove` panics on an empty range).
    ForgetBelow(Height),
}

/// Units of one kind whose image rows an update rewrote; one variant per unit family.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageRegion {
    Range {
        kind: RangeKind,
        heights: Range<Height>,
    },
    Points {
        kind: PointKind,
        heights: Range<Height>,
    },
    Epochs {
        kind: EpochKind,
        epochs: Range<Epoch>,
    },
}

impl ImageRegion {
    /// The kind label and the units the region covers.
    pub fn units(&self) -> (DataKind, Range<u64>) {
        match self {
            Self::Range { kind, heights } => ((*kind).into(), heights.start.0..heights.end.0),
            Self::Points { kind, heights } => ((*kind).into(), heights.start.0..heights.end.0),
            Self::Epochs { kind, epochs } => ((*kind).into(), epochs.start.0..epochs.end.0),
        }
    }
}

/// What one update rewrote, for the write-through image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageDiff {
    pub regions: Vec<ImageRegion>,
    /// Row count before minus after, saturating at 0; the `forgotten` count of `ForgetBelow`.
    pub rows_removed: usize,
}

/// Missing data of one node in memory; SQLite holds its image. Range rows of a kind are disjoint,
/// and touching rows differ in state (`RangeMap` coalesces equal neighbours).
#[derive(Clone, Debug)]
pub struct MissingSet {
    epochs: Epochs,
    ranges: BTreeMap<RangeKind, RangeMap<Height, UnitState>>,
    points: BTreeMap<(PointKind, Height), UnitState>,
    epoch_rows: BTreeMap<(EpochKind, Epoch), UnitState>,
}

impl MissingSet {
    /// Empty set.
    pub fn new(epochs: Epochs) -> Self {
        Self {
            epochs,
            ranges: BTreeMap::new(),
            points: BTreeMap::new(),
            epoch_rows: BTreeMap::new(),
        }
    }

    /// From image rows; errors on range rows of a kind that overlap (`overlapping` non-empty
    /// before `insert`), or touch with equal state (row count after loading below the input
    /// count).
    pub fn from_rows(
        epochs: Epochs,
        rows: impl IntoIterator<Item = MissingRow>,
    ) -> anyhow::Result<Self> {
        let mut set = Self::new(epochs);
        let mut input_rows = 0;
        for MissingRow { key, state } in rows {
            input_rows += 1;
            match key {
                RowKey::Range { kind, heights } => {
                    ensure!(
                        heights.start < heights.end,
                        "empty {kind:?} row {heights:?}"
                    );
                    let map = set.ranges.entry(kind).or_default();
                    ensure!(
                        map.overlapping(&heights).next().is_none(),
                        "overlapping {kind:?} rows at {heights:?}"
                    );
                    map.insert(heights, state);
                },
                RowKey::Point { kind, height } => {
                    ensure!(
                        set.points.insert((kind, height), state).is_none(),
                        "duplicate {kind:?} row at {height}"
                    );
                },
                RowKey::Epoch { kind, epoch } => {
                    ensure!(
                        set.epoch_rows.insert((kind, epoch), state).is_none(),
                        "duplicate {kind:?} row at epoch {}",
                        epoch.0
                    );
                },
            }
        }
        ensure!(
            set.row_count() == input_rows,
            "touching rows with equal state in the image"
        );
        Ok(set)
    }

    /// Applies one update at `now`: a probe result applies `transition` to each piece it covers
    /// (range rows split at piece bounds, units without a row read `None`); returns the regions it
    /// rewrote. The only mutator.
    pub fn apply(&mut self, update: MissingUpdate, now: DateTime<Utc>) -> ImageDiff {
        let rows_before = self.row_count();
        let regions = match update {
            MissingUpdate::Observed(result) => self.observe(result, now).into_iter().collect(),
            MissingUpdate::ForgetBelow(window_start) => self.forget_below(window_start),
        };
        ImageDiff {
            regions,
            rows_removed: rows_before.saturating_sub(self.row_count()),
        }
    }

    fn row_count(&self) -> usize {
        self.ranges.values().map(RangeMap::len).sum::<usize>()
            + self.points.len()
            + self.epoch_rows.len()
    }

    /// Applies one probe result; the region it rewrote, `None` when nothing changed.
    fn observe(&mut self, result: ProbeResult, now: DateTime<Utc>) -> Option<ImageRegion> {
        match result {
            ProbeResult::Range { kind, findings } => self.observe_range(kind, &findings, now),
            ProbeResult::Point {
                kind,
                height,
                observation,
            } => observe_unit(&mut self.points, (kind, height), observation, now).then_some(
                ImageRegion::Points {
                    kind,
                    heights: height..height.next(),
                },
            ),
            ProbeResult::Epoch {
                kind,
                epoch,
                observation,
            } => observe_unit(&mut self.epoch_rows, (kind, epoch), observation, now).then_some(
                ImageRegion::Epochs {
                    kind,
                    epochs: epoch..epoch.next(),
                },
            ),
        }
    }

    /// Applies the pieces of `findings`; the region (the hull of the probed range and the rows
    /// overlapping it before and after) when a row changed, `None` otherwise.
    fn observe_range(
        &mut self,
        kind: RangeKind,
        findings: &RangeFindings,
        now: DateTime<Utc>,
    ) -> Option<ImageRegion> {
        let map = self.ranges.entry(kind).or_default();
        let probed = findings.heights();
        let before = overlapping_rows(map, probed);
        let region = before
            .iter()
            .fold(probed.clone(), |hull, (range, _)| hull_of(&hull, range));
        for (piece, observation) in findings.pieces() {
            observe_piece(map, piece, observation, now);
        }
        let after = overlapping_rows(map, &region);
        if before == after {
            return None;
        }
        let heights = after
            .iter()
            .fold(region, |hull, (range, _)| hull_of(&hull, range));
        Some(ImageRegion::Range { kind, heights })
    }

    /// Drops what lies below `window_start`; the rewritten regions.
    fn forget_below(&mut self, window_start: Height) -> Vec<ImageRegion> {
        if window_start == Height(0) {
            return Vec::new();
        }
        let mut regions = Vec::new();
        let below = Height(0)..window_start;
        for (&kind, map) in &mut self.ranges {
            let rows = overlapping_rows(map, &below);
            if let Some(heights) = rows
                .iter()
                .map(|(range, _)| range.clone())
                .reduce(|hull, range| hull_of(&hull, &range))
            {
                map.remove(below.clone());
                regions.push(ImageRegion::Range { kind, heights });
            }
        }
        for &kind in PointKind::VARIANTS {
            let dropped = |height: Height| height < window_start;
            if let Some(heights) = drop_units(&mut self.points, kind, dropped, Height::next) {
                regions.push(ImageRegion::Points { kind, heights });
            }
        }
        let epochs = self.epochs;
        for &kind in EpochKind::VARIANTS {
            let dropped = |epoch: Epoch| {
                epochs
                    .root_block(kind, epoch)
                    .is_some_and(|block| block < window_start)
            };
            if let Some(epochs) = drop_units(&mut self.epoch_rows, kind, dropped, Epoch::next) {
                regions.push(ImageRegion::Epochs { kind, epochs });
            }
        }
        regions
    }

    /// The epochs the set judges epoch rows with; also the `probes` argument.
    pub fn epochs(&self) -> Epochs {
        self.epochs
    }

    /// Every row, kinds in `DataKind` order, units ascending.
    pub fn rows(&self) -> impl Iterator<Item = MissingRow> + '_ {
        let ranges = self.ranges.iter().flat_map(|(&kind, map)| {
            map.iter().map(move |(heights, state)| MissingRow {
                key: RowKey::Range {
                    kind,
                    heights: heights.clone(),
                },
                state: *state,
            })
        });
        let points = self
            .points
            .iter()
            .map(|(&(kind, height), state)| MissingRow {
                key: RowKey::Point { kind, height },
                state: *state,
            });
        let epochs = self
            .epoch_rows
            .iter()
            .map(|(&(kind, epoch), state)| MissingRow {
                key: RowKey::Epoch { kind, epoch },
                state: *state,
            });
        ranges.chain(points).chain(epochs)
    }

    /// Rows overlapping `region`; each lies inside it after the update that returned it.
    pub fn rows_in(&self, region: &ImageRegion) -> Vec<MissingRow> {
        match region {
            ImageRegion::Range { kind, heights } => {
                self.ranges.get(kind).map_or_else(Vec::new, |map| {
                    overlapping_rows(map, heights)
                        .into_iter()
                        .map(|(heights, state)| MissingRow {
                            key: RowKey::Range {
                                kind: *kind,
                                heights,
                            },
                            state,
                        })
                        .collect()
                })
            },
            ImageRegion::Points { kind, heights } => self
                .points
                .range((*kind, heights.start)..(*kind, heights.end))
                .map(|(&(kind, height), state)| MissingRow {
                    key: RowKey::Point { kind, height },
                    state: *state,
                })
                .collect(),
            ImageRegion::Epochs { kind, epochs } => self
                .epoch_rows
                .range((*kind, epochs.start)..(*kind, epochs.end))
                .map(|(&(kind, epoch), state)| MissingRow {
                    key: RowKey::Epoch { kind, epoch },
                    state: *state,
                })
                .collect(),
        }
    }

    /// `Known::Gap` when `heights` lies inside one `NotFound` row of `kind`, else `Known::Missing`.
    pub fn known(&self, kind: RangeKind, heights: &Range<Height>) -> Known {
        let inside_not_found = self
            .ranges
            .get(&kind)
            .and_then(|map| map.get_key_value(&heights.start))
            .is_some_and(|(row, state)| {
                row.end >= heights.end && matches!(state, UnitState::NotFound { .. })
            });
        if inside_not_found {
            Known::Gap
        } else {
            Known::Missing
        }
    }

    /// Range end, point height + 1, or epoch root block (`None` when `Off` or on overflow).
    pub fn alert_height(&self, key: &RowKey) -> Option<Height> {
        match key {
            RowKey::Range { heights, .. } => Some(heights.end),
            RowKey::Point { height, .. } => Some(height.next()),
            RowKey::Epoch { kind, epoch } => self.epochs.root_block(*kind, *epoch),
        }
    }

    /// One per `DataKind`: units missing, the oldest row by (`first_missing_at`, first unit), and
    /// the confirmed units whose `alert_height` is at or below `settled_end`, split at
    /// `counted_until`.
    pub fn tallies(&self, settled_end: Height, counted_until: DateTime<Utc>) -> Vec<KindTally> {
        let mut tallies: Vec<KindTally> = DataKind::VARIANTS
            .iter()
            .map(|&kind| KindTally {
                kind,
                missing_count: 0,
                oldest: None,
                confirmed: ConfirmedUnits::default(),
            })
            .collect();
        for row in self.rows() {
            let settled = self
                .alert_height(&row.key)
                .is_some_and(|height| height <= settled_end);
            let tally = tallies
                .iter_mut()
                .find(|tally| tally.kind == row.key.kind())
                .expect("a tally exists per kind");
            tally.missing_count += row.key.unit_count();
            if settled && row.state.is_confirmed() {
                tally.confirmed.add(&row, counted_until);
            }
            let older = tally
                .oldest
                .as_ref()
                .is_none_or(|old| row.age_order() < old.age_order());
            if older {
                tally.oldest = Some(row);
            }
        }
        tallies
    }

    /// Up to `limit` rows by (`first_missing_at`, first unit, kind), and the total.
    pub fn listed(&self, limit: usize) -> ListedRows {
        let mut rows: Vec<_> = self.rows().collect();
        let total = rows.len() as u64;
        rows.sort_by_key(MissingRow::age_order);
        rows.truncate(limit);
        ListedRows { shown: rows, total }
    }
}

/// The age from which a confirmed unit counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlertRule {
    pub min_age: Duration,
}

impl AlertRule {
    /// `now - min_age`: units first missing at or before it count.
    pub fn counted_until(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        now - TimeDelta::from_std(self.min_age).expect("min_age fits a chrono delta")
    }
}

/// Units missing of one kind, its oldest row and its confirmed settled units; valid from the
/// publish on.
#[derive(Clone, Debug, PartialEq)]
pub struct KindTally {
    pub kind: DataKind,
    pub missing_count: u64,
    pub oldest: Option<MissingRow>,
    pub confirmed: ConfirmedUnits,
}

/// Confirmed settled units of one kind, split at the publish's `counted_until`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfirmedUnits {
    /// Units first missing at or before the publish's `counted_until`.
    pub aged: u64,
    /// Younger units per `first_missing_at`; at most one entry per second of `min_age` (60 at the
    /// default).
    pub young: BTreeMap<DateTime<Utc>, u64>,
}

impl ConfirmedUnits {
    /// Adds the units of the confirmed `row`, split at `counted_until`.
    fn add(&mut self, row: &MissingRow, counted_until: DateTime<Utc>) {
        let units = row.key.unit_count();
        let first = row.state.first_missing_at();
        if first <= counted_until {
            self.aged += units;
        } else {
            *self.young.entry(first).or_default() += units;
        }
    }

    /// `aged` plus the `young` units first missing at or before `counted_until`; the confirmed
    /// missing count of a kind. A `counted_until` below the publish's (clock stepped back) still
    /// counts every `aged` unit.
    pub fn count(&self, counted_until: DateTime<Utc>) -> u64 {
        self.aged
            + self
                .young
                .range(..=counted_until)
                .map(|(_, n)| n)
                .sum::<u64>()
    }
}

/// The oldest rows of a node and the count of all of them.
#[derive(Clone, Debug, PartialEq)]
pub struct ListedRows {
    pub shown: Vec<MissingRow>,
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use std::{num::NonZeroU64, str::FromStr};

    use pretty_assertions::assert_eq;
    use proptest::{collection::vec, option, prelude::*};
    use reqwest::StatusCode;
    use sqlx::sqlite::SqliteConnectOptions;

    use super::*;
    use crate::{
        kind::{CHUNK_LEN, EpochConfig, KindClass},
        outcome::{FailedStatus, RequestFailure},
        retry::{ChunkQueue, RetryQueue},
        state::{Retry, RetryDelay},
        store::{PersistedMissingSet, Store},
    };

    const T0: i64 = 1_000;
    const MIN_AGE: Duration = Duration::from_secs(60);

    fn epochs(height: u64, start: u64) -> Epochs {
        Epochs::On(EpochConfig {
            epoch_height: NonZeroU64::new(height).unwrap(),
            epoch_start_block: Height(start),
        })
    }

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn h(range: Range<u64>) -> Range<Height> {
        Height(range.start)..Height(range.end)
    }

    fn server_error() -> RequestFailure {
        RequestFailure::Status(FailedStatus::new(StatusCode::INTERNAL_SERVER_ERROR).unwrap())
    }

    fn range_result(kind: RangeKind, range: Range<u64>, observation: Observation) -> ProbeResult {
        ProbeResult::Range {
            kind,
            findings: RangeFindings::uniform(h(range), observation),
        }
    }

    /// A range whose search found one `gap` and left the rest present.
    fn gap_result(kind: RangeKind, range: Range<u64>, gap: Range<u64>) -> ProbeResult {
        let mut findings = RangeFindings::new(h(range));
        findings.mark(h(gap), Observation::NotFound);
        ProbeResult::Range { kind, findings }
    }

    fn point_result(kind: PointKind, height: u64, observation: Observation) -> ProbeResult {
        ProbeResult::Point {
            kind,
            height: Height(height),
            observation,
        }
    }

    fn epoch_result(kind: EpochKind, epoch: u64, observation: Observation) -> ProbeResult {
        ProbeResult::Epoch {
            kind,
            epoch: Epoch(epoch),
            observation,
        }
    }

    fn observe(set: &mut MissingSet, result: ProbeResult, secs: i64) -> ImageDiff {
        set.apply(MissingUpdate::Observed(result), at(secs))
    }

    /// `times` observations at `first`, `first + 1`, ...
    fn repeat(set: &mut MissingSet, result: &ProbeResult, times: i64, first: i64) {
        for i in 0..times {
            observe(set, result.clone(), first + i);
        }
    }

    fn leaf_gap(range: Range<u64>) -> ProbeResult {
        range_result(RangeKind::Leaf, range, Observation::NotFound)
    }

    fn range_key(kind: RangeKind, range: Range<u64>) -> RowKey {
        RowKey::Range {
            kind,
            heights: h(range),
        }
    }

    fn tally(set: &MissingSet, kind: DataKind) -> KindTally {
        set.tallies(Height(10_000), at(T0 + 10_000))
            .into_iter()
            .find(|tally| tally.kind == kind)
            .unwrap()
    }

    fn count(set: &MissingSet, kind: DataKind) -> u64 {
        tally(set, kind).missing_count
    }

    fn keys(set: &MissingSet) -> Vec<RowKey> {
        set.rows().map(|row| row.key).collect()
    }

    /// Confirmed settled units of every kind that count at `counted_until`.
    fn confirmed_count(
        set: &MissingSet,
        settled_end: u64,
        counted_until: i64,
    ) -> Vec<(DataKind, u64)> {
        let rule = AlertRule { min_age: MIN_AGE };
        let counted_until = rule.counted_until(at(counted_until));
        set.tallies(Height(settled_end), counted_until)
            .into_iter()
            .map(|tally| (tally.kind, tally.confirmed.count(counted_until)))
            .filter(|(_, count)| *count > 0)
            .collect()
    }

    #[test]
    fn missing_backoff_and_tallies() {
        let mut set = MissingSet::new(epochs(30, 61));
        assert!(
            set.tallies(Height(1_000), at(T0))
                .iter()
                .all(|t| t.missing_count == 0 && t.oldest.is_none())
        );
        assert_eq!(
            set.tallies(Height(1_000), at(T0)).len(),
            DataKind::VARIANTS.len()
        );

        observe(&mut set, leaf_gap(0..100), T0);
        observe(
            &mut set,
            epoch_result(EpochKind::StakeTable, 5, Observation::NotFound),
            T0 + 50,
        );
        let next_at = |set: &MissingSet| set.rows().next().unwrap().state.retry().next_at;
        assert_eq!(next_at(&set), at(T0 + 5));

        // The delay doubles from 5 s; a row with 3 404s stops at 60 s.
        let mut now = T0 + 5;
        for delay in [10, 20, 40, 60, 60, 60] {
            observe(&mut set, leaf_gap(0..100), now);
            assert_eq!(next_at(&set), at(now + delay));
            now += delay;
        }

        let leaf = tally(&set, DataKind::Leaf);
        assert_eq!(leaf.oldest.unwrap().key, range_key(RangeKind::Leaf, 0..100));
        assert_eq!(leaf.missing_count, 100);
        assert_eq!(count(&set, DataKind::StakeTable), 1);

        // A later row does not displace the oldest. Touching rows with the same state merge.
        for (kind, range, secs) in [
            (RangeKind::Leaf, 100..200, 20),
            (RangeKind::Block, 300..400, 30),
            (RangeKind::Block, 200..300, 30),
        ] {
            observe(
                &mut set,
                range_result(kind, range, Observation::NotFound),
                T0 + secs,
            );
        }
        assert_eq!(count(&set, DataKind::Leaf), 200);
        assert_eq!(count(&set, DataKind::Block), 200);
        let block = tally(&set, DataKind::Block).oldest.unwrap();
        assert_eq!(block.key, range_key(RangeKind::Block, 200..400));

        let page = set.listed(3);
        let order: Vec<_> = page.shown.iter().map(|row| row.key.clone()).collect();
        assert_eq!(
            order,
            [
                range_key(RangeKind::Leaf, 0..100),
                range_key(RangeKind::Leaf, 100..200),
                range_key(RangeKind::Block, 200..400),
            ]
        );
        assert_eq!(page.total, 4);
    }

    #[test]
    fn rows_first_missing_after_now_count_as_missing_and_never_as_confirmed() {
        let mut set = MissingSet::new(Epochs::Off);
        repeat(&mut set, &leaf_gap(0..10), 3, T0 + 500);
        assert_eq!(count(&set, DataKind::Leaf), 10);
        // Counted at a `counted_until` before the row went missing: not confirmed missing.
        assert_eq!(confirmed_count(&set, 1_000, T0 + 100), []);
        assert_eq!(set.listed(10).total, 1);
    }

    #[test]
    fn retry_delay_stops_at_a_minute_only_for_confirmed_rows() {
        let cases = [
            (Observation::NotFound, vec![10, 20, 40, 60, 60]),
            (
                Observation::Failed(server_error()),
                vec![10, 20, 40, 80, 160, 320, 600, 600],
            ),
        ];
        for (observation, delays) in cases {
            let mut set = MissingSet::new(Epochs::Off);
            let result = point_result(PointKind::LcLeafProof, 99, observation);
            observe(&mut set, result.clone(), T0);
            let mut now = T0 + 5;
            for delay in delays {
                observe(&mut set, result.clone(), now);
                let next_at = set.rows().next().unwrap().state.retry().next_at;
                assert_eq!(next_at, at(now + delay), "{observation:?} {delay}");
                now += delay;
            }
        }
    }

    #[test]
    fn rows_of_single_heights_stay_apart() {
        let mut set = MissingSet::new(Epochs::Off);
        let kind = PointKind::LcLeafProof;
        observe(
            &mut set,
            point_result(kind, 1106, Observation::NotFound),
            T0,
        );
        observe(
            &mut set,
            point_result(kind, 1199, Observation::Present),
            T0 + 1,
        );
        assert_eq!(count(&set, kind.into()), 1, "1199 present keeps 1106");
        observe(
            &mut set,
            point_result(kind, 1199, Observation::NotFound),
            T0 + 3,
        );
        assert_eq!(count(&set, kind.into()), 2);
        observe(
            &mut set,
            point_result(kind, 1106, Observation::Present),
            T0 + 5,
        );
        assert_eq!(count(&set, kind.into()), 1);
    }

    #[test]
    fn point_kinds_count_one_per_row() {
        let mut set = MissingSet::new(Epochs::Off);
        observe(
            &mut set,
            point_result(PointKind::LcLeafProof, 99, Observation::NotFound),
            T0,
        );
        assert_eq!(count(&set, DataKind::LcLeafProof), 1);
    }

    #[test]
    fn an_exact_gap_found_in_a_failed_row_starts_fresh() {
        let mut set = MissingSet::new(Epochs::Off);
        let failed = range_result(RangeKind::Leaf, 0..100, Observation::Failed(server_error()));
        repeat(&mut set, &failed, 3, T0);

        observe(
            &mut set,
            gap_result(RangeKind::Leaf, 0..100, 10..20),
            T0 + 100,
        );
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, range_key(RangeKind::Leaf, 10..20));
        assert_eq!(rows[0].state.probe_count().get(), 1);
        assert_eq!(rows[0].state.first_missing_at(), at(T0 + 100));
        assert_eq!(confirmed_count(&set, 1_000, T0 + 200), []);

        repeat(&mut set, &leaf_gap(10..20), 2, T0 + 101);
        assert_eq!(
            confirmed_count(&set, 1_000, T0 + 200),
            [(DataKind::Leaf, 10)]
        );
    }

    #[test]
    fn epoch_rows_before_the_root_lag_depend_on_block_zero() {
        let mut set = MissingSet::new(epochs(100, 1));
        repeat(
            &mut set,
            &epoch_result(EpochKind::StakeTable, 1, Observation::NotFound),
            3,
            T0,
        );
        assert_eq!(
            confirmed_count(&set, 0, T0 + 200),
            [(DataKind::StakeTable, 1)],
            "epoch 1 of a stake table rests on block 0"
        );
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(1)), at(T0 + 60));
        assert_eq!(diff.rows_removed, 1);
    }

    #[test]
    fn found_trims_rows_to_the_heights_still_missing() {
        let mut set = MissingSet::new(Epochs::Off);
        observe(&mut set, leaf_gap(100..200), T0);
        observe(&mut set, leaf_gap(100..150), T0 + 1);
        assert_eq!(count(&set, DataKind::Leaf), 100);

        let present = |range| range_result(RangeKind::Leaf, range, Observation::Present);
        observe(&mut set, present(100..150), T0 + 2);
        assert_eq!(count(&set, DataKind::Leaf), 50);
        assert_eq!(keys(&set), [range_key(RangeKind::Leaf, 150..200)]);
        observe(&mut set, present(100..200), T0 + 2);
        assert_eq!(count(&set, DataKind::Leaf), 0);
    }

    #[test]
    fn found_keeps_the_part_of_a_row_that_crosses_its_edge() {
        let mut set = MissingSet::new(Epochs::Off);
        observe(&mut set, leaf_gap(100..200), T0);
        let present = range_result(RangeKind::Leaf, 150..250, Observation::Present);
        observe(&mut set, present, T0 + 1);
        assert_eq!(keys(&set), [range_key(RangeKind::Leaf, 100..150)]);
    }

    #[test]
    fn found_clears_exact_keys_of_point_kinds() {
        let mut set = MissingSet::new(Epochs::Off);
        let kind = PointKind::LcLeafProof;
        for height in [5, 6] {
            observe(
                &mut set,
                point_result(kind, height, Observation::NotFound),
                T0,
            );
        }
        observe(
            &mut set,
            point_result(kind, 5, Observation::Present),
            T0 + 1,
        );
        assert_eq!(
            keys(&set),
            [RowKey::Point {
                kind,
                height: Height(6)
            }]
        );
    }

    #[test]
    fn forget_below_window_start() {
        let mut set = MissingSet::new(epochs(25, 1));
        for range in [0..60, 80..140, 150..200] {
            observe(&mut set, leaf_gap(range), T0);
        }
        for height in [99, 150] {
            observe(
                &mut set,
                point_result(PointKind::LcLeafProof, height, Observation::NotFound),
                T0,
            );
        }
        for epoch in [3, 4] {
            observe(
                &mut set,
                epoch_result(EpochKind::StateCert, epoch, Observation::NotFound),
                T0,
            );
        }
        for epoch in [5, 6] {
            observe(
                &mut set,
                epoch_result(EpochKind::StakeTable, epoch, Observation::NotFound),
                T0,
            );
        }

        // Window starts at 100. Leaf 0..60 and lc_leaf_proof 99 end below it and go; leaf 80..140
        // is trimmed to 100..140. The state cert of epoch 3 depends on block 75 and goes, epoch 4
        // on block 100 and stays. The stake table of epoch 5 depends on block 75 and goes, epoch
        // 6 on block 100 and stays.
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(100)), at(T0));
        assert_eq!(diff.rows_removed, 4);
        assert_eq!(count(&set, DataKind::Leaf), 90);
        assert_eq!(
            tally(&set, DataKind::Leaf).oldest.unwrap().key,
            range_key(RangeKind::Leaf, 100..140)
        );
        assert_eq!(count(&set, DataKind::LcLeafProof), 1);
        assert_eq!(count(&set, DataKind::StateCert), 1);
        assert_eq!(count(&set, DataKind::StakeTable), 1);
    }

    #[test]
    fn forget_below_zero_changes_nothing() {
        let mut set = MissingSet::new(Epochs::Off);
        observe(&mut set, leaf_gap(0..10), T0);
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(0)), at(T0));
        assert_eq!(
            diff,
            ImageDiff {
                regions: vec![],
                rows_removed: 0
            }
        );
        assert_eq!(count(&set, DataKind::Leaf), 10);
    }

    #[test]
    fn a_gap_continues_each_unit_and_new_units_start_fresh() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        observe(&mut set, leaf_gap(100..200), T0);
        observe(&mut set, leaf_gap(100..200), T0 + 5);
        // A gap over the old row and 50 new heights.
        observe(&mut set, leaf_gap(150..250), T0 + 30);
        let rows: Vec<_> = set.rows().collect();
        let shape: Vec<_> = rows
            .iter()
            .map(|row| {
                (
                    row.key.units(),
                    row.state.first_missing_at(),
                    row.state.probe_count().get(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                (100..150, at(T0), 2),
                (150..200, at(T0), 3),
                (200..250, at(T0 + 30), 1)
            ]
        );
        assert_eq!(rows[0].key, range_key(leaf, 100..150));
    }

    #[test]
    fn a_failed_chunk_probe_backs_off_failed_and_delays_not_found() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        repeat(&mut set, &leaf_gap(100..200), 3, T0);
        let failed = Observation::Failed(server_error());
        observe(
            &mut set,
            range_result(RangeKind::Leaf, 100..300, failed),
            T0 + 10,
        );
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(
            rows.iter().map(|r| r.key.clone()).collect::<Vec<_>>(),
            [range_key(leaf, 100..200), range_key(leaf, 200..300)]
        );
        // Not found: count and first time kept, delay doubled under the 60 s cap.
        let UnitState::NotFound {
            first_missing_at,
            not_found_probes,
            retry,
        } = rows[0].state
        else {
            panic!("{:?}", rows[0].state);
        };
        assert_eq!((first_missing_at, not_found_probes.get()), (at(T0), 3));
        assert_eq!(retry.next_at, at(T0 + 10 + 40));
        // Heights no row covered start as failed rows.
        assert!(matches!(rows[1].state, UnitState::Failed { .. }));

        // A second failure backs the failed row off and keeps its first time.
        observe(&mut set, range_result(leaf, 200..300, failed), T0 + 20);
        let second = set.rows().nth(1).unwrap().state;
        assert_eq!(second.probe_count().get(), 2);
        assert_eq!(second.first_missing_at(), at(T0 + 10));
    }

    #[test]
    fn refined_keeps_the_parts_outside_the_range() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        observe(&mut set, leaf_gap(50..150), T0);
        observe(&mut set, leaf_gap(150..250), T0 + 10);
        observe(&mut set, gap_result(leaf, 100..200, 120..130), T0 + 30);
        let shape: Vec<_> = set
            .rows()
            .map(|row| {
                (
                    row.key,
                    row.state.first_missing_at(),
                    row.state.probe_count().get(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                (range_key(leaf, 50..100), at(T0), 1),
                (range_key(leaf, 120..130), at(T0), 2),
                (range_key(leaf, 200..250), at(T0 + 10), 1),
            ]
        );
        assert_eq!(count(&set, DataKind::Leaf), 110, "no height counted twice");
    }

    #[test]
    fn refinement_continues_not_found_units_and_a_failed_tail_never_confirms() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        for secs in [0, 5] {
            observe(&mut set, leaf_gap(0..100), T0 + secs);
        }
        let mut findings = RangeFindings::new(h(0..100));
        findings.mark(h(10..20), Observation::NotFound);
        findings.mark(h(50..100), Observation::Failed(server_error()));
        observe(
            &mut set,
            ProbeResult::Range {
                kind: leaf,
                findings,
            },
            T0 + 30,
        );
        let shape: Vec<_> = set
            .rows()
            .map(|row| {
                (
                    row.key,
                    row.state.probe_count().get(),
                    row.state.is_confirmed(),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [
                (range_key(leaf, 10..20), 3, true),
                (range_key(leaf, 50..100), 2, false),
            ]
        );
        assert_eq!(
            confirmed_count(&set, 1_000, T0 + 200),
            [(DataKind::Leaf, 10)]
        );
    }

    #[test]
    fn a_failure_inside_a_404_row_splits_it_and_keeps_its_count() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        repeat(&mut set, &leaf_gap(100..300), 3, T0);
        let failed = Observation::Failed(server_error());
        observe(&mut set, range_result(leaf, 150..200, failed), T0 + 10);
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(
            rows.iter().map(|r| r.key.clone()).collect::<Vec<_>>(),
            [
                range_key(leaf, 100..150),
                range_key(leaf, 150..200),
                range_key(leaf, 200..300)
            ]
        );
        assert_eq!(rows[1].state.probe_count().get(), 3);
        assert_eq!(rows[1].state.retry().next_at, at(T0 + 50));
        assert_eq!(rows[0].state, rows[2].state);
    }

    #[test]
    fn a_failure_in_the_middle_of_404s_on_a_point_row_keeps_it_confirmed() {
        let mut set = MissingSet::new(Epochs::Off);
        let kind = PointKind::LcLeafProof;
        repeat(
            &mut set,
            &point_result(kind, 99, Observation::NotFound),
            3,
            T0,
        );
        let failure = Observation::Failed(server_error());
        observe(&mut set, point_result(kind, 99, failure), T0 + 3);
        let row = set.rows().next().unwrap();
        assert!(row.state.is_confirmed());
        assert_eq!(row.state.probe_count().get(), 3);
        assert_eq!(row.state.retry().next_at, at(T0 + 3 + 40));
        observe(
            &mut set,
            point_result(kind, 99, Observation::NotFound),
            T0 + 4,
        );
        let row = set.rows().next().unwrap();
        assert_eq!(row.state.probe_count().get(), 4);
        assert_eq!(row.state.retry().delay, RetryDelay::from_secs(60).unwrap());
    }

    #[test]
    fn a_known_gap_that_ends_in_a_failed_tail_keeps_its_404_row() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        repeat(&mut set, &leaf_gap(100..200), 3, T0);
        let failed = Observation::Failed(server_error());
        observe(&mut set, range_result(leaf, 100..200, failed), T0 + 3);
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].state.is_confirmed());
        observe(&mut set, leaf_gap(100..200), T0 + 4);
        let row = set.rows().next().unwrap();
        assert_eq!(row.state.probe_count().get(), 4);
        assert_eq!(row.state.first_missing_at(), at(T0));
    }

    #[test]
    fn a_failed_tail_over_a_404_row_and_open_heights_splits_them() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        observe(&mut set, leaf_gap(120..140), T0);
        let mut findings = RangeFindings::new(h(100..200));
        findings.mark(h(100..200), Observation::Failed(server_error()));
        observe(
            &mut set,
            ProbeResult::Range {
                kind: leaf,
                findings,
            },
            T0 + 1,
        );
        let shape: Vec<_> = set
            .rows()
            .map(|row| {
                (
                    row.key.units(),
                    matches!(row.state, UnitState::NotFound { .. }),
                )
            })
            .collect();
        assert_eq!(
            shape,
            [(100..120, false), (120..140, true), (140..200, false)]
        );
    }

    #[test]
    fn touching_rows_with_equal_state_merge_past_a_chunk() {
        let mut set = MissingSet::new(Epochs::Off);
        for range in [0..100, 100..200, 200..300] {
            observe(&mut set, leaf_gap(range), T0);
        }
        assert_eq!(keys(&set), [range_key(RangeKind::Leaf, 0..300)]);
    }

    #[test]
    fn a_failed_retry_of_a_piece_splits_the_row() {
        let mut set = MissingSet::new(Epochs::Off);
        observe(&mut set, leaf_gap(0..300), T0);
        let failed = Observation::Failed(server_error());
        observe(
            &mut set,
            range_result(RangeKind::Leaf, 0..100, failed),
            T0 + 5,
        );
        assert_eq!(set.rows().count(), 2);
    }

    #[test]
    fn confirmed_units_apply_every_condition() {
        let mut set = MissingSet::new(epochs(100, 1));
        let nf = Observation::NotFound;
        repeat(&mut set, &leaf_gap(100..200), 3, T0);
        repeat(&mut set, &leaf_gap(50..100), 3, T0 + 10);
        repeat(
            &mut set,
            &range_result(
                RangeKind::Block,
                100..200,
                Observation::Failed(server_error()),
            ),
            3,
            T0,
        );
        repeat(
            &mut set,
            &range_result(RangeKind::VidCommon, 100..200, nf),
            2,
            T0,
        );
        repeat(
            &mut set,
            &point_result(PointKind::LcLeafProof, 199, nf),
            3,
            T0 + 50,
        );
        repeat(&mut set, &epoch_result(EpochKind::StateCert, 3, nf), 3, T0);
        for epoch in [8, 7] {
            repeat(
                &mut set,
                &epoch_result(EpochKind::StakeTable, epoch, nf),
                3,
                T0,
            );
        }

        // At T0 + 60 a unit counts when first missing at or before T0. Not counting: block (500),
        // vid_common (2 probes), lc_leaf_proof (too young), the leaf rows past the settled end 500
        // are none; stake table epoch 7 rests on block 500 and counts, epoch 8 on block 600 does
        // not.
        assert_eq!(
            confirmed_count(&set, 500, T0 + 60),
            [
                (DataKind::Leaf, 100),
                (DataKind::StakeTable, 1),
                (DataKind::StateCert, 1)
            ]
        );
        // Leaf 50..100 first missing at T0 + 10 counts from T0 + 70.
        assert_eq!(
            confirmed_count(&set, 500, T0 + 70)[0],
            (DataKind::Leaf, 150)
        );
        // Without epochs epoch rows never count.
        let rows = set.rows().collect::<Vec<_>>();
        let off = MissingSet::from_rows(Epochs::Off, rows).unwrap();
        assert!(
            confirmed_count(&off, 500, T0 + 70)
                .iter()
                .all(|(k, _)| *k == DataKind::Leaf)
        );
        // The young units sit in `young` until counted.
        let units = set
            .tallies(Height(500), at(T0 + 60 - 60))
            .into_iter()
            .find(|t| t.kind == DataKind::Leaf)
            .unwrap()
            .confirmed;
        assert_eq!(units.aged, 100);
        assert_eq!(units.young.len(), 1);
        assert_eq!(units.count(at(T0 + 10)), 150);
    }

    #[test]
    fn epoch_rows_without_epochs_are_kept_and_never_count() {
        let mut set = MissingSet::new(Epochs::Off);
        repeat(
            &mut set,
            &epoch_result(EpochKind::StateCert, 3, Observation::NotFound),
            3,
            T0,
        );
        assert_eq!(confirmed_count(&set, 10_000, T0 + 1_000), []);
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(10_000)), at(T0));
        assert_eq!(diff.rows_removed, 0);
        assert_eq!(count(&set, DataKind::StateCert), 1);
    }

    #[test]
    fn epoch_rows_with_an_overflowing_root_block_are_kept() {
        let mut set = MissingSet::new(epochs(30, 61));
        observe(
            &mut set,
            epoch_result(EpochKind::StateCert, u64::MAX, Observation::NotFound),
            T0,
        );
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(u64::MAX)), at(T0));
        assert_eq!(diff.rows_removed, 0);
        assert_eq!(count(&set, DataKind::StateCert), 1);
    }

    #[test]
    fn an_empty_search_equals_present() {
        let mut set = MissingSet::new(Epochs::Off);
        observe(&mut set, leaf_gap(100..200), T0);
        let all_present = ProbeResult::Range {
            kind: RangeKind::Leaf,
            findings: RangeFindings::new(h(100..200)),
        };
        observe(&mut set, all_present, T0 + 1);
        assert_eq!(set.rows().count(), 0);
    }

    #[test]
    fn known_gap_needs_a_not_found_row_around_the_probe() {
        let mut set = MissingSet::new(Epochs::Off);
        let leaf = RangeKind::Leaf;
        observe(&mut set, leaf_gap(100..200), T0);
        observe(
            &mut set,
            range_result(
                RangeKind::Block,
                300..400,
                Observation::Failed(server_error()),
            ),
            T0,
        );
        assert_eq!(set.known(leaf, &h(100..200)), Known::Gap);
        assert_eq!(set.known(leaf, &h(120..150)), Known::Gap);
        assert_eq!(set.known(leaf, &h(100..201)), Known::Missing);
        assert_eq!(set.known(leaf, &h(50..150)), Known::Missing);
        assert_eq!(set.known(RangeKind::Block, &h(300..400)), Known::Missing);
    }

    #[test]
    fn from_rows_rejects_overlapping_and_touching_equal_rows() {
        let state = |secs| UnitState::NotFound {
            first_missing_at: at(secs),
            not_found_probes: std::num::NonZeroU32::MIN,
            retry: Retry::after(RetryDelay::FIRST, at(secs)),
        };
        let row = |range, secs| MissingRow {
            key: range_key(RangeKind::Leaf, range),
            state: state(secs),
        };
        let load = |rows: Vec<MissingRow>| MissingSet::from_rows(Epochs::Off, rows);
        assert!(load(vec![row(0..10, T0), row(5..15, T0 + 1)]).is_err());
        assert!(load(vec![row(0..10, T0), row(10..20, T0)]).is_err());
        assert!(load(vec![row(0..10, T0), row(10..20, T0 + 1)]).is_ok());
    }

    #[test]
    fn rows_come_out_sorted_by_kind() {
        let mut set = MissingSet::new(epochs(30, 61));
        let nf = Observation::NotFound;
        observe(&mut set, epoch_result(EpochKind::LcStakeTable, 7, nf), T0);
        observe(&mut set, point_result(PointKind::LcLeafProof, 5, nf), T0);
        observe(&mut set, range_result(RangeKind::VidCommon, 0..10, nf), T0);
        observe(&mut set, leaf_gap(0..10), T0);
        let kinds: Vec<_> = set.rows().map(|row| row.key.kind()).collect();
        assert!(kinds.windows(2).all(|pair| pair[0] < pair[1]), "{kinds:?}");
    }

    #[test]
    fn split_first_chunk_cuts_ranges_only() {
        let (probe, rest) = range_key(RangeKind::Leaf, 10..250).split_first_chunk();
        assert_eq!(
            probe,
            Probe::Range {
                kind: RangeKind::Leaf,
                heights: h(10..110)
            }
        );
        assert_eq!(rest, Some(range_key(RangeKind::Leaf, 110..250)));
        let (_, rest) = range_key(RangeKind::Leaf, 10..110).split_first_chunk();
        assert_eq!(rest, None);
        let point = RowKey::Point {
            kind: PointKind::LcLeafProof,
            height: Height(4),
        };
        assert_eq!(point.clone().split_first_chunk().1, None);
    }

    /// Drains a queue into probes.
    fn drain(mut queue: ChunkQueue) -> Vec<Probe> {
        std::iter::from_fn(|| queue.next_probe()).collect()
    }

    fn leaf_probe(range: Range<u64>) -> Probe {
        Probe::Range {
            kind: RangeKind::Leaf,
            heights: h(range),
        }
    }

    #[test]
    fn retry_queue_order() {
        let mut set = MissingSet::new(Epochs::Off);
        let nf = Observation::NotFound;
        // Confirming rows with different next_at: 5 s after T0 + 3, 5 s after T0.
        observe(&mut set, leaf_gap(1_000..1_250), T0 + 3);
        observe(&mut set, range_result(RangeKind::Block, 5..10, nf), T0);
        observe(&mut set, point_result(PointKind::LcLeafProof, 7, nf), T0);
        // Two confirmed rows, the older one first missing at T0 - 100.
        repeat(&mut set, &leaf_gap(300..310), 3, T0 - 100);
        repeat(&mut set, &leaf_gap(400..410), 3, T0 - 50);

        let queue = RetryQueue::new(&set, at(T0 + 8));
        assert_eq!(
            drain(queue.confirming),
            [
                Probe::Range {
                    kind: RangeKind::Block,
                    heights: h(5..10)
                },
                Probe::Point {
                    kind: PointKind::LcLeafProof,
                    height: Height(7)
                },
                leaf_probe(1_000..1_100),
                leaf_probe(1_100..1_200),
                leaf_probe(1_200..1_250),
            ]
        );
        // The older confirmed row has next_at T0 - 98 + 20: due. The younger would be due too.
        let queue = RetryQueue::new(&set, at(T0));
        assert_eq!(queue.representative, Some(leaf_probe(300..310)));
        // No confirming row is due at T0: the earliest next_at is T0 + 5.
        assert!(drain(queue.confirming).is_empty());
        // Not due although the younger confirmed row is.
        let mut late = set.clone();
        observe(&mut late, leaf_gap(300..310), T0 + 1_000);
        let queue = RetryQueue::new(&late, at(T0 + 1_000));
        assert_eq!(queue.representative, None);
        assert_eq!(
            drain(ChunkQueue::confirmed(&set)),
            [leaf_probe(300..310), leaf_probe(400..410)]
        );
    }

    // Property test against a per-unit reference model.

    const EPOCHS: Epochs = Epochs::On(EpochConfig {
        epoch_height: NonZeroU64::new(30).unwrap(),
        epoch_start_block: Height(61),
    });
    const SETTLED_ENDS: [u64; 3] = [150, 300, 500];

    type Unit = (DataKind, u64);

    /// A range shape relative to the probed range.
    #[derive(Clone, Debug)]
    enum Shape {
        Uniform(Observation),
        /// Gaps are the maximal true runs of the mask below the optional failed tail start.
        Mask {
            mask: Vec<bool>,
            tail: Option<(u64, RequestFailure)>,
        },
    }

    /// What a retry finds for the probe it takes.
    #[derive(Clone, Copy, Debug)]
    enum Answer {
        Present,
        NotFound,
        Failed,
        Transport,
        /// First half not found, the rest failed.
        GapThenFailed,
        /// First half present, the rest not found.
        PresentThenGap,
    }

    #[derive(Clone, Debug)]
    enum Op {
        Range {
            kind: RangeKind,
            start: u64,
            len: u64,
            shape: Shape,
        },
        Point {
            kind: PointKind,
            height: u64,
            observation: Observation,
        },
        Epoch {
            kind: EpochKind,
            epoch: u64,
            observation: Observation,
        },
        Forget(u64),
        Advance(u64),
        Retry(Answer),
        Representative(Answer),
    }

    fn failure() -> impl Strategy<Value = RequestFailure> {
        prop_oneof![
            Just(RequestFailure::Transport),
            Just(server_error()),
            Just(RequestFailure::Status(
                FailedStatus::new(StatusCode::SERVICE_UNAVAILABLE).unwrap()
            )),
        ]
    }

    fn observation() -> impl Strategy<Value = Observation> {
        prop_oneof![
            Just(Observation::Present),
            Just(Observation::NotFound),
            failure().prop_map(Observation::Failed),
        ]
    }

    fn answer() -> impl Strategy<Value = Answer> {
        prop_oneof![
            2 => Just(Answer::Present),
            4 => Just(Answer::NotFound),
            2 => Just(Answer::Failed),
            1 => Just(Answer::Transport),
            1 => Just(Answer::GapThenFailed),
            1 => Just(Answer::PresentThenGap),
        ]
    }

    fn shape(len: u64) -> impl Strategy<Value = Shape> {
        prop_oneof![
            observation().prop_map(Shape::Uniform),
            (
                vec(any::<bool>(), len as usize),
                option::of((0..len, failure()))
            )
                .prop_map(|(mask, tail)| Shape::Mask { mask, tail }),
        ]
    }

    /// Ops over heights `0..heights` and epochs `0..epochs`; a small domain repeats units often.
    fn op(heights: u64, epochs: u64) -> impl Strategy<Value = Op> {
        let range = (
            prop::sample::select(RangeKind::VARIANTS),
            0..heights,
            1..=CHUNK_LEN.min(heights),
        )
            .prop_flat_map(|(kind, start, len)| (Just(kind), Just(start), Just(len), shape(len)))
            .prop_map(|(kind, start, len, shape)| Op::Range {
                kind,
                start,
                len,
                shape,
            });
        let point = (
            prop::sample::select(PointKind::VARIANTS),
            0..heights,
            observation(),
        )
            .prop_map(|(kind, height, observation)| Op::Point {
                kind,
                height,
                observation,
            });
        let epoch = (
            prop::sample::select(EpochKind::VARIANTS),
            0..epochs,
            observation(),
        )
            .prop_map(|(kind, epoch, observation)| Op::Epoch {
                kind,
                epoch,
                observation,
            });
        prop_oneof![
            4 => range,
            2 => point,
            2 => epoch,
            1 => (0..heights).prop_map(Op::Forget),
            4 => prop_oneof![0u64..12, 0u64..700].prop_map(Op::Advance),
            6 => answer().prop_map(Op::Retry),
            3 => answer().prop_map(Op::Representative),
        ]
    }

    /// Findings of `shape` over `heights`.
    fn findings_of(heights: Range<Height>, shape: &Shape) -> RangeFindings {
        let (mask, tail) = match shape {
            Shape::Uniform(observation) => return RangeFindings::uniform(heights, *observation),
            Shape::Mask { mask, tail } => (mask, tail),
        };
        let start = heights.start.0;
        let end = tail.map_or(mask.len(), |(offset, _)| offset as usize);
        let mut findings = RangeFindings::new(heights.clone());
        let mut run: Option<usize> = None;
        for (i, &on) in mask[..end].iter().chain([&false]).enumerate() {
            match (on, run) {
                (true, None) => run = Some(i),
                (false, Some(from)) => {
                    let gap = Height(start + from as u64)..Height(start + i as u64);
                    findings.mark(gap, Observation::NotFound);
                    run = None;
                },
                _ => {},
            }
        }
        if let Some((offset, failure)) = tail {
            findings.mark(
                Height(start + offset)..heights.end,
                Observation::Failed(*failure),
            );
        }
        findings
    }

    /// The result a retry of `probe` finds.
    fn found_result(probe: &Probe, found: Answer) -> ProbeResult {
        let unit = |observation| match probe {
            Probe::Point { kind, height } => ProbeResult::Point {
                kind: *kind,
                height: *height,
                observation,
            },
            Probe::Epoch { kind, epoch } => ProbeResult::Epoch {
                kind: *kind,
                epoch: *epoch,
                observation,
            },
            Probe::Range { .. } => unreachable!("units only"),
        };
        let uniform = |observation| match probe {
            Probe::Range { kind, heights } => ProbeResult::Range {
                kind: *kind,
                findings: RangeFindings::uniform(heights.clone(), observation),
            },
            _ => unit(observation),
        };
        let split = |first, second| match probe {
            Probe::Range { kind, heights } if height_count(heights) > 1 => {
                let mid = Height(heights.start.0 + height_count(heights) / 2);
                let mut findings = RangeFindings::new(heights.clone());
                if first != Observation::Present {
                    findings.mark(heights.start..mid, first);
                }
                findings.mark(mid..heights.end, second);
                ProbeResult::Range {
                    kind: *kind,
                    findings,
                }
            },
            _ => uniform(second),
        };
        match found {
            Answer::Present => uniform(Observation::Present),
            Answer::NotFound => uniform(Observation::NotFound),
            Answer::Failed => uniform(Observation::Failed(server_error())),
            Answer::Transport => uniform(Observation::Failed(RequestFailure::Transport)),
            Answer::GapThenFailed => {
                split(Observation::NotFound, Observation::Failed(server_error()))
            },
            Answer::PresentThenGap => split(Observation::Present, Observation::NotFound),
        }
    }

    /// Units a probe covers.
    fn probe_units(probe: &Probe) -> (DataKind, Range<u64>) {
        match probe {
            Probe::Range { kind, heights } => ((*kind).into(), heights.start.0..heights.end.0),
            Probe::Point { kind, height } => ((*kind).into(), height.0..height.0 + 1),
            Probe::Epoch { kind, epoch } => ((*kind).into(), epoch.0..epoch.0 + 1),
        }
    }

    /// One state per unit; the only rule applied is `state::transition`, per unit.
    #[derive(Default)]
    struct Model {
        units: BTreeMap<Unit, UnitState>,
    }

    /// A maximal run of consecutive units of one range kind with one state; one run per other
    /// unit.
    struct Run {
        kind: DataKind,
        units: Range<u64>,
        state: UnitState,
    }

    impl Model {
        fn observe_unit(&mut self, unit: Unit, observation: Observation, now: DateTime<Utc>) {
            match crate::state::transition(self.units.get(&unit).copied(), observation, now) {
                Some(state) => self.units.insert(unit, state),
                None => self.units.remove(&unit),
            };
        }

        fn apply(&mut self, result: &ProbeResult, now: DateTime<Utc>) {
            match result {
                ProbeResult::Range { kind, findings } => {
                    for (piece, observation) in findings.pieces() {
                        for height in piece.start.0..piece.end.0 {
                            self.observe_unit(((*kind).into(), height), observation, now);
                        }
                    }
                },
                ProbeResult::Point {
                    kind,
                    height,
                    observation,
                } => self.observe_unit(((*kind).into(), height.0), *observation, now),
                ProbeResult::Epoch {
                    kind,
                    epoch,
                    observation,
                } => self.observe_unit(((*kind).into(), epoch.0), *observation, now),
            }
        }

        fn forget_below(&mut self, window_start: u64) {
            self.units.retain(|&(kind, unit), _| match kind.class() {
                KindClass::Range(_) | KindClass::Point(_) => unit >= window_start,
                KindClass::Epoch(kind) => EPOCHS
                    .root_block(kind, Epoch(unit))
                    .is_none_or(|block| block.0 >= window_start),
            });
        }

        fn runs(&self) -> Vec<Run> {
            let mut runs: Vec<Run> = Vec::new();
            for (&(kind, unit), &state) in &self.units {
                let extends = matches!(kind.class(), KindClass::Range(_))
                    && runs.last().is_some_and(|run| {
                        run.kind == kind && run.units.end == unit && run.state == state
                    });
                if extends {
                    runs.last_mut().unwrap().units.end += 1;
                } else {
                    runs.push(Run {
                        kind,
                        units: unit..unit + 1,
                        state,
                    });
                }
            }
            runs
        }

        /// The height a run is judged against the tip by.
        fn alert_height(run: &Run) -> Option<u64> {
            match run.kind.class() {
                KindClass::Range(_) => Some(run.units.end),
                KindClass::Point(_) => Some(run.units.end),
                KindClass::Epoch(kind) => {
                    EPOCHS.root_block(kind, Epoch(run.units.start)).map(|h| h.0)
                },
            }
        }
    }

    fn seconds(secs: u64) -> TimeDelta {
        TimeDelta::seconds(secs as i64)
    }

    /// The units of every row of the set.
    fn expand(set: &MissingSet) -> BTreeMap<Unit, UnitState> {
        set.rows()
            .flat_map(|row| {
                let kind = row.key.kind();
                row.key.units().map(move |unit| ((kind, unit), row.state))
            })
            .collect()
    }

    fn check_units(set: &MissingSet, model: &Model) -> Result<(), TestCaseError> {
        prop_assert_eq!(&expand(set), &model.units);
        Ok(())
    }

    /// Range rows of a kind ascend, are disjoint, and touching rows differ in state.
    fn check_row_order(set: &MissingSet) -> Result<(), TestCaseError> {
        let rows: Vec<_> = set.rows().collect();
        for pair in rows.windows(2) {
            let (a, b) = (&pair[0], &pair[1]);
            prop_assert!(a.key.kind() <= b.key.kind());
            if a.key.kind() == b.key.kind() {
                prop_assert!(a.key.units().end <= b.key.units().start, "{a:?} {b:?}");
                let touching = a.key.units().end == b.key.units().start;
                if touching && matches!(a.key, RowKey::Range { .. }) {
                    prop_assert!(a.state != b.state, "touching equal rows {a:?} {b:?}");
                }
            }
        }
        Ok(())
    }

    fn check_tallies(
        set: &MissingSet,
        model: &Model,
        now: DateTime<Utc>,
    ) -> Result<(), TestCaseError> {
        let runs = model.runs();
        for settled_end in SETTLED_ENDS {
            for counted in [now - seconds(100), now, now + seconds(300)] {
                let tallies = set.tallies(Height(settled_end), counted);
                prop_assert_eq!(tallies.len(), DataKind::VARIANTS.len());
                for tally in tallies {
                    let of_kind = || runs.iter().filter(|run| run.kind == tally.kind);
                    let missing: u64 = of_kind().map(|run| run.units.end - run.units.start).sum();
                    prop_assert_eq!(tally.missing_count, missing);
                    let oldest =
                        of_kind().min_by_key(|run| (run.state.first_missing_at(), run.units.start));
                    prop_assert_eq!(
                        tally
                            .oldest
                            .as_ref()
                            .map(|row| (row.state, row.key.first_unit())),
                        oldest.map(|run| (run.state, run.units.start))
                    );
                    for later in [counted, counted + seconds(50), counted + seconds(900)] {
                        let expected: u64 = of_kind()
                            .filter(|run| {
                                run.state.is_confirmed()
                                    && run.state.first_missing_at() <= later
                                    && Model::alert_height(run).is_some_and(|h| h <= settled_end)
                            })
                            .map(|run| run.units.end - run.units.start)
                            .sum();
                        prop_assert_eq!(tally.confirmed.count(later), expected);
                    }
                }
            }
        }
        Ok(())
    }

    /// The probes of `run`: its units cut at `CHUNK_LEN` from the start.
    fn run_probes(run: &Run) -> Vec<Probe> {
        match run.kind.class() {
            KindClass::Range(kind) => {
                let mut probes = Vec::new();
                let mut start = run.units.start;
                while start < run.units.end {
                    let end = run.units.end.min(start + CHUNK_LEN);
                    probes.push(Probe::Range {
                        kind,
                        heights: h(start..end),
                    });
                    start = end;
                }
                probes
            },
            KindClass::Point(kind) => vec![Probe::Point {
                kind,
                height: Height(run.units.start),
            }],
            KindClass::Epoch(kind) => vec![Probe::Epoch {
                kind,
                epoch: Epoch(run.units.start),
            }],
        }
    }

    fn check_retry_queue(
        set: &MissingSet,
        model: &Model,
        now: DateTime<Utc>,
    ) -> Result<(), TestCaseError> {
        let runs = model.runs();
        let queue = RetryQueue::new(set, now);
        let oldest_confirmed = runs
            .iter()
            .filter(|run| run.state.is_confirmed())
            .min_by_key(|run| (run.state.first_missing_at(), run.units.start, run.kind));
        let representative = oldest_confirmed
            .filter(|run| run.state.retry().next_at <= now)
            .map(|run| run_probes(run).remove(0));
        prop_assert_eq!(&queue.representative, &representative);

        let mut due: Vec<_> = runs
            .iter()
            .filter(|run| !run.state.is_confirmed() && run.state.retry().next_at <= now)
            .collect();
        due.sort_by_key(|run| (run.state.retry().next_at, run.kind, run.units.start));
        let expected: Vec<Probe> = due.into_iter().flat_map(run_probes).collect();
        prop_assert_eq!(drain(queue.confirming), expected);

        let confirmed: Vec<Probe> = runs
            .iter()
            .filter(|run| run.state.is_confirmed())
            .flat_map(run_probes)
            .collect();
        prop_assert_eq!(drain(ChunkQueue::confirmed(set)), confirmed);
        Ok(())
    }

    fn check_known_and_listed(set: &MissingSet, model: &Model) -> Result<(), TestCaseError> {
        let runs = model.runs();
        for run in &runs {
            let KindClass::Range(kind) = run.kind.class() else {
                continue;
            };
            let not_found = matches!(run.state, UnitState::NotFound { .. });
            let expect = |gap| if gap { Known::Gap } else { Known::Missing };
            prop_assert_eq!(set.known(kind, &h(run.units.clone())), expect(not_found));
            prop_assert_eq!(
                set.known(kind, &h(run.units.start..run.units.end + 1)),
                Known::Missing
            );
        }
        let mut order: Vec<_> = runs.iter().collect();
        order.sort_by_key(|run| (run.state.first_missing_at(), run.units.start, run.kind));
        let listed = set.listed(20);
        prop_assert_eq!(listed.total, runs.len() as u64);
        let shown: Vec<_> = listed
            .shown
            .iter()
            .map(|row| (row.key.kind(), row.key.units()))
            .collect();
        let expected: Vec<_> = order
            .iter()
            .take(20)
            .map(|run| (run.kind, run.units.clone()))
            .collect();
        prop_assert_eq!(shown, expected);
        Ok(())
    }

    /// The write-through image: the units of the rows of each rewritten region are replaced.
    fn apply_diff(
        set: &MissingSet,
        diff: &ImageDiff,
        image: &mut BTreeMap<Unit, UnitState>,
    ) -> Result<(), TestCaseError> {
        for region in &diff.regions {
            let (kind, units) = region.units();
            image.retain(|&(k, unit), _| k != kind || !units.contains(&unit));
            for row in set.rows_in(region) {
                prop_assert_eq!(row.key.kind(), kind);
                prop_assert!(
                    row.key.units().start >= units.start && row.key.units().end <= units.end,
                    "{row:?} outside {region:?}"
                );
                for unit in row.key.units() {
                    image.insert((kind, unit), row.state);
                }
            }
        }
        prop_assert_eq!(&*image, &expand(set));
        Ok(())
    }

    /// After a probe from the retry queue, its units are gone or wait for a later retry.
    fn check_liveness(
        set: &MissingSet,
        probe: &Probe,
        now: DateTime<Utc>,
    ) -> Result<(), TestCaseError> {
        let (kind, units) = probe_units(probe);
        let state = expand(set);
        for unit in units {
            if let Some(state) = state.get(&(kind, unit)) {
                prop_assert!(state.retry().next_at > now, "{kind:?} {unit} is still due");
            }
        }
        Ok(())
    }

    /// What an op does at `now`.
    enum Step {
        Advance(u64),
        Skip,
        Update(MissingUpdate, Option<Probe>),
    }

    fn step_of(op: &Op, set: &MissingSet, now: DateTime<Utc>) -> Step {
        let observed = |result| Step::Update(MissingUpdate::Observed(result), None);
        match op {
            Op::Advance(secs) => Step::Advance(*secs),
            Op::Forget(height) => Step::Update(MissingUpdate::ForgetBelow(Height(*height)), None),
            Op::Range {
                kind,
                start,
                len,
                shape,
            } => observed(ProbeResult::Range {
                kind: *kind,
                findings: findings_of(h(*start..*start + *len), shape),
            }),
            Op::Point {
                kind,
                height,
                observation,
            } => observed(point_result(*kind, *height, *observation)),
            Op::Epoch {
                kind,
                epoch,
                observation,
            } => observed(epoch_result(*kind, *epoch, *observation)),
            Op::Retry(found) => {
                let probe = RetryQueue::new(set, now).confirming.next_probe();
                probe.map_or(Step::Skip, |probe| {
                    let result = found_result(&probe, *found);
                    Step::Update(MissingUpdate::Observed(result), Some(probe))
                })
            },
            Op::Representative(found) => {
                let probe = RetryQueue::new(set, now).representative;
                probe.map_or(Step::Skip, |probe| {
                    let result = found_result(&probe, *found);
                    Step::Update(MissingUpdate::Observed(result), Some(probe))
                })
            },
        }
    }

    /// Runs `ops` against the set and the model, checking every invariant after each step.
    fn run_ops(ops: Vec<Op>) -> Result<(), TestCaseError> {
        let mut set = MissingSet::new(EPOCHS);
        let mut model = Model::default();
        let mut image = BTreeMap::new();
        let mut now = at(T0);
        for op in ops {
            let (update, probe) = match step_of(&op, &set, now) {
                Step::Advance(secs) => {
                    now += seconds(secs);
                    continue;
                },
                Step::Skip => continue,
                Step::Update(update, probe) => (update, probe),
            };
            let runs_before = model.runs().len();
            match &update {
                MissingUpdate::Observed(result) => model.apply(result, now),
                MissingUpdate::ForgetBelow(window_start) => model.forget_below(window_start.0),
            }
            let diff = set.apply(update, now);
            let forgotten = matches!(op, Op::Forget(_));
            if forgotten {
                prop_assert_eq!(
                    diff.rows_removed,
                    runs_before.saturating_sub(model.runs().len())
                );
            }
            apply_diff(&set, &diff, &mut image)?;
            check_units(&set, &model)?;
            check_row_order(&set)?;
            check_tallies(&set, &model, now)?;
            check_retry_queue(&set, &model, now)?;
            check_known_and_listed(&set, &model)?;
            if let Some(probe) = probe {
                check_liveness(&set, &probe, now)?;
            }
        }
        Ok(())
    }

    /// Runs `ops` through `PersistedMissingSet` on SQLite in memory and reloads the image after
    /// every step.
    fn run_ops_persisted(ops: Vec<Op>) -> Result<(), TestCaseError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a runtime builds");
        runtime.block_on(async {
            let options = SqliteConnectOptions::from_str("sqlite::memory:").unwrap();
            let store = Store::connect(options).await.unwrap().node("a:1");
            let mut persisted = PersistedMissingSet::load(store.clone(), EPOCHS)
                .await
                .unwrap();
            let mut now = at(T0);
            for op in ops {
                match step_of(&op, persisted.set(), now) {
                    Step::Advance(secs) => now += seconds(secs),
                    Step::Skip => {},
                    Step::Update(update, _) => {
                        persisted.apply(update, now).await.unwrap();
                    },
                }
                let loaded = PersistedMissingSet::load(store.clone(), EPOCHS)
                    .await
                    .expect("the image loads");
                prop_assert_eq!(
                    loaded.set().rows().collect::<Vec<_>>(),
                    persisted.set().rows().collect::<Vec<_>>()
                );
            }
            Ok(())
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn missing_set_matches_the_per_unit_model(ops in vec(op(450, 12), 1..60)) {
            run_ops(ops)?;
        }

        /// A small domain, so rows are probed again and again and reach their delay caps.
        #[test]
        fn repeated_probes_match_the_per_unit_model(ops in vec(op(110, 3), 1..60)) {
            run_ops(ops)?;
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn the_persisted_image_reloads_after_every_step(ops in vec(op(110, 3), 1..40)) {
            run_ops_persisted(ops)?;
        }
    }
}
