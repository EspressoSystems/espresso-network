//! The retry state of missing units and the one rule that changes it.

use std::{num::NonZeroU32, time::Duration};

use anyhow::ensure;
use chrono::{DateTime, TimeDelta, Utc};

use crate::outcome::{NotServed, Observation, RequestFailure};

/// 404s since the first, inclusive, before a not-found unit is confirmed: retried by the
/// representative and the recovery queue.
pub const CONFIRM_MIN_NOT_FOUND: u32 = 3;

/// Smallest retry delay in seconds.
const DELAY_MIN_SECS: u64 = 5;
/// Largest retry delay in seconds.
const DELAY_MAX_SECS: u64 = 600;
/// Retry delay cap of a not-found unit in seconds.
const NOT_FOUND_CAP_SECS: u16 = 60;

/// Upper bound of a doubling retry delay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RetryCap {
    /// 600 s, for a unit with no 404 since it went missing.
    Failed,
    /// 60 s, so every not-found unit is re-probed within a minute.
    NotFound,
}

impl RetryCap {
    /// The cap in seconds.
    fn secs(self) -> u16 {
        match self {
            Self::Failed => DELAY_MAX_SECS as u16,
            Self::NotFound => NOT_FOUND_CAP_SECS,
        }
    }
}

/// A retry delay in whole seconds within [5 s, 600 s].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RetryDelay(u16);

impl RetryDelay {
    /// 5 s, the delay of a fresh unit.
    pub const FIRST: RetryDelay = RetryDelay(DELAY_MIN_SECS as u16);

    /// Twice `self`, at most `cap`; a stored delay above `cap` comes down to it. The only backoff
    /// rule.
    pub fn doubled(self, cap: RetryCap) -> RetryDelay {
        RetryDelay(self.0.saturating_mul(2).min(cap.secs()))
    }

    /// A stored delay; errors outside [5, 600] s.
    pub fn from_secs(secs: u64) -> anyhow::Result<RetryDelay> {
        ensure!(
            (DELAY_MIN_SECS..=DELAY_MAX_SECS).contains(&secs),
            "retry delay {secs} s outside [{DELAY_MIN_SECS}, {DELAY_MAX_SECS}]"
        );
        Ok(RetryDelay(secs as u16))
    }

    /// The delay as a duration.
    pub fn duration(self) -> Duration {
        Duration::from_secs(self.0.into())
    }
}

/// When a missing unit is probed again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retry {
    pub delay: RetryDelay,
    pub next_at: DateTime<Utc>,
}

impl Retry {
    /// `delay` from `now`.
    pub fn after(delay: RetryDelay, now: DateTime<Utc>) -> Retry {
        let delta = TimeDelta::from_std(delay.duration()).expect("a delay fits a chrono delta");
        Retry {
            delay,
            next_at: now + delta,
        }
    }

    /// When the probe that set this retry ran: `next_at - delay`.
    pub fn last_tried_at(&self) -> DateTime<Utc> {
        self.next_at
            - TimeDelta::from_std(self.delay.duration()).expect("a delay fits a chrono delta")
    }
}

/// State shared by every unit of one missing row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitState {
    /// Missing with no 404 since it went missing; never counts.
    Failed {
        first_missing_at: DateTime<Utc>,
        reason: RequestFailure,
        failed_probes: NonZeroU32,
        retry: Retry,
    },
    /// A 404 answered for it; `not_found_probes` counts the 404s since, inclusive.
    NotFound {
        first_missing_at: DateTime<Utc>,
        not_found_probes: NonZeroU32,
        retry: Retry,
    },
}

impl UnitState {
    /// When the unit went missing in this state.
    pub fn first_missing_at(&self) -> DateTime<Utc> {
        match self {
            Self::Failed {
                first_missing_at, ..
            }
            | Self::NotFound {
                first_missing_at, ..
            } => *first_missing_at,
        }
    }

    /// The retry of either variant.
    pub fn retry(&self) -> Retry {
        match self {
            Self::Failed { retry, .. } | Self::NotFound { retry, .. } => *retry,
        }
    }

    /// `failed_probes` or `not_found_probes`: the `failed_probes` column.
    pub fn probe_count(&self) -> NonZeroU32 {
        match self {
            Self::Failed { failed_probes, .. } => *failed_probes,
            Self::NotFound {
                not_found_probes, ..
            } => *not_found_probes,
        }
    }

