//! The missing data of one node: rows, updates and the queries the loops and the page read.

use std::{collections::BTreeMap, ops::Range, time::Duration};

use anyhow::ensure;
use chrono::{DateTime, TimeDelta, Utc};
use rangemap::RangeMap;
use strum::VariantArray;

use crate::{
    gaps::Known,
    kind::{
        DataKind, Epoch, EpochConfig, EpochKind, Height, PointKind, Probe, RangeKind, height_count,
        next_chunk,
    },
    outcome::{
        Failure, GapSearch, MissingReason, Observation, RangeOutcome, Unconfirmed, UnitOutcome,
    },
};

/// First retry delay of a row.
const RETRY_DELAY_MIN: Duration = Duration::from_secs(5);
/// Cap of the doubling retry delay.
const RETRY_DELAY_MAX: Duration = Duration::from_secs(600);
/// Cap for a confirmed row, so a refilled range is noticed within a minute.
const ALERT_RETRY_DELAY_MAX: Duration = Duration::from_secs(60);
/// Consecutive failed probes before a row is confirmed.
pub const ALERT_MIN_FAILED_PROBES: u32 = 3;

/// `duration` as a chrono delta; delays and ages are at most days.
fn delta(duration: Duration) -> TimeDelta {
    TimeDelta::from_std(duration).expect("durations here fit a chrono delta")
}

/// Time since the row first went missing; rows first missing after `now` are filtered out before.
fn age(now: DateTime<Utc>, state: &RowState) -> Duration {
    (now - state.first_missing_at)
        .to_std()
        .expect("rows first missing after now are left out")
}

/// Whether `row` went missing before `other`: earlier first-missing time, then lower first unit.
fn older(row: &MissingRow, other: &MissingRow) -> bool {
    let key = |row: &MissingRow| (row.state.first_missing_at, row.key.first_unit());
    key(row) < key(other)
}

/// The smallest range holding both.
fn hull_of<T: Ord + Copy>(a: &Range<T>, b: &Range<T>) -> Range<T> {
    a.start.min(b.start)..a.end.max(b.end)
}

/// Rows of `map` overlapping `heights`, whole.
fn overlapping_rows(
    map: &RangeMap<Height, RowState>,
    heights: &Range<Height>,
) -> Vec<(Range<Height>, RowState)> {
    map.overlapping(heights)
        .map(|(range, state)| (range.clone(), state.clone()))
        .collect()
}

/// The state of a point or epoch row after `outcome`; `None` when the unit is present.
fn unit_state(
    kind: DataKind,
    old: Option<&RowState>,
    outcome: UnitOutcome,
    now: DateTime<Utc>,
) -> Option<RowState> {
    let UnitOutcome::Missing(not_served) = outcome else {
        return None;
    };
    Some(RowState::merged(kind, old, not_served.into(), now))
}

/// Applies a point or epoch observation to `rows`; whether the row changed.
fn observe_unit<K: Ord + Copy, U: Ord + Copy>(
    rows: &mut BTreeMap<(K, U), RowState>,
    key: (K, U),
    kind: DataKind,
    outcome: UnitOutcome,
    now: DateTime<Utc>,
) -> bool {
    let old = rows.get(&key).cloned();
    let new = unit_state(kind, old.as_ref(), outcome, now);
    let changed = new != old;
    match new {
        Some(state) => rows.insert(key, state),
        None => rows.remove(&key),
    };
    changed
}

/// Removes the rows of `kind` whose unit satisfies `dropped`; the units they covered, `None` when
/// there were none.
fn drop_units<K: Ord + Copy, U: Ord + Copy>(
    rows: &mut BTreeMap<(K, U), RowState>,
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

/// Pieces a failed range probe writes. A probe inside one unconfirmed row backs that part off;
/// otherwise rows stay, confirmed `NotFound` rows included, and the heights no row covers become
/// fresh rows.
fn failure_pieces(
    kind: RangeKind,
    map: &RangeMap<Height, RowState>,
    before: &[(Range<Height>, RowState)],
    heights: &Range<Height>,
    failure: Failure,
    now: DateTime<Utc>,
) -> Vec<(Range<Height>, RowState)> {
    let reason = MissingReason::Unconfirmed(Unconfirmed::Failed(failure));
    match before {
        [(row, state)]
            if row.start <= heights.start
                && row.end >= heights.end
                && state.reason != MissingReason::NotFound =>
        {
            vec![(
                heights.clone(),
                RowState::merged(kind.into(), [state], reason, now),
            )]
        },
        _ => map
            .gaps(heights)
            .map(|gap| (gap, RowState::merged(kind.into(), [], reason, now)))
            .collect(),
    }
}

/// Pieces a refined search writes: each gap (`NotFound`) and the tail, inheriting from the rows
/// they overlap.
fn refined_pieces(
    kind: RangeKind,
    before: &[(Range<Height>, RowState)],
    search: &GapSearch,
    now: DateTime<Utc>,
) -> Vec<(Range<Height>, RowState)> {
    let gaps = search
        .gaps
        .iter()
        .map(|gap| (gap.clone(), MissingReason::NotFound));
    let tail = search.tail.iter().map(|tail| {
        (
            tail.heights.clone(),
            MissingReason::Unconfirmed(tail.reason),
        )
    });
    gaps.chain(tail)
        .map(|(piece, reason)| {
            let old = before
                .iter()
                .filter(|(row, _)| row.start < piece.end && row.end > piece.start)
                .map(|(_, state)| state);
            let state = RowState::merged(kind.into(), old, reason, now);
            (piece, state)
        })
        .collect()
}

/// Retry and alert state shared by every unit of a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowState {
    pub first_missing_at: DateTime<Utc>,
    pub failed_probes: u32,
    pub retry_delay: Duration,
    pub next_retry_at: DateTime<Utc>,
    pub reason: MissingReason,
}

impl RowState {
    /// State after one more failed probe of `kind` with `reason` at `now`, continuing `old`: for
    /// `NotFound` only the `NotFound` states of `old` count. Oldest `first_missing_at`, largest
    /// `failed_probes + 1`, largest delay doubled under the cap (`ALERT_RETRY_DELAY_MAX` when the
    /// result is confirmed, else `RETRY_DELAY_MAX`); nothing counted: fresh at `now`, 1 probe,
    /// `RETRY_DELAY_MIN`. The only inheritance and backoff rule.
    fn merged<'a>(
        kind: DataKind,
        old: impl IntoIterator<Item = &'a RowState>,
        reason: MissingReason,
        now: DateTime<Utc>,
    ) -> RowState {
        let counted: Vec<_> = old
            .into_iter()
            .filter(|state| {
                reason != MissingReason::NotFound || state.reason == MissingReason::NotFound
            })
            .collect();
        let failed_probes = counted
            .iter()
            .map(|state| state.failed_probes)
            .max()
            .map_or(1, |failed| failed + 1);
        let first_missing_at = counted
            .iter()
            .map(|state| state.first_missing_at)
            .min()
            .unwrap_or(now);
        let retry_delay = match counted.iter().map(|state| state.retry_delay).max() {
            None => RETRY_DELAY_MIN,
            Some(delay) => {
                let cap = if is_confirmed(kind, reason, failed_probes) {
                    ALERT_RETRY_DELAY_MAX
                } else {
                    RETRY_DELAY_MAX
                };
                (delay * 2).min(cap)
            },
        };
        RowState {
            first_missing_at,
            failed_probes,
            retry_delay,
            next_retry_at: now + delta(retry_delay),
            reason,
        }
    }

    /// `kind.can_alert()`, reason `NotFound`, at least `ALERT_MIN_FAILED_PROBES` failed probes.
    /// Selects the 60 s retry cap in `merged` and gates `AlertRule::fires`.
    pub fn is_confirmed(&self, kind: DataKind) -> bool {
        is_confirmed(kind, self.reason, self.failed_probes)
    }
}

/// `RowState::is_confirmed` over its inputs, for `merged` before the state exists.
fn is_confirmed(kind: DataKind, reason: MissingReason, failed_probes: u32) -> bool {
    kind.can_alert()
        && reason == MissingReason::NotFound
        && failed_probes >= ALERT_MIN_FAILED_PROBES
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
    /// point or epoch; the page link and the alert `url`.
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

    /// Probes of the whole row, each at most `CHUNK_LEN` heights from the start; the point or
    /// epoch probe for those kinds.
    fn probes(&self) -> Box<dyn Iterator<Item = Probe> + '_> {
        match self {
            Self::Range { kind, heights } => {
                let (kind, end) = (*kind, heights.end);
                let chunks = std::iter::successors(next_chunk(heights.start, end), move |chunk| {
                    next_chunk(chunk.end, end)
                });
                Box::new(chunks.map(move |heights| Probe::Range { kind, heights }))
            },
            key => Box::new(std::iter::once(key.first_probe())),
        }
    }
}

/// A key and its state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingRow {
    pub key: RowKey,
    pub state: RowState,
}

/// The only kinds of change to a missing set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MissingUpdate {
    /// A probe result.
    Observed(Observation),
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
    epoch_config: Option<EpochConfig>,
    ranges: BTreeMap<RangeKind, RangeMap<Height, RowState>>,
    points: BTreeMap<(PointKind, Height), RowState>,
    epoch_rows: BTreeMap<(EpochKind, Epoch), RowState>,
}

impl MissingSet {
    /// Empty set.
    pub fn new(epoch_config: Option<EpochConfig>) -> Self {
        Self {
            epoch_config,
            ranges: BTreeMap::new(),
            points: BTreeMap::new(),
            epoch_rows: BTreeMap::new(),
        }
    }