    /// `NotFound` with at least `CONFIRM_MIN_NOT_FOUND` 404s; the rule for the representative and
    /// the recovery queue.
    pub fn is_confirmed(&self) -> bool {
        match self {
            Self::Failed { .. } => false,
            Self::NotFound {
                not_found_probes, ..
            } => not_found_probes.get() >= CONFIRM_MIN_NOT_FOUND,
        }
    }

    /// `NotServed::NotFound` for `NotFound`, `Failed(reason)` otherwise; `missing_chunk.last_status`
    /// by its label.
    pub fn last_status(&self) -> NotServed {
        match self {
            Self::Failed { reason, .. } => NotServed::Failed(*reason),
            Self::NotFound { .. } => NotServed::NotFound,
        }
    }

    /// Inverse of `last_status`, `first_missing_at`, `probe_count` and `retry`; total, the label is
    /// parsed before.
    pub fn from_stored(
        last_status: NotServed,
        first_missing_at: DateTime<Utc>,
        probe_count: NonZeroU32,
        retry: Retry,
    ) -> UnitState {
        match last_status {
            NotServed::NotFound => Self::NotFound {
                first_missing_at,
                not_found_probes: probe_count,
                retry,
            },
            NotServed::Failed(reason) => Self::Failed {
                first_missing_at,
                reason,
                failed_probes: probe_count,
                retry,
            },
        }
    }

    /// `self` with its own delay doubled under the cap of its variant, counted from `now`.
    fn backed_off(self, now: DateTime<Utc>) -> UnitState {
        let cap = match self {
            Self::Failed { .. } => RetryCap::Failed,
            Self::NotFound { .. } => RetryCap::NotFound,
        };
        let retry = Retry::after(self.retry().delay.doubled(cap), now);
        match self {
            Self::Failed {
                first_missing_at,
                reason,
                failed_probes,
                ..
            } => Self::Failed {
                first_missing_at,
                reason,
                failed_probes,
                retry,
            },
            Self::NotFound {
                first_missing_at,
                not_found_probes,
                ..
            } => Self::NotFound {
                first_missing_at,
                not_found_probes,
                retry,
            },
        }
    }
}

/// The state of one unit after `observation` at `now`; `None`: present. One arm per cell of the
/// README table; the only rule that changes unit states.
pub fn transition(
    old: Option<UnitState>,
    observation: Observation,
    now: DateTime<Utc>,
) -> Option<UnitState> {
    let first = Retry::after(RetryDelay::FIRST, now);
    let one = NonZeroU32::MIN;
    let fresh_not_found = UnitState::NotFound {
        first_missing_at: now,
        not_found_probes: one,
        retry: first,
    };
    match (old, observation) {
        (None, Observation::Present)
        | (Some(UnitState::Failed { .. }), Observation::Present)
        | (Some(UnitState::NotFound { .. }), Observation::Present) => None,
        (None, Observation::NotFound) => Some(fresh_not_found),
        (None, Observation::Failed(reason)) => Some(UnitState::Failed {
            first_missing_at: now,
            reason,
            failed_probes: one,
            retry: first,
        }),
        (Some(UnitState::Failed { .. }), Observation::NotFound) => Some(fresh_not_found),
        (
            Some(UnitState::Failed {
                first_missing_at,
                failed_probes,
                retry,
                ..
            }),
            Observation::Failed(reason),
        ) => Some(
            UnitState::Failed {
                first_missing_at,
                reason,
                failed_probes: failed_probes.saturating_add(1),
                retry,
            }
            .backed_off(now),
        ),
        (
            Some(UnitState::NotFound {
                first_missing_at,
                not_found_probes,
                retry,
            }),
            Observation::NotFound,
        ) => Some(
            UnitState::NotFound {
                first_missing_at,
                not_found_probes: not_found_probes.saturating_add(1),
                retry,
            }
            .backed_off(now),
        ),
        (Some(old @ UnitState::NotFound { .. }), Observation::Failed(_reason)) => {
            Some(old.backed_off(now))
        },
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use reqwest::StatusCode;
    use strum::{EnumIter, IntoEnumIterator};

    use super::*;
    use crate::outcome::FailedStatus;

    const NOW: i64 = 1_000_000;

    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).unwrap()
    }

    fn status(code: u16) -> RequestFailure {
        RequestFailure::Status(FailedStatus::new(StatusCode::from_u16(code).unwrap()).unwrap())
    }

    fn retry(delay: u64) -> Retry {
        Retry::after(RetryDelay::from_secs(delay).unwrap(), at(NOW))
    }

    /// `N(f, n, d)` with `next_at = NOW + d`.
    fn n(first: i64, probes: u32, delay: u64) -> UnitState {
        UnitState::NotFound {
            first_missing_at: at(first),
            not_found_probes: NonZeroU32::new(probes).unwrap(),
            retry: retry(delay),
        }
    }

    /// `F(f, r, k, d)` with `next_at = NOW + d`.
    fn f(first: i64, reason: RequestFailure, probes: u32, delay: u64) -> UnitState {
        UnitState::Failed {
            first_missing_at: at(first),
            reason,
            failed_probes: NonZeroU32::new(probes).unwrap(),
            retry: retry(delay),
        }
    }

    /// An old state with a retry from before `NOW`; the transition replaces it.
    fn old(state: UnitState) -> UnitState {
        match state {
            UnitState::Failed {
                first_missing_at,
                reason,
                failed_probes,
                retry,
            } => UnitState::Failed {
                first_missing_at,
                reason,
                failed_probes,
                retry: Retry {
                    next_at: at(NOW - 1),
                    ..retry
                },
            },
            UnitState::NotFound {
                first_missing_at,
                not_found_probes,
                retry,
            } => UnitState::NotFound {
                first_missing_at,
                not_found_probes,
                retry: Retry {
                    next_at: at(NOW - 1),
                    ..retry
                },
            },
        }
    }

    const T: i64 = NOW;
    const OLD: i64 = NOW - 100;

    /// Cells written from the README table; the `Failed` and `NotFound` columns of the old state
    /// are given as the state before (with its delay) and the state after.
    fn cells() -> Vec<(Option<UnitState>, Observation, Option<UnitState>)> {
        use Observation::{Failed as Fail, NotFound as Nf, Present};
        let transport = RequestFailure::Transport;
        let s500 = status(500);
        vec![
            (None, Present, None),
            (None, Nf, Some(n(T, 1, 5))),
            (None, Fail(s500), Some(f(T, s500, 1, 5))),
            (None, Fail(transport), Some(f(T, transport, 1, 5))),
            (Some(f(OLD, s500, 1, 5)), Present, None),
            (Some(f(OLD, s500, 1, 5)), Nf, Some(n(T, 1, 5))),
            (
                Some(f(OLD, s500, 1, 5)),
                Fail(s500),
                Some(f(OLD, s500, 2, 10)),
            ),
            (
                Some(f(OLD, s500, 1, 5)),
                Fail(transport),
                Some(f(OLD, transport, 2, 10)),
            ),
            (
                Some(f(OLD, s500, 7, 320)),
                Fail(transport),
                Some(f(OLD, transport, 8, 600)),
            ),
            (
                Some(f(OLD, s500, 9, 600)),
                Fail(s500),
                Some(f(OLD, s500, 10, 600)),
            ),
            (Some(n(OLD, 1, 5)), Present, None),
            (Some(n(OLD, 4, 60)), Present, None),
            (Some(n(OLD, 1, 5)), Nf, Some(n(OLD, 2, 10))),
            (Some(n(OLD, 2, 10)), Nf, Some(n(OLD, 3, 20))),
            (Some(n(OLD, 3, 40)), Nf, Some(n(OLD, 4, 60))),
            (Some(n(OLD, 2, 40)), Nf, Some(n(OLD, 3, 60))),
            (Some(n(OLD, 5, 600)), Nf, Some(n(OLD, 6, 60))),
            (Some(n(OLD, 4, 60)), Fail(s500), Some(n(OLD, 4, 60))),
            (Some(n(OLD, 4, 60)), Fail(transport), Some(n(OLD, 4, 60))),
            (Some(n(OLD, 2, 10)), Fail(s500), Some(n(OLD, 2, 20))),
            (Some(n(OLD, 2, 10)), Fail(transport), Some(n(OLD, 2, 20))),
            (Some(n(OLD, 2, 400)), Fail(s500), Some(n(OLD, 2, 60))),
        ]
    }

    #[test]
    fn state_cell_table() {
        for (before, observation, expected) in cells() {
            let before = before.map(old);
            assert_eq!(
                transition(before, observation, at(NOW)),
                expected,
                "{before:?} x {observation:?}"
            );
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
    enum StateShape {
        Absent,
        Failed,
        NotFoundBelowMin,
        Confirmed,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, EnumIter)]
    enum ObservationShape {
        Present,
        NotFound,
        FailedStatus,
        Transport,
    }

    fn state_shape(state: &Option<UnitState>) -> StateShape {
        match state {
            None => StateShape::Absent,
            Some(UnitState::Failed { .. }) => StateShape::Failed,
            Some(UnitState::NotFound {
                not_found_probes, ..
            }) if not_found_probes.get() >= 3 => StateShape::Confirmed,
            Some(UnitState::NotFound { .. }) => StateShape::NotFoundBelowMin,
        }
    }

    fn observation_shape(observation: Observation) -> ObservationShape {
        match observation {
            Observation::Present => ObservationShape::Present,
            Observation::NotFound => ObservationShape::NotFound,
            Observation::Failed(RequestFailure::Status(_)) => ObservationShape::FailedStatus,
            Observation::Failed(RequestFailure::Transport) => ObservationShape::Transport,
        }
    }

    #[test]
    fn the_cell_table_covers_every_pair() {
        let table = cells();
        for state in StateShape::iter() {
            for observation in ObservationShape::iter() {
                assert!(
                    table.iter().any(|(before, obs, _)| {
                        state_shape(before) == state && observation_shape(*obs) == observation
                    }),
                    "no row for {state:?} x {observation:?}"
                );
            }
        }
    }

    #[test]
    fn confirmation_follows_the_state() {
        assert!(!f(OLD, status(500), 99, 600).is_confirmed());
        assert!(!n(OLD, 2, 10).is_confirmed());
        assert!(n(OLD, 3, 10).is_confirmed());
        assert!(n(OLD, 4, 10).is_confirmed());
    }

    #[test]
    fn delays_double_under_both_caps() {
        let secs = |s| RetryDelay::from_secs(s).unwrap();
        let cases = [
            (5, RetryCap::Failed, 10),
            (320, RetryCap::Failed, 600),
            (600, RetryCap::Failed, 600),
            (5, RetryCap::NotFound, 10),
            (30, RetryCap::NotFound, 60),
            (60, RetryCap::NotFound, 60),
            (600, RetryCap::NotFound, 60),
        ];
        for (delay, cap, expected) in cases {
            assert_eq!(secs(delay).doubled(cap), secs(expected), "{delay} {cap:?}");
        }
        assert!(RetryDelay::from_secs(4).is_err());
        assert!(RetryDelay::from_secs(601).is_err());
    }

    #[test]
    fn stored_state_roundtrips() {
        let states = [
            f(OLD, status(500), 3, 20),
            f(OLD, RequestFailure::Transport, 1, 5),
            n(OLD, 1, 5),
            n(OLD, 7, 60),
        ];
        for state in states {
            let back = UnitState::from_stored(
                state.last_status(),
                state.first_missing_at(),
                state.probe_count(),
                state.retry(),
            );
            assert_eq!(back, state);
        }
    }

    #[test]
    fn counts_saturate() {
        let max = |state| match state {
            UnitState::Failed {
                first_missing_at,
                reason,
                retry,
                ..
            } => UnitState::Failed {
                first_missing_at,
                reason,
                failed_probes: NonZeroU32::MAX,
                retry,
            },
            UnitState::NotFound {
                first_missing_at,
                retry,
                ..
            } => UnitState::NotFound {
                first_missing_at,
                not_found_probes: NonZeroU32::MAX,
                retry,
            },
        };
        let s500 = status(500);
        let failed = transition(
            Some(max(f(OLD, s500, 1, 5))),
            Observation::Failed(s500),
            at(NOW),
        );
        assert_eq!(failed.unwrap().probe_count(), NonZeroU32::MAX);
        let not_found = transition(Some(max(n(OLD, 1, 5))), Observation::NotFound, at(NOW));
        assert_eq!(not_found.unwrap().probe_count(), NonZeroU32::MAX);
    }

    #[test]
    fn a_stored_delay_above_the_cap_comes_down() {
        let stored = Some(old(n(OLD, 3, 600)));
        let after = transition(stored, Observation::Failed(status(500)), at(NOW)).unwrap();
        assert_eq!(after.retry().delay, RetryDelay::from_secs(60).unwrap());
    }

    #[test]
    fn failures_after_one_404_stay_within_a_minute() {
        let mut state = transition(None, Observation::NotFound, at(NOW));
        for _ in 0..10 {
            state = transition(state, Observation::Failed(status(500)), at(NOW));
            let delay = state.unwrap().retry().delay;
            assert!(delay <= RetryDelay::from_secs(60).unwrap(), "{delay:?}");
        }
    }
}