    /// From image rows; errors on range rows of a kind that overlap (`overlapping` non-empty
    /// before `insert`), or touch with equal state (row count after loading below the input
    /// count).
    pub fn from_rows(
        epoch_config: Option<EpochConfig>,
        rows: impl IntoIterator<Item = MissingRow>,
    ) -> anyhow::Result<Self> {
        let mut set = Self::new(epoch_config);
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

    /// Applies one update at `now` and returns the image regions it rewrote. The only mutator.
    pub fn apply(&mut self, update: MissingUpdate, now: DateTime<Utc>) -> ImageDiff {
        let rows_before = self.row_count();
        let regions = match update {
            MissingUpdate::Observed(observation) => {
                self.observe(observation, now).into_iter().collect()
            },
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

    /// Applies one observation; the region it rewrote, `None` when nothing changed.
    fn observe(&mut self, observation: Observation, now: DateTime<Utc>) -> Option<ImageRegion> {
        match observation {
            Observation::Range {
                kind,
                heights,
                outcome,
            } => self.observe_range(kind, heights, &outcome, now),
            Observation::Point {
                kind,
                height,
                outcome,
            } => observe_unit(&mut self.points, (kind, height), kind.into(), outcome, now)
                .then_some(ImageRegion::Points {
                    kind,
                    heights: height..height.next(),
                }),
            Observation::Epoch {
                kind,
                epoch,
                outcome,
            } => observe_unit(
                &mut self.epoch_rows,
                (kind, epoch),
                kind.into(),
                outcome,
                now,
            )
            .then_some(ImageRegion::Epochs {
                kind,
                epochs: epoch..epoch.next(),
            }),
        }
    }

    /// Applies a range outcome over `heights`; the rewritten region, `None` when nothing changed.
    fn observe_range(
        &mut self,
        kind: RangeKind,
        heights: Range<Height>,
        outcome: &RangeOutcome,
        now: DateTime<Utc>,
    ) -> Option<ImageRegion> {
        let map = self.ranges.entry(kind).or_default();
        let before = overlapping_rows(map, &heights);
        let hull0 = before
            .iter()
            .fold(heights.clone(), |hull, (range, _)| hull_of(&hull, range));
        let pieces = match outcome {
            RangeOutcome::Present => Vec::new(),
            RangeOutcome::Failed(failure) => {
                failure_pieces(kind, map, &before, &heights, *failure, now)
            },
            RangeOutcome::Refined(search) => refined_pieces(kind, &before, search, now),
        };
        // A failure keeps every row; the other outcomes replace what lies in `heights`.
        if !matches!(outcome, RangeOutcome::Failed(_)) {
            map.remove(heights);
        }
        for (range, state) in pieces {
            map.insert(range, state);
        }
        let after = overlapping_rows(map, &hull0);
        if before == after {
            return None;
        }
        let hull = after
            .iter()
            .fold(hull0, |hull, (range, _)| hull_of(&hull, range));
        Some(ImageRegion::Range {
            kind,
            heights: hull,
        })
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
        if let Some(config) = self.epoch_config {
            for &kind in EpochKind::VARIANTS {
                let dropped = |epoch: Epoch| {
                    config
                        .root_block(kind, epoch)
                        .is_some_and(|block| block < window_start)
                };
                if let Some(epochs) = drop_units(&mut self.epoch_rows, kind, dropped, Epoch::next) {
                    regions.push(ImageRegion::Epochs { kind, epochs });
                }
            }
        }
        regions
    }

    /// The epoch config the set judges epoch rows with; also the `probes` argument.
    pub fn epoch_config(&self) -> Option<EpochConfig> {
        self.epoch_config
    }

    /// Every row, kinds in `DataKind` order, units ascending.
    pub fn rows(&self) -> impl Iterator<Item = MissingRow> + '_ {
        let ranges = self.ranges.iter().flat_map(|(&kind, map)| {
            map.iter().map(move |(heights, state)| MissingRow {
                key: RowKey::Range {
                    kind,
                    heights: heights.clone(),
                },
                state: state.clone(),
            })
        });
        let points = self
            .points
            .iter()
            .map(|(&(kind, height), state)| MissingRow {
                key: RowKey::Point { kind, height },
                state: state.clone(),
            });
        let epochs = self
            .epoch_rows
            .iter()
            .map(|(&(kind, epoch), state)| MissingRow {
                key: RowKey::Epoch { kind, epoch },
                state: state.clone(),
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
                    state: state.clone(),
                })
                .collect(),
            ImageRegion::Epochs { kind, epochs } => self
                .epoch_rows
                .range((*kind, epochs.start)..(*kind, epochs.end))
                .map(|(&(kind, epoch), state)| MissingRow {
                    key: RowKey::Epoch { kind, epoch },
                    state: state.clone(),
                })
                .collect(),
        }
    }

    /// Up to `limit` probes with `next_retry_at <= now`, ordered by (`next_retry_at`, kind, first
    /// unit); a range row longer than `CHUNK_LEN` yields consecutive probes of at most
    /// `CHUNK_LEN` heights from its start, split by `kind::next_chunk`.
    pub fn due(&self, now: DateTime<Utc>, limit: usize) -> Vec<Probe> {
        let mut due: Vec<_> = self
            .rows()
            .filter(|row| row.state.next_retry_at <= now)
            .collect();
        due.sort_by_key(|row| {
            (
                row.state.next_retry_at,
                row.key.kind(),
                row.key.first_unit(),
            )
        });
        due.iter()
            .flat_map(|row| row.key.probes())
            .take(limit)
            .collect()
    }

    /// `Known::Gap` when `heights` lies inside one row of `kind` with reason `NotFound`, else
    /// `Known::Missing`.
    pub fn known(&self, kind: RangeKind, heights: &Range<Height>) -> Known {
        let inside_not_found = self
            .ranges
            .get(&kind)
            .and_then(|map| map.get_key_value(&heights.start))
            .is_some_and(|(row, state)| {
                row.end >= heights.end && state.reason == MissingReason::NotFound
            });
        if inside_not_found {
            Known::Gap
        } else {
            Known::Missing
        }
    }

    /// The block a row is judged against the tip by: range end, point height + 1, epoch root
    /// block (`None` without epochs or on overflow).
    pub fn alert_height(&self, key: &RowKey) -> Option<Height> {
        match key {
            RowKey::Range { heights, .. } => Some(heights.end),
            RowKey::Point { height, .. } => Some(height.next()),
            RowKey::Epoch { kind, epoch } => self.epoch_config?.root_block(*kind, *epoch),
        }
    }

    /// One entry per `DataKind`: units missing and the oldest row (`first_missing_at`, then first
    /// unit) with its age; rows first missing after `now` are left out.
    pub fn summary(&self, now: DateTime<Utc>) -> Vec<KindSummary> {
        let mut by_kind: BTreeMap<DataKind, (u64, Option<MissingRow>)> = BTreeMap::new();
        for row in self.rows().filter(|row| row.state.first_missing_at <= now) {
            let (count, oldest) = by_kind.entry(row.key.kind()).or_default();
            *count += row.key.unit_count();
            if oldest.as_ref().is_none_or(|old| older(&row, old)) {
                *oldest = Some(row);
            }
        }
        DataKind::VARIANTS
            .iter()
            .map(|&kind| {
                let (missing_count, oldest) = by_kind.remove(&kind).unwrap_or_default();
                KindSummary {
                    kind,
                    missing_count,
                    oldest: oldest.map(|row| OldestMissing {
                        age: age(now, &row.state),
                        key: row.key,
                    }),
                }
            })
            .collect()
    }

    /// Up to `limit` rows first missing at or before `now`, ordered by (`first_missing_at`, first
    /// unit, kind), with ages, and the total.
    pub fn listed(&self, now: DateTime<Utc>, limit: usize) -> ListedRows {
        let mut rows: Vec<_> = self
            .rows()
            .filter(|row| row.state.first_missing_at <= now)
            .collect();
        let total = rows.len() as u64;
        rows.sort_by_key(|row| {
            (
                row.state.first_missing_at,
                row.key.first_unit(),
                row.key.kind(),
            )
        });
        let shown = rows
            .into_iter()
            .take(limit)
            .map(|row| AgedRow {
                age: age(now, &row.state),
                row,
            })
            .collect();
        ListedRows { shown, total }
    }

    /// Per kind, the oldest row (`first_missing_at`, then first unit) for which `rule.fires`; rows
    /// without an `alert_height` never fire.
    pub fn alert_candidates(
        &self,
        rule: AlertRule,
        settled_end: Height,
        now: DateTime<Utc>,
    ) -> Vec<AgedRow> {
        let mut by_kind: BTreeMap<DataKind, MissingRow> = BTreeMap::new();
        for row in self.rows() {
            let fires = self
                .alert_height(&row.key)
                .is_some_and(|height| rule.fires(&row, height, settled_end, now));
            if fires
                && by_kind
                    .get(&row.key.kind())
                    .is_none_or(|old| older(&row, old))
            {
                by_kind.insert(row.key.kind(), row);
            }
        }
        by_kind
            .into_values()
            .map(|row| AgedRow {
                age: age(now, &row.state),
                row,
            })
            .collect()
    }
}

/// When a missing row pages.
#[derive(Clone, Copy, Debug)]
pub struct AlertRule {
    pub min_age: Duration,
}

impl AlertRule {
    /// `RowState::is_confirmed`, first missing at least `min_age` before `now`, `alert_height` at
    /// or below `settled_end`.
    pub fn fires(
        &self,
        row: &MissingRow,
        alert_height: Height,
        settled_end: Height,
        now: DateTime<Utc>,
    ) -> bool {
        row.state.is_confirmed(row.key.kind())
            && row.state.first_missing_at <= now - delta(self.min_age)
            && alert_height <= settled_end
    }
}

/// State of one kind on one node.
#[derive(Clone, Debug, PartialEq)]
pub struct KindSummary {
    pub kind: DataKind,
    pub missing_count: u64,
    pub oldest: Option<OldestMissing>,
}

/// The oldest row of a kind.
#[derive(Clone, Debug, PartialEq)]
pub struct OldestMissing {
    pub key: RowKey,
    pub age: Duration,
}

/// A row and its age at the `now` it was read; a listed row or an alert candidate.
#[derive(Clone, Debug, PartialEq)]
pub struct AgedRow {
    pub row: MissingRow,
    pub age: Duration,
}

/// The oldest rows of a node and the count of all of them.
#[derive(Clone, Debug, PartialEq)]
pub struct ListedRows {
    pub shown: Vec<AgedRow>,
    pub total: u64,
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use proptest::{collection::vec, option, prelude::*};
    use reqwest::StatusCode;

    use super::*;
    use crate::{
        kind::KindClass,
        outcome::{FailedStatus, NotServed, Tail},
    };

    const T0: i64 = 1_000;
    const MIN_AGE: Duration = Duration::from_secs(60);

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn h(range: Range<u64>) -> Range<Height> {
        Height(range.start)..Height(range.end)
    }

    fn status(code: StatusCode) -> Failure {
        Failure::Status(FailedStatus::new(code).unwrap())
    }

    fn server_error() -> Failure {
        status(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn observe(set: &mut MissingSet, observation: Observation, secs: i64) -> ImageDiff {
        set.apply(MissingUpdate::Observed(observation), at(secs))
    }

    fn range_obs(kind: RangeKind, range: Range<u64>, outcome: RangeOutcome) -> Observation {
        Observation::Range {
            kind,
            heights: h(range),
            outcome,
        }
    }

    /// A 404 whose search found the whole range missing.
    fn gap_obs(kind: RangeKind, range: Range<u64>) -> Observation {
        let outcome = RangeOutcome::Refined(GapSearch {
            gaps: vec![h(range.clone())],
            tail: None,
        });
        range_obs(kind, range, outcome)
    }

    fn failed_obs(kind: RangeKind, range: Range<u64>) -> Observation {
        range_obs(kind, range, RangeOutcome::Failed(server_error()))
    }

    fn present_obs(kind: RangeKind, range: Range<u64>) -> Observation {
        range_obs(kind, range, RangeOutcome::Present)
    }

    fn refined_obs(
        kind: RangeKind,
        range: Range<u64>,
        gaps: &[Range<u64>],
        tail: Option<(Range<u64>, Unconfirmed)>,
    ) -> Observation {
        let outcome = RangeOutcome::Refined(GapSearch {
            gaps: gaps.iter().cloned().map(h).collect(),
            tail: tail.map(|(heights, reason)| Tail {
                heights: h(heights),
                reason,
            }),
        });
        range_obs(kind, range, outcome)
    }

    fn one(range: Range<u64>) -> Vec<Range<u64>> {
        vec![range]
    }

    fn point_obs(kind: PointKind, height: u64, outcome: UnitOutcome) -> Observation {
        Observation::Point {
            kind,
            height: Height(height),
            outcome,
        }
    }

    fn epoch_obs(kind: EpochKind, epoch: u64, outcome: UnitOutcome) -> Observation {
        Observation::Epoch {
            kind,
            epoch: Epoch(epoch),
            outcome,
        }
    }

    fn missing(not_served: NotServed) -> UnitOutcome {
        UnitOutcome::Missing(not_served)
    }

    fn count(set: &MissingSet, now: i64, kind: DataKind) -> u64 {
        set.summary(at(now))
            .iter()
            .find(|summary| summary.kind == kind)
            .unwrap()
            .missing_count
    }

    fn oldest(set: &MissingSet, now: i64, kind: DataKind) -> OldestMissing {
        set.summary(at(now))
            .into_iter()
            .find(|summary| summary.kind == kind)
            .unwrap()
            .oldest
            .unwrap()
    }

    fn range_key(kind: RangeKind, range: Range<u64>) -> RowKey {
        RowKey::Range {
            kind,
            heights: h(range),
        }
    }

    /// Listed rows as `(key, age seconds)`.
    fn listed(set: &MissingSet, now: i64) -> Vec<(RowKey, u64)> {
        set.listed(at(now), 100)
            .shown
            .into_iter()
            .map(|aged| (aged.row.key, aged.age.as_secs()))
            .collect()
    }

    fn rule() -> AlertRule {
        AlertRule { min_age: MIN_AGE }
    }

    #[test]
    fn missing_backoff_and_summary() {
        let mut set = MissingSet::new(Some(EpochConfig {
            epoch_height: 30,
            epoch_start_block: 61,
        }));
        assert!(
            set.summary(at(T0))
                .iter()
                .all(|k| k.missing_count == 0 && k.oldest.is_none())
        );
        assert_eq!(set.summary(at(T0)).len(), DataKind::VARIANTS.len());

        let leaf = Probe::Range {
            kind: RangeKind::Leaf,
            heights: h(0..100),
        };
        observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), T0);
        observe(
            &mut set,
            epoch_obs(EpochKind::StakeTable, 5, missing(NotServed::NotFound)),
            T0 + 50,
        );
        assert!(set.due(at(T0 + 4), 10).is_empty());
        assert_eq!(set.due(at(T0 + 5), 10), std::slice::from_ref(&leaf));

        // The delay doubles from 5 s; a 404 row with 3 failed probes stops at 60 s.
        let mut now = T0 + 5;
        for delay in [10, 20, 40, 60, 60, 60] {
            observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), now);
            assert!(!set.due(at(now + delay - 1), 10).contains(&leaf));
            assert!(set.due(at(now + delay), 10).contains(&leaf));
            now += delay;
        }

        let leaf_oldest = oldest(&set, T0 + 100, DataKind::Leaf);
        assert_eq!(leaf_oldest.key, range_key(RangeKind::Leaf, 0..100));
        assert_eq!(leaf_oldest.age, Duration::from_secs(100));
        assert_eq!(count(&set, T0 + 100, DataKind::Leaf), 100);
        assert_eq!(count(&set, T0 + 100, DataKind::StakeTable), 1);

        // A later row does not displace the oldest. Touching rows with the same state merge.
        for (kind, range, secs) in [
            (RangeKind::Leaf, 100..200, 20),
            (RangeKind::Block, 300..400, 30),
            (RangeKind::Block, 200..300, 30),
        ] {
            observe(&mut set, gap_obs(kind, range), T0 + secs);
        }
        assert_eq!(count(&set, T0 + 100, DataKind::Leaf), 200);
        assert_eq!(
            oldest(&set, T0 + 100, DataKind::Leaf).key,
            range_key(RangeKind::Leaf, 0..100)
        );
        assert_eq!(count(&set, T0 + 100, DataKind::Block), 200);
        let block = oldest(&set, T0 + 100, DataKind::Block);
        assert_eq!(block.key, range_key(RangeKind::Block, 200..400));
        assert_eq!(block.age, Duration::from_secs(70));

        // A row first missing after `now` is ignored instead of failing the age.
        observe(&mut set, gap_obs(RangeKind::Leaf, 900..1000), T0 + 500);
        assert_eq!(count(&set, T0 + 100, DataKind::Leaf), 200);

        let page = set.listed(at(T0 + 100), 3);
        let order: Vec<_> = page.shown.iter().map(|aged| aged.row.key.clone()).collect();
        assert_eq!(
            order,
            [
                range_key(RangeKind::Leaf, 0..100),
                range_key(RangeKind::Leaf, 100..200),
                range_key(RangeKind::Block, 200..400),
            ]
        );
        assert_eq!(page.total, 4);

        let other = MissingSet::new(None);
        assert_eq!(
            other
                .summary(at(T0))
                .iter()
                .map(|k| k.missing_count)
                .sum::<u64>(),
            0
        );
    }

    /// One failed probe of `kind` over `[0, 100)` (height 99 for point kinds), as the loop sees a
    /// 404 (`NotFound`) or a 500.
    fn fail_kind(set: &mut MissingSet, kind: DataKind, not_found: bool, secs: i64) {
        let observation = match kind.class() {
            KindClass::Range(kind) if not_found => gap_obs(kind, 0..100),
            KindClass::Range(kind) => failed_obs(kind, 0..100),
            KindClass::Point(kind) => point_obs(
                kind,
                99,
                missing(if not_found {
                    NotServed::NotFound
                } else {
                    NotServed::Failed(server_error())
                }),
            ),
            KindClass::Epoch(_) => unreachable!("range and point kinds only"),
        };
        observe(set, observation, secs);
    }

    #[test]
    fn retry_delay_stops_at_a_minute_only_for_alerting_rows() {
        let cases = [
            (DataKind::Leaf, true, vec![10, 20, 40, 60, 60]),
            // The aggregate never alerts, so its 404 rows back off up to 600 s.
            (
                DataKind::Aggregate,
                true,
                vec![10, 20, 40, 80, 160, 320, 600, 600],
            ),
            (
                DataKind::Block,
                false,
                vec![10, 20, 40, 80, 160, 320, 600, 600],
            ),
        ];
        for (kind, not_found, delays) in cases {
            let mut set = MissingSet::new(None);
            fail_kind(&mut set, kind, not_found, T0);
            let mut now = T0 + 5;
            for delay in delays {
                fail_kind(&mut set, kind, not_found, now);
                assert!(
                    set.due(at(now + delay - 1), 10).is_empty(),
                    "{kind:?} before {delay}"
                );
                assert_eq!(
                    set.due(at(now + delay), 10).len(),
                    1,
                    "{kind:?} after {delay}"
                );
                now += delay;
            }
        }
    }

    #[test]
    fn rows_of_single_heights_stay_apart() {
        // Follow probed up to 1107 and found height 1106 missing; the sweep chunk 1100..1200
        // probes only height 1199. The two rows are keyed by the heights they probed.
        let mut set = MissingSet::new(None);
        let kind = PointKind::LcLeafProof;
        observe(
            &mut set,
            point_obs(kind, 1106, missing(NotServed::NotFound)),
            T0,
        );

        observe(
            &mut set,
            point_obs(kind, 1199, UnitOutcome::Present),
            T0 + 1,
        );
        assert_eq!(
            count(&set, T0 + 2, kind.into()),
            1,
            "1199 present keeps 1106"
        );

        observe(
            &mut set,
            point_obs(kind, 1199, missing(NotServed::NotFound)),
            T0 + 3,
        );
        assert_eq!(count(&set, T0 + 4, kind.into()), 2);
        let due = set.due(at(T0 + 1_000), 10);
        assert_eq!(due.len(), 2, "{due:?}");

        observe(
            &mut set,
            point_obs(kind, 1106, UnitOutcome::Present),
            T0 + 5,
        );
        assert_eq!(count(&set, T0 + 6, kind.into()), 1);
    }

    #[test]
    fn an_exact_gap_found_in_a_failed_row_starts_fresh() {
        let mut set = MissingSet::new(None);
        for secs in 0..3 {
            observe(&mut set, failed_obs(RangeKind::Leaf, 0..100), T0 + secs);
        }

        // Refining the wide row finds a gap: a new 404 row, not a continuation of the 500s.
        observe(
            &mut set,
            refined_obs(RangeKind::Leaf, 0..100, &one(10..20), None),
            T0 + 100,
        );
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, range_key(RangeKind::Leaf, 10..20));
        assert_eq!(rows[0].state.failed_probes, 1);
        assert_eq!(rows[0].state.first_missing_at, at(T0 + 100));
        let zero_age = AlertRule {
            min_age: Duration::ZERO,
        };
        assert!(
            set.alert_candidates(zero_age, Height(1_000), at(T0 + 100))
                .is_empty(),
            "no alert before 3 probes of its own"
        );

        // Two more 404 probes of the gap bring it to 3 and it alerts, with its own age.
        for secs in [101, 102] {
            observe(&mut set, gap_obs(RangeKind::Leaf, 10..20), T0 + secs);
        }
        let alerting = set.alert_candidates(zero_age, Height(1_000), at(T0 + 102));
        assert_eq!(alerting.len(), 1);
        assert_eq!(alerting[0].age.as_secs(), 2);
    }

    #[test]
    fn epoch_rows_before_the_root_lag_depend_on_block_zero() {
        let epoch_config = EpochConfig {
            epoch_height: 100,
            epoch_start_block: 1,
        };
        let mut set = MissingSet::new(Some(epoch_config));
        for secs in 0..3 {
            observe(
                &mut set,
                epoch_obs(EpochKind::StakeTable, 1, missing(NotServed::NotFound)),
                T0 + secs,
            );
        }
        let found = set.alert_candidates(rule(), Height(0), at(T0 + 60));
        assert_eq!(found.len(), 1, "epoch 1 of a stake table rests on block 0");
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(1)), at(T0 + 60));
        assert_eq!(diff.rows_removed, 1);
    }

    #[test]
    fn found_trims_rows_to_the_heights_still_missing() {
        let mut set = MissingSet::new(None);
        observe(&mut set, gap_obs(RangeKind::Leaf, 100..200), T0);
        observe(&mut set, gap_obs(RangeKind::Leaf, 100..150), T0 + 1);
        assert_eq!(count(&set, T0 + 2, DataKind::Leaf), 100);

        observe(&mut set, present_obs(RangeKind::Leaf, 100..150), T0 + 2);
        assert_eq!(count(&set, T0 + 2, DataKind::Leaf), 50);
        assert_eq!(
            oldest(&set, T0 + 2, DataKind::Leaf).key,
            range_key(RangeKind::Leaf, 150..200)
        );

        observe(&mut set, present_obs(RangeKind::Leaf, 100..200), T0 + 2);
        assert_eq!(count(&set, T0 + 2, DataKind::Leaf), 0);
    }

    #[test]
    fn forget_below_window_start() {
        let epoch_config = EpochConfig {
            epoch_height: 25,
            epoch_start_block: 1,
        };
        let mut set = MissingSet::new(Some(epoch_config));
        let not_found = missing(NotServed::NotFound);
        for range in [0..60, 80..140, 150..200] {
            observe(&mut set, gap_obs(RangeKind::Leaf, range), T0);
        }
        for height in [99, 150] {
            observe(
                &mut set,
                point_obs(PointKind::LcLeafProof, height, not_found),
                T0,
            );
        }
        for epoch in [3, 4] {
            observe(
                &mut set,
                epoch_obs(EpochKind::StateCert, epoch, not_found),
                T0,
            );
        }
        for epoch in [5, 6] {
            observe(
                &mut set,
                epoch_obs(EpochKind::StakeTable, epoch, not_found),
                T0,
            );
        }

        // Window starts at 100. Leaf 0..60 and lc_leaf_proof 99 end below it and go; leaf 80..140
        // crosses it and is trimmed to 100..140. The state cert of epoch 3 depends on block 75 and
        // goes, epoch 4 on block 100 and stays. The stake table of epoch 5 depends on block 75
        // (two epochs back) and goes, epoch 6 on block 100 and stays.
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(100)), at(T0));
        assert_eq!(diff.rows_removed, 4);
        assert_eq!(count(&set, T0, DataKind::Leaf), 90);
        assert_eq!(
            oldest(&set, T0, DataKind::Leaf).key,
            range_key(RangeKind::Leaf, 100..140)
        );
        assert_eq!(count(&set, T0, DataKind::LcLeafProof), 1);
        assert_eq!(count(&set, T0, DataKind::StateCert), 1);
        assert_eq!(count(&set, T0, DataKind::StakeTable), 1);
    }

    #[test]
    fn refined_inherits_age_and_backoff() {
        let mut set = MissingSet::new(None);
        observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), T0);
        observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), T0 + 5);

        observe(
            &mut set,
            refined_obs(RangeKind::Leaf, 0..100, &[10..20, 50..60], None),
            T0 + 30,
        );
        assert_eq!(count(&set, T0 + 100, DataKind::Leaf), 20);
        let leaf = oldest(&set, T0 + 100, DataKind::Leaf);
        assert_eq!(
            (leaf.key, leaf.age),
            (range_key(RangeKind::Leaf, 10..20), Duration::from_secs(100))
        );
        // The old delay was 10 s, so the new rows retry 20 s after the replacement.
        assert!(set.due(at(T0 + 49), 10).is_empty());
        assert_eq!(set.due(at(T0 + 50), 10).len(), 2);

        observe(
            &mut set,
            refined_obs(RangeKind::Leaf, 100..200, &one(120..130), None),
            T0 + 40,
        );
        assert_eq!(set.due(at(T0 + 45), 10).len(), 1);

        observe(&mut set, present_obs(RangeKind::Leaf, 0..100), T0 + 100);
        assert_eq!(count(&set, T0 + 100, DataKind::Leaf), 10);
    }

    #[test]
    fn point_kinds_count_one_per_row() {
        let mut set = MissingSet::new(None);
        for kind in [PointKind::LcLeafProof, PointKind::Aggregate] {
            observe(
                &mut set,
                point_obs(kind, 99, missing(NotServed::NotFound)),
                T0,
            );
        }
        assert_eq!(count(&set, T0, DataKind::LcLeafProof), 1);
        assert_eq!(count(&set, T0, DataKind::Aggregate), 1);
    }

    /// `times` observations of a missing unit or range at `first`, `first + 1`, ...
    fn repeat(set: &mut MissingSet, observation: &Observation, times: i64, first: i64) {
        for i in 0..times {
            observe(set, observation.clone(), first + i);
        }
    }

    #[test]
    fn alert_candidates_apply_every_condition() {
        let epoch_config = EpochConfig {
            epoch_height: 100,
            epoch_start_block: 1,
        };
        let mut set = MissingSet::new(Some(epoch_config));
        let not_found = missing(NotServed::NotFound);
        repeat(&mut set, &gap_obs(RangeKind::Leaf, 100..200), 3, T0);
        repeat(&mut set, &gap_obs(RangeKind::Leaf, 50..100), 3, T0 + 10);
        repeat(&mut set, &failed_obs(RangeKind::Block, 100..200), 3, T0);
        repeat(&mut set, &gap_obs(RangeKind::VidCommon, 100..200), 2, T0);
        repeat(
            &mut set,
            &point_obs(PointKind::LcLeafProof, 199, not_found),
            3,
            T0 + 50,
        );
        for height in [999, 499] {
            repeat(
                &mut set,
                &point_obs(PointKind::Aggregate, height, not_found),
                3,
                T0,
            );
        }
        repeat(
            &mut set,
            &epoch_obs(EpochKind::StateCert, 3, not_found),
            3,
            T0,
        );
        for epoch in [8, 7] {
            repeat(
                &mut set,
                &epoch_obs(EpochKind::StakeTable, epoch, not_found),
                3,
                T0,
            );
        }

        let now = at(T0 + 60);
        let summary = |set: &MissingSet, now| -> Vec<_> {
            set.alert_candidates(rule(), Height(500), now)
                .iter()
                .map(|c| {
                    (
                        c.row.key.kind(),
                        c.row.key.first_unit(),
                        c.age.as_secs(),
                        c.row.state.failed_probes,
                    )
                })
                .collect()
        };
        // Not alerting: block (500), vid_common (2 probes), lc_leaf_proof (too young), aggregate
        // (never), and the stake_table row past the margin. Leaf takes the older of its two rows.
        // A stake table depends on the block two epochs back: epoch 7 on block 500, exactly the
        // margin, so it alerts; epoch 8 on block 600 does not.
        assert_eq!(
            summary(&set, now),
            [
                (DataKind::Leaf, 100, 60, 3),
                (DataKind::StakeTable, 7, 60, 3),
                (DataKind::StateCert, 3, 60, 3),
            ]
        );

        // A 404 after 500s starts the row afresh: the 500s do not count toward the alert.
        observe(&mut set, gap_obs(RangeKind::Block, 100..200), T0 + 3);
        assert!(summary(&set, now).iter().all(|c| c.0 != DataKind::Block));
        repeat(&mut set, &gap_obs(RangeKind::Block, 100..200), 2, T0 + 4);
        let found = summary(&set, at(T0 + 70));
        let block = found.iter().find(|c| c.0 == DataKind::Block).unwrap();
        assert_eq!(block.3, 3);
    }

    #[test]
    fn refinement_inherits_failed_probes_and_unconfirmed_never_alerts() {
        let mut set = MissingSet::new(None);
        for secs in [0, 5] {
            observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), T0 + secs);
        }
        let tail = (50..100, Unconfirmed::Failed(server_error()));
        observe(
            &mut set,
            refined_obs(RangeKind::Leaf, 0..100, &one(10..20), Some(tail)),
            T0 + 30,
        );
        let found = set.alert_candidates(rule(), Height(1000), at(T0 + 100));
        let rows: Vec<_> = found
            .iter()
            .map(|c| (c.row.key.clone(), c.row.state.failed_probes))
            .collect();
        assert_eq!(rows, [(range_key(RangeKind::Leaf, 10..20), 3)]);
    }

    #[test]
    fn found_keeps_the_part_of_a_row_that_crosses_its_edge() {
        let mut set = MissingSet::new(None);
        observe(&mut set, gap_obs(RangeKind::Block, 90..120), T0);
        observe(&mut set, present_obs(RangeKind::Block, 100..200), T0 + 10);
        assert_eq!(
            listed(&set, T0 + 10),
            [(range_key(RangeKind::Block, 90..100), 10)]
        );
    }

    #[test]
    fn found_clears_exact_keys_of_point_kinds() {
        let mut set = MissingSet::new(None);
        let kind = PointKind::LcLeafProof;
        for height in [199, 179] {
            observe(
                &mut set,
                point_obs(kind, height, missing(NotServed::NotFound)),
                T0,
            );
        }
        observe(&mut set, point_obs(kind, 199, UnitOutcome::Present), T0);
        assert_eq!(
            count(&set, T0, kind.into()),
            1,
            "only the row at the same height goes"
        );
    }

    #[test]
    fn refined_keeps_the_parts_outside_the_range() {
        let mut set = MissingSet::new(None);
        observe(&mut set, gap_obs(RangeKind::Leaf, 50..150), T0);
        observe(&mut set, gap_obs(RangeKind::Leaf, 150..250), T0 + 10);
        observe(
            &mut set,
            refined_obs(RangeKind::Leaf, 100..200, &one(120..130), None),
            T0 + 30,
        );
        let key = |range| range_key(RangeKind::Leaf, range);
        assert_eq!(
            listed(&set, T0 + 100),
            [
                (key(50..100), 100),
                (key(120..130), 100),
                (key(200..250), 90)
            ]
        );
        assert_eq!(
            count(&set, T0 + 100, DataKind::Leaf),
            110,
            "no height counted twice"
        );
    }

    #[test]
    fn a_failed_chunk_probe_leaves_refined_rows_alone() {
        let mut set = MissingSet::new(None);
        let leaf = RangeKind::Leaf;
        observe(
            &mut set,
            refined_obs(leaf, 100..200, &one(110..115), None),
            T0,
        );
        observe(&mut set, gap_obs(leaf, 115..130), T0 + 5);

        observe(&mut set, failed_obs(leaf, 100..200), T0 + 30);
        let key = |range| range_key(leaf, range);
        // The refined rows keep their age; the heights no row covers become new rows.
        assert_eq!(
            listed(&set, T0 + 100),
            [
                (key(110..115), 100),
                (key(115..130), 95),
                (key(100..110), 70),
                (key(130..200), 70),
            ]
        );
        assert_eq!(
            count(&set, T0 + 100, DataKind::Leaf),
            100,
            "no height counted twice"
        );
        let reasons: Vec<_> = set.rows().map(|row| row.state.reason).collect();
        assert_eq!(
            reasons
                .iter()
                .filter(|r| **r == MissingReason::NotFound)
                .count(),
            2
        );

        // The same failure again backs the exact row off instead of adding rows.
        observe(&mut set, failed_obs(leaf, 110..115), T0 + 40);
        assert_eq!(set.listed(at(T0 + 100), 10).total, 4);
    }

    #[test]
    fn a_failure_inside_a_confirmed_row_leaves_it_alone() {
        let mut set = MissingSet::new(None);
        let leaf = RangeKind::Leaf;
        repeat(&mut set, &gap_obs(leaf, 100..300), 3, T0);
        let before: Vec<_> = set.rows().collect();

        // Inside the row, then across its edge: the row stays, the uncovered heights are new.
        let diff = observe(&mut set, failed_obs(leaf, 150..200), T0 + 10);
        assert!(diff.regions.is_empty());
        assert_eq!(set.rows().collect::<Vec<_>>(), before);
        observe(&mut set, failed_obs(leaf, 250..350), T0 + 11);
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], before[0]);
        assert_eq!(rows[1].key, range_key(leaf, 300..350));
        assert_eq!(rows[1].state.failed_probes, 1);
        assert_eq!(
            alerting(&set, T0 + 100),
            [RowKey::Range {
                kind: leaf,
                heights: h(100..300)
            }]
        );
    }

    /// Keys of the rows that alert at `secs`.
    fn alerting(set: &MissingSet, secs: i64) -> Vec<RowKey> {
        set.alert_candidates(rule(), Height(1_000), at(secs))
            .into_iter()
            .map(|aged| aged.row.key)
            .collect()
    }

    #[test]
    fn forget_below_zero_changes_nothing() {
        let mut set = MissingSet::new(Some(EpochConfig {
            epoch_height: 30,
            epoch_start_block: 61,
        }));
        let not_found = missing(NotServed::NotFound);
        observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), T0);
        observe(&mut set, point_obs(PointKind::Aggregate, 5, not_found), T0);
        observe(&mut set, epoch_obs(EpochKind::StateCert, 3, not_found), T0);
        let before: Vec<_> = set.rows().collect();
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(0)), at(T0 + 5));
        assert_eq!((diff.regions.len(), diff.rows_removed), (0, 0));
        assert_eq!(set.rows().collect::<Vec<_>>(), before);
    }

    #[test]
    fn touching_rows_with_equal_state_merge_past_a_chunk() {
        let mut set = MissingSet::new(None);
        observe(&mut set, gap_obs(RangeKind::Leaf, 100..200), T0);
        observe(&mut set, gap_obs(RangeKind::Leaf, 200..300), T0);
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].key.first_probe(),
            Probe::Range {
                kind: RangeKind::Leaf,
                heights: h(100..200),
            }
        );
        let probes = set.due(at(T0 + 5), 10);
        let heights: Vec<_> = probes
            .iter()
            .map(|probe| match probe {
                Probe::Range { heights, .. } => heights.clone(),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(heights, [h(100..200), h(200..300)]);
    }

    #[test]
    fn a_failed_retry_of_a_piece_splits_the_row() {
        let mut set = MissingSet::new(None);
        observe(&mut set, failed_obs(RangeKind::Leaf, 100..300), T0);
        observe(&mut set, failed_obs(RangeKind::Leaf, 100..200), T0 + 5);
        let rows: Vec<_> = set.rows().collect();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].key, range_key(RangeKind::Leaf, 100..200));
        assert_eq!(rows[0].state.failed_probes, 2);
        assert_eq!(rows[1].state.failed_probes, 1);
    }

    #[test]
    fn rows_first_missing_after_now_are_left_out() {
        let mut set = MissingSet::new(None);
        for secs in 0..3 {
            observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), T0 + 500 + secs);
        }
        let now = at(T0);
        assert_eq!(count(&set, T0, DataKind::Leaf), 0);
        assert!(oldest_is_none(&set, T0));
        assert_eq!(set.listed(now, 10).total, 0);
        assert!(set.alert_candidates(rule(), Height(1_000), now).is_empty());
    }

    fn oldest_is_none(set: &MissingSet, now: i64) -> bool {
        set.summary(at(now)).iter().all(|k| k.oldest.is_none())
    }

    #[test]
    fn epoch_rows_without_a_config_are_kept_and_never_alert() {
        let mut set = MissingSet::new(None);
        repeat(
            &mut set,
            &epoch_obs(EpochKind::StateCert, 5, missing(NotServed::NotFound)),
            3,
            T0,
        );
        assert!(
            set.alert_candidates(
                AlertRule {
                    min_age: Duration::ZERO
                },
                Height(u64::MAX),
                at(T0 + 1_000)
            )
            .is_empty()
        );
        let diff = set.apply(
            MissingUpdate::ForgetBelow(Height(1_000_000)),
            at(T0 + 1_000),
        );
        assert_eq!((diff.regions.len(), diff.rows_removed), (0, 0));
        assert_eq!(set.due(at(T0 + 1_000), 10).len(), 1);
    }

    #[test]
    fn epoch_rows_with_an_overflowing_root_block_are_kept() {
        let epoch_config = EpochConfig {
            epoch_height: 30,
            epoch_start_block: 61,
        };
        let mut set = MissingSet::new(Some(epoch_config));
        let epoch = u64::MAX / 2;
        assert_eq!(
            epoch_config.root_block(EpochKind::StateCert, Epoch(epoch)),
            None
        );
        repeat(
            &mut set,
            &epoch_obs(EpochKind::StateCert, epoch, missing(NotServed::NotFound)),
            3,
            T0,
        );
        assert!(
            set.alert_candidates(
                AlertRule {
                    min_age: Duration::ZERO
                },
                Height(u64::MAX),
                at(T0 + 1_000)
            )
            .is_empty()
        );
        let diff = set.apply(MissingUpdate::ForgetBelow(Height(u64::MAX)), at(T0 + 1_000));
        assert_eq!(diff.rows_removed, 0);
    }

    #[test]
    fn an_empty_refined_search_equals_present() {
        let mut set = MissingSet::new(None);
        observe(&mut set, gap_obs(RangeKind::Leaf, 0..100), T0);
        observe(
            &mut set,
            refined_obs(RangeKind::Leaf, 0..100, &[], None),
            T0 + 5,
        );
        assert_eq!(set.rows().count(), 0);
    }

    #[test]
    fn known_gap_needs_a_not_found_row_around_the_probe() {
        let mut set = MissingSet::new(None);
        observe(&mut set, gap_obs(RangeKind::Leaf, 100..300), T0);
        observe(&mut set, failed_obs(RangeKind::Block, 100..300), T0);
        assert_eq!(set.known(RangeKind::Leaf, &h(150..200)), Known::Gap);
        assert_eq!(set.known(RangeKind::Leaf, &h(250..350)), Known::Missing);
        assert_eq!(set.known(RangeKind::Block, &h(150..200)), Known::Missing);
    }

    #[test]
    fn from_rows_rejects_overlapping_and_touching_equal_rows() {
        let state = |secs| RowState {
            first_missing_at: at(secs),
            failed_probes: 1,
            retry_delay: RETRY_DELAY_MIN,
            next_retry_at: at(secs + 5),
            reason: MissingReason::NotFound,
        };
        let row = |range, secs| MissingRow {
            key: range_key(RangeKind::Leaf, range),
            state: state(secs),
        };
        assert!(MissingSet::from_rows(None, [row(0..10, T0), row(5..15, T0 + 1)]).is_err());
        assert!(MissingSet::from_rows(None, [row(0..10, T0), row(10..20, T0)]).is_err());
        assert!(MissingSet::from_rows(None, [row(0..10, T0), row(10..20, T0 + 1)]).is_ok());
    }

    #[test]
    fn rows_come_out_sorted_by_kind() {
        let mut set = MissingSet::new(Some(EpochConfig {
            epoch_height: 30,
            epoch_start_block: 61,
        }));
        let not_found = missing(NotServed::NotFound);
        observe(
            &mut set,
            epoch_obs(EpochKind::LcStakeTable, 7, not_found),
            T0,
        );
        observe(&mut set, point_obs(PointKind::Aggregate, 5, not_found), T0);
        observe(&mut set, gap_obs(RangeKind::VidCommon, 0..10), T0);
        observe(&mut set, gap_obs(RangeKind::Leaf, 0..10), T0);
        let kinds: Vec<_> = set.rows().map(|row| row.key.kind()).collect();
        assert!(kinds.windows(2).all(|pair| pair[0] < pair[1]), "{kinds:?}");
    }

    // Property test against a per-unit reference model.

    const EPOCH_CONFIG: EpochConfig = EpochConfig {
        epoch_height: 30,
        epoch_start_block: 61,
    };
    const SETTLED_END: Height = Height(300);

    type Unit = (DataKind, u64);

    /// A relative range outcome: gaps are offsets into the probed range.
    #[derive(Clone, Debug)]
    enum RelOutcome {
        Present,
        Failed(Failure),
        Refined {
            gaps: Vec<Range<u64>>,
            tail: Option<(u64, Unconfirmed)>,
        },
    }

    #[derive(Clone, Debug)]
    enum Op {
        Range {
            kind: RangeKind,
            start: u64,
            len: u64,
            outcome: RelOutcome,
        },
        Point {
            kind: PointKind,
            height: u64,
            outcome: UnitOutcome,
        },
        Epoch {
            kind: EpochKind,
            epoch: u64,
            outcome: UnitOutcome,
        },
        Forget(u64),
        Advance(u64),
        Retry(bool),
    }

    fn failure() -> impl Strategy<Value = Failure> {
        prop_oneof![
            Just(Failure::Transport),
            Just(server_error()),
            Just(status(StatusCode::SERVICE_UNAVAILABLE)),
        ]
    }

    fn unconfirmed() -> impl Strategy<Value = Unconfirmed> {
        prop_oneof![
            Just(Unconfirmed::NoHeight),
            Just(Unconfirmed::OutOfRange),
            failure().prop_map(Unconfirmed::Failed),
        ]
    }

    fn unit_outcome() -> impl Strategy<Value = UnitOutcome> {
        prop_oneof![
            Just(UnitOutcome::Present),
            Just(UnitOutcome::Missing(NotServed::NotFound)),
            failure().prop_map(|f| UnitOutcome::Missing(NotServed::Failed(f))),
        ]
    }

    /// Gaps are the maximal true runs of a mask below the optional tail start. A gap may end where
    /// the tail starts, as a real search leaves it.
    fn refined(len: u64) -> impl Strategy<Value = RelOutcome> {
        (
            vec(any::<bool>(), len as usize),
            option::of((0..len, unconfirmed())),
        )
            .prop_map(|(mask, tail)| {
                let end = tail
                    .as_ref()
                    .map_or(mask.len(), |(start, _)| *start as usize);
                let mut gaps = Vec::new();
                let mut start = None;
                for (i, &on) in mask[..end].iter().enumerate() {
                    match (on, start) {
                        (true, None) => start = Some(i as u64),
                        (false, Some(s)) => {
                            gaps.push(s..i as u64);
                            start = None;
                        },
                        _ => {},
                    }
                }
                if let Some(s) = start {
                    gaps.push(s..end as u64);
                }
                RelOutcome::Refined { gaps, tail }
            })
    }

    /// Ops over heights `0..heights` and epochs `0..epochs`; a small domain repeats units often.
    fn op(heights: u64, epochs: u64) -> impl Strategy<Value = Op> {
        let range = (
            prop::sample::select(RangeKind::VARIANTS),
            0..heights,
            1..=crate::kind::CHUNK_LEN.min(heights),
        )
            .prop_flat_map(|(kind, start, len)| {
                let outcome = prop_oneof![
                    Just(RelOutcome::Present),
                    failure().prop_map(RelOutcome::Failed),
                    refined(len),
                ];
                (Just(kind), Just(start), Just(len), outcome)
            })
            .prop_map(|(kind, start, len, outcome)| Op::Range {
                kind,
                start,
                len,
                outcome,
            });
        let point = (
            prop::sample::select(PointKind::VARIANTS),
            0..heights,
            unit_outcome(),
        )
            .prop_map(|(kind, height, outcome)| Op::Point {
                kind,
                height,
                outcome,
            });
        let epoch = (
            prop::sample::select(EpochKind::VARIANTS),
            0..epochs,
            unit_outcome(),
        )
            .prop_map(|(kind, epoch, outcome)| Op::Epoch {
                kind,
                epoch,
                outcome,
            });
        prop_oneof![
            4 => range,
            2 => point,
            2 => epoch,
            1 => (0..heights).prop_map(Op::Forget),
            4 => prop_oneof![0u64..12, 0u64..700].prop_map(Op::Advance),
            8 => prop_oneof![1 => Just(true), 3 => Just(false)].prop_map(Op::Retry),
        ]
    }

    /// One state per unit (a height, or an epoch number) with the time it was written. The rules
    /// are written per unit and spelled out, not shared with `MissingSet`.
    #[derive(Default)]
    struct Model {
        units: BTreeMap<Unit, (RowState, DateTime<Utc>)>,
    }

    /// A maximal run of consecutive units of one range kind with one state; one run per other
    /// unit.
    struct Run {
        kind: DataKind,
        units: Range<u64>,
        state: RowState,
    }

    fn seconds(secs: u64) -> TimeDelta {
        TimeDelta::seconds(secs as i64)
    }

    /// The block an epoch's object is built from: two epochs back for stake tables.
    fn model_root_block(kind: DataKind, epoch: u64) -> u64 {
        let lag = if matches!(kind, DataKind::StakeTable | DataKind::LcStakeTable) {
            2
        } else {
            0
        };
        epoch.saturating_sub(lag) * 30
    }

    /// Confirmed 404 rows of alerting kinds retry at most once a minute.
    fn model_confirmed(kind: DataKind, state: &RowState) -> bool {
        kind != DataKind::Aggregate
            && state.reason == MissingReason::NotFound
            && state.failed_probes >= 3
    }

    /// The state after one more failed probe, continuing the states of the units it covers.
    fn model_state(
        kind: DataKind,
        olds: &[RowState],
        reason: MissingReason,
        now: DateTime<Utc>,
    ) -> RowState {
        let counted: Vec<&RowState> = if reason == MissingReason::NotFound {
            olds.iter()
                .filter(|old| old.reason == MissingReason::NotFound)
                .collect()
        } else {
            olds.iter().collect()
        };
        let mut state = RowState {
            first_missing_at: now,
            failed_probes: 1,
            retry_delay: Duration::from_secs(5),
            next_retry_at: now + seconds(5),
            reason,
        };
        if counted.is_empty() {
            return state;
        }
        state.first_missing_at = counted.iter().map(|o| o.first_missing_at).min().unwrap();
        state.failed_probes = counted.iter().map(|o| o.failed_probes).max().unwrap() + 1;
        let doubled = counted
            .iter()
            .map(|o| o.retry_delay.as_secs())
            .max()
            .unwrap()
            * 2;
        let cap = if model_confirmed(kind, &state) {
            60
        } else {
            600
        };
        state.retry_delay = Duration::from_secs(doubled.min(cap));
        state.next_retry_at = now + seconds(state.retry_delay.as_secs());
        state
    }

    impl Model {
        fn state(&self, unit: Unit) -> Option<&RowState> {
            self.units.get(&unit).map(|(state, _)| state)
        }

        fn put(&mut self, unit: Unit, state: RowState, now: DateTime<Utc>) {
            self.units.insert(unit, (state, now));
        }

        fn states_in(&self, kind: DataKind, units: Range<u64>) -> Vec<RowState> {
            units
                .filter_map(|unit| self.state((kind, unit)).cloned())
                .collect()
        }

        fn apply(&mut self, observation: &Observation, now: DateTime<Utc>) {
            match observation {
                Observation::Range {
                    kind,
                    heights,
                    outcome,
                } => {
                    let units = heights.start.0..heights.end.0;
                    self.apply_range((*kind).into(), units, outcome, now)
                },
                Observation::Point {
                    kind,
                    height,
                    outcome,
                } => self.apply_unit((*kind).into(), height.0, *outcome, now),
                Observation::Epoch {
                    kind,
                    epoch,
                    outcome,
                } => self.apply_unit((*kind).into(), epoch.0, *outcome, now),
            }
        }

        fn apply_unit(
            &mut self,
            kind: DataKind,
            unit: u64,
            outcome: UnitOutcome,
            now: DateTime<Utc>,
        ) {
            let UnitOutcome::Missing(not_served) = outcome else {
                self.units.remove(&(kind, unit));
                return;
            };
            let olds = self.states_in(kind, unit..unit + 1);
            let state = model_state(kind, &olds, not_served.into(), now);
            self.put((kind, unit), state, now);
        }

        fn apply_range(
            &mut self,
            kind: DataKind,
            range: Range<u64>,
            outcome: &RangeOutcome,
            now: DateTime<Utc>,
        ) {
            match outcome {
                RangeOutcome::Present => {
                    for unit in range {
                        self.units.remove(&(kind, unit));
                    }
                },
                RangeOutcome::Failed(failure) => {
                    let reason = MissingReason::Unconfirmed(Unconfirmed::Failed(*failure));
                    let states: Vec<_> = range
                        .clone()
                        .map(|unit| self.state((kind, unit)).cloned())
                        .collect();
                    // One unconfirmed state over the whole range backs off; confirmed 404 rows stay
                    // as they are; units without a row get a fresh one.
                    let backs_off = states[0].as_ref().is_some_and(|first| {
                        first.reason != MissingReason::NotFound
                            && states.iter().all(|s| s.as_ref() == Some(first))
                    });
                    for (unit, old) in range.zip(states) {
                        match old {
                            Some(old) if backs_off => {
                                let state = model_state(kind, &[old], reason, now);
                                self.put((kind, unit), state, now);
                            },
                            None => {
                                self.put((kind, unit), model_state(kind, &[], reason, now), now)
                            },
                            Some(_) => {},
                        }
                    }
                },
                RangeOutcome::Refined(search) => {
                    let gaps = search
                        .gaps
                        .iter()
                        .map(|gap| (gap.start.0..gap.end.0, MissingReason::NotFound));
                    let tail = search.tail.iter().map(|tail| {
                        (
                            tail.heights.start.0..tail.heights.end.0,
                            MissingReason::Unconfirmed(tail.reason),
                        )
                    });
                    let new: Vec<_> = gaps
                        .chain(tail)
                        .map(|(piece, reason)| {
                            let olds = self.states_in(kind, piece.clone());
                            (piece, model_state(kind, &olds, reason, now))
                        })
                        .collect();
                    for unit in range {
                        self.units.remove(&(kind, unit));
                    }
                    for (piece, state) in new {
                        for unit in piece {
                            self.put((kind, unit), state.clone(), now);
                        }
                    }
                },
            }
        }

        fn forget_below(&mut self, window_start: u64) {
            self.units.retain(|&(kind, unit), _| match kind.class() {
                KindClass::Range(_) => unit >= window_start,
                KindClass::Point(_) => unit + 1 > window_start,
                KindClass::Epoch(_) => model_root_block(kind, unit) >= window_start,
            });
        }

        fn runs(&self) -> Vec<Run> {
            let mut runs: Vec<Run> = Vec::new();
            for (&(kind, unit), (state, _)) in &self.units {
                let extends = matches!(kind.class(), KindClass::Range(_))
                    && runs.last().is_some_and(|run| {
                        run.kind == kind && run.units.end == unit && run.state == *state
                    });
                if extends {
                    runs.last_mut().unwrap().units.end = unit + 1;
                } else {
                    runs.push(Run {
                        kind,
                        units: unit..unit + 1,
                        state: state.clone(),
                    });
                }
            }
            runs
        }

        /// Whether `range` lies inside one 404 run of `kind`.
        fn is_known_gap(&self, kind: DataKind, range: &Range<u64>) -> bool {
            let first = self.state((kind, range.start));
            first.is_some_and(|first| {
                first.reason == MissingReason::NotFound
                    && range
                        .clone()
                        .all(|unit| self.state((kind, unit)) == Some(first))
            })
        }
    }

    fn model_alert_height(run: &Run) -> u64 {
        match run.kind.class() {
            KindClass::Range(_) => run.units.end,
            KindClass::Point(_) => run.units.start + 1,
            KindClass::Epoch(_) => model_root_block(run.kind, run.units.start),
        }
    }

    fn expand(set: &MissingSet) -> BTreeMap<Unit, RowState> {
        let mut units = BTreeMap::new();
        for row in set.rows() {
            for unit in row.key.units() {
                units.insert((row.key.kind(), unit), row.state.clone());
            }
        }
        units
    }

    /// The image row key.
    fn image_key(row: &MissingRow) -> (DataKind, u64) {
        (row.key.kind(), row.key.first_unit())
    }

    /// Builds the observation of an op, resolving `Retry` against `due`.
    fn observation_of(op: &Op, set: &MissingSet, now: DateTime<Utc>) -> Option<Observation> {
        Some(match op {
            Op::Range {
                kind,
                start,
                len,
                outcome,
            } => {
                let heights = h(*start..start + len);
                let outcome = match outcome {
                    RelOutcome::Present => RangeOutcome::Present,
                    RelOutcome::Failed(f) => RangeOutcome::Failed(*f),
                    RelOutcome::Refined { gaps, tail } => RangeOutcome::Refined(GapSearch {
                        gaps: gaps
                            .iter()
                            .map(|g| h(start + g.start..start + g.end))
                            .collect(),
                        tail: tail.map(|(offset, reason)| Tail {
                            heights: h(start + offset..start + len),
                            reason,
                        }),
                    }),
                };
                Observation::Range {
                    kind: *kind,
                    heights,
                    outcome,
                }
            },
            Op::Point {
                kind,
                height,
                outcome,
            } => point_obs(*kind, *height, *outcome),
            Op::Epoch {
                kind,
                epoch,
                outcome,
            } => epoch_obs(*kind, *epoch, *outcome),
            Op::Retry(failed) => {
                let probe = set.due(now, 20).into_iter().next()?;
                let unit = if *failed {
                    UnitOutcome::Missing(NotServed::Failed(Failure::Transport))
                } else {
                    UnitOutcome::Missing(NotServed::NotFound)
                };
                match probe {
                    Probe::Range { kind, heights } => {
                        let outcome = if *failed {
                            RangeOutcome::Failed(Failure::Transport)
                        } else {
                            RangeOutcome::Refined(GapSearch {
                                gaps: vec![heights.clone()],
                                tail: None,
                            })
                        };
                        Observation::Range {
                            kind,
                            heights,
                            outcome,
                        }
                    },
                    Probe::Point { kind, height } => Observation::Point {
                        kind,
                        height,
                        outcome: unit,
                    },
                    Probe::Epoch { kind, epoch } => Observation::Epoch {
                        kind,
                        epoch,
                        outcome: unit,
                    },
                }
            },
            Op::Forget(_) | Op::Advance(_) => return None,
        })
    }

    type Image = BTreeMap<(DataKind, u64), MissingRow>;

    fn check_units(set: &MissingSet, model: &Model) -> Result<(), TestCaseError> {
        let expected: BTreeMap<_, _> = model
            .units
            .iter()
            .map(|(unit, (state, _))| (*unit, state.clone()))
            .collect();
        prop_assert_eq!(expand(set), expected);
        Ok(())
    }

    /// Rows are sorted by kind, disjoint, and touching rows differ in state.
    fn check_row_order(set: &MissingSet) -> Result<(), TestCaseError> {
        let rows: Vec<_> = set.rows().collect();
        for pair in rows.windows(2) {
            prop_assert!(pair[0].key.kind() <= pair[1].key.kind());
            if let (
                RowKey::Range {
                    kind: ka,
                    heights: a,
                },
                RowKey::Range {
                    kind: kb,
                    heights: b,
                },
            ) = (&pair[0].key, &pair[1].key)
                && ka == kb
            {
                prop_assert!(a.end <= b.start);
                prop_assert!(a.end != b.start || pair[0].state != pair[1].state);
            }
        }
        Ok(())
    }

    /// Delays stay in 5 to 600 s, 60 s for confirmed rows; the retry time follows the write.
    fn check_state_rules(model: &Model) -> Result<(), TestCaseError> {
        for (&(kind, _), (state, written)) in &model.units {
            let delay = state.retry_delay.as_secs();
            prop_assert!((5..=600).contains(&delay));
            prop_assert!(state.failed_probes >= 1);
            prop_assert!(!model_confirmed(kind, state) || delay <= 60);
            prop_assert_eq!(state.next_retry_at, *written + seconds(delay));
        }
        Ok(())
    }

    fn check_summary(
        set: &MissingSet,
        model: &Model,
        now: DateTime<Utc>,
    ) -> Result<(), TestCaseError> {
        let runs = model.runs();
        for summary in set.summary(now) {
            let of_kind = || {
                runs.iter()
                    .filter(|run| run.kind == summary.kind && run.state.first_missing_at <= now)
            };
            let count: u64 = of_kind().map(|run| run.units.end - run.units.start).sum();
            prop_assert_eq!(summary.missing_count, count);
            let oldest = of_kind().min_by_key(|run| (run.state.first_missing_at, run.units.start));
            prop_assert_eq!(
                summary
                    .oldest
                    .map(|o| (o.key.first_unit(), o.age.as_secs() as i64)),
                oldest.map(|run| (
                    run.units.start,
                    (now - run.state.first_missing_at).num_seconds()
                ))
            );
        }
        Ok(())
    }

    /// The oldest firing run per kind alerts; nothing unconfirmed or of the aggregate does.
    fn check_alerts(
        set: &MissingSet,
        model: &Model,
        now: DateTime<Utc>,
    ) -> Result<(), TestCaseError> {
        let runs = model.runs();
        let candidates = set.alert_candidates(rule(), SETTLED_END, now);
        for &kind in DataKind::VARIANTS {
            let firing = runs
                .iter()
                .filter(|run| {
                    run.kind == kind
                        && model_confirmed(kind, &run.state)
                        && run.state.first_missing_at <= now - seconds(60)
                        && model_alert_height(run) <= SETTLED_END.0
                })
                .min_by_key(|run| (run.state.first_missing_at, run.units.start));
            let found = candidates.iter().find(|c| c.row.key.kind() == kind);
            prop_assert_eq!(
                found.map(|c| (c.row.key.units(), c.row.state.clone())),
                firing.map(|run| (run.units.clone(), run.state.clone()))
            );
        }
        for candidate in &candidates {
            prop_assert_eq!(candidate.row.state.reason, MissingReason::NotFound);
            prop_assert!(candidate.row.key.kind() != DataKind::Aggregate);
        }
        Ok(())
    }

    /// Due probes: due runs by retry time, kind and start, split at 100 heights, first 20.
    fn check_due(set: &MissingSet, model: &Model, now: DateTime<Utc>) -> Result<(), TestCaseError> {
        let runs = model.runs();
        let mut due: Vec<_> = runs
            .iter()
            .filter(|run| run.state.next_retry_at <= now)
            .collect();
        due.sort_by_key(|run| (run.state.next_retry_at, run.kind, run.units.start));
        let mut expected = Vec::new();
        for run in due {
            let first = run.units.start;
            match run.kind.class() {
                KindClass::Range(kind) => {
                    for start in (first..run.units.end).step_by(100) {
                        let end = run.units.end.min(start + 100);
                        expected.push(Probe::Range {
                            kind,
                            heights: h(start..end),
                        });
                    }
                },
                KindClass::Point(kind) => expected.push(Probe::Point {
                    kind,
                    height: Height(first),
                }),
                KindClass::Epoch(kind) => expected.push(Probe::Epoch {
                    kind,
                    epoch: Epoch(first),
                }),
            }
        }
        expected.truncate(20);
        prop_assert_eq!(set.due(now, 20), expected);
        Ok(())
    }

    /// `known` agrees with the model on the runs and on ranges just inside and past them.
    fn check_known(set: &MissingSet, model: &Model) -> Result<(), TestCaseError> {
        for run in model.runs() {
            let KindClass::Range(kind) = run.kind.class() else {
                continue;
            };
            let (start, end) = (run.units.start, run.units.end);
            for range in [start..end, start..start + 1, start..end + 1] {
                let expected = if model.is_known_gap(run.kind, &range) {
                    Known::Gap
                } else {
                    Known::Missing
                };
                prop_assert_eq!(set.known(kind, &h(range.clone())), expected, "{:?}", range);
            }
        }
        Ok(())
    }

    /// The listing is the oldest runs first (start, then kind), with the total.
    fn check_listed(
        set: &MissingSet,
        model: &Model,
        now: DateTime<Utc>,
    ) -> Result<(), TestCaseError> {
        let mut runs: Vec<_> = model
            .runs()
            .into_iter()
            .filter(|run| run.state.first_missing_at <= now)
            .collect();
        runs.sort_by_key(|run| (run.state.first_missing_at, run.units.start, run.kind));
        let expected: Vec<_> = runs
            .iter()
            .take(7)
            .map(|run| {
                (
                    run.kind,
                    run.units.clone(),
                    (now - run.state.first_missing_at).num_seconds(),
                )
            })
            .collect();
        let listed = set.listed(now, 7);
        let shown: Vec<_> = listed
            .shown
            .iter()
            .map(|aged| {
                (
                    aged.row.key.kind(),
                    aged.row.key.units(),
                    aged.age.as_secs() as i64,
                )
            })
            .collect();
        prop_assert_eq!(shown, expected);
        prop_assert_eq!(listed.total, runs.len() as u64);
        Ok(())
    }

    /// The image equals the set, and loading it gives the set back.
    fn check_image(set: &MissingSet, image: &Image) -> Result<(), TestCaseError> {
        let rows: Vec<_> = set.rows().collect();
        let image_rows: Vec<_> = image.values().cloned().collect();
        prop_assert_eq!(&image_rows, &rows);
        let loaded = MissingSet::from_rows(Some(EPOCH_CONFIG), image_rows)
            .expect("an image written from a set loads");
        prop_assert_eq!(loaded.rows().collect::<Vec<_>>(), rows);
        Ok(())
    }

    /// Applies the diff's regions to the image and checks each lies inside its region.
    fn apply_diff(
        set: &MissingSet,
        diff: &ImageDiff,
        image: &mut Image,
    ) -> Result<(), TestCaseError> {
        for region in &diff.regions {
            let (kind, units) = region.units();
            image.retain(|_, row| {
                let rows = row.key.units();
                !(row.key.kind() == kind && rows.start < units.end && rows.end > units.start)
            });
            for row in set.rows_in(region) {
                let rows = row.key.units();
                prop_assert!(
                    units.start <= rows.start && rows.end <= units.end,
                    "{row:?} outside {region:?}"
                );
                image.insert(image_key(&row), row);
            }
        }
        Ok(())
    }

    /// Runs `ops` against the set and the model, checking every invariant after each step.
    fn run_ops(ops: Vec<Op>) -> Result<(), TestCaseError> {
        let mut set = MissingSet::new(Some(EPOCH_CONFIG));
        let mut model = Model::default();
        let mut image = Image::new();
        let mut now = at(T0);
        for op in ops {
            let runs_before = model.runs().len();
            let diff = match &op {
                Op::Advance(secs) => {
                    now += seconds(*secs);
                    continue;
                },
                Op::Forget(window_start) => {
                    model.forget_below(*window_start);
                    set.apply(MissingUpdate::ForgetBelow(Height(*window_start)), now)
                },
                op => match observation_of(op, &set, now) {
                    Some(observation) => {
                        model.apply(&observation, now);
                        set.apply(MissingUpdate::Observed(observation), now)
                    },
                    None => continue,
                },
            };
            prop_assert_eq!(
                diff.rows_removed,
                runs_before.saturating_sub(model.runs().len())
            );
            apply_diff(&set, &diff, &mut image)?;
            check_units(&set, &model)?;
            check_row_order(&set)?;
            check_state_rules(&model)?;
            check_summary(&set, &model, now)?;
            check_alerts(&set, &model, now)?;
            check_due(&set, &model, now)?;
            check_known(&set, &model)?;
            check_listed(&set, &model, now)?;
            check_image(&set, &image)?;
        }
        Ok(())
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
}
