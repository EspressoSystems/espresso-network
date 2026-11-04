//! What a probe established: typed outcomes between the node's answers and the missing set.

use std::{ops::Range, str::FromStr};

use anyhow::Context;
use reqwest::StatusCode;

use crate::kind::{Epoch, EpochKind, Height, PointKind, RangeKind};

/// An HTTP status other than 2xx and 404; built only by `new`, so a 404 is never a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FailedStatus(StatusCode);

impl FailedStatus {
    /// `None` for 2xx and 404.
    pub fn new(status: StatusCode) -> Option<Self> {
        (!status.is_success() && status != StatusCode::NOT_FOUND).then_some(Self(status))
    }

    /// `"500"`, `"503"`, ...
    pub fn label(&self) -> &str {
        self.0.as_str()
    }
}

/// A request that got neither 2xx nor 404.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    Status(FailedStatus),
    Transport,
}

/// A non-2xx answer as the missing set reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotServed {
    NotFound,
    Failed(Failure),
}

impl From<NotServed> for Unconfirmed {
    /// A failure stays a failure; a 404 that reached a refinement request named no height.
    fn from(not_served: NotServed) -> Self {
        match not_served {
            NotServed::NotFound => Self::NoHeight,
            NotServed::Failed(failure) => Self::Failed(failure),
        }
    }
}

impl From<NotServed> for MissingReason {
    /// A 404 is `NotFound`; a failure is unconfirmed.
    fn from(not_served: NotServed) -> Self {
        match not_served {
            NotServed::NotFound => Self::NotFound,
            NotServed::Failed(failure) => Self::Unconfirmed(Unconfirmed::Failed(failure)),
        }
    }
}

/// Missing without a 404 that named the height: the count is an upper bound and never alerts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unconfirmed {
    /// A 404 body that names no height (`"no-height"`).
    NoHeight,
    /// A 404 body naming a height outside the range (`"out-of-range"`).
    OutOfRange,
    /// No 2xx or 404 (`"500"`, `"transport"`, ...).
    Failed(Failure),
}

/// Why a row is missing; persisted as `last_status`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingReason {
    /// A 404; for range kinds one that named the gap's first height. The only reason that alerts.
    NotFound,
    Unconfirmed(Unconfirmed),
}

impl MissingReason {
    /// `"404"`, `"no-height"`, `"out-of-range"`, the failed status (`StatusCode::as_str`), or
    /// `"transport"`.
    pub fn label(&self) -> &str {
        match self {
            Self::NotFound => "404",
            Self::Unconfirmed(Unconfirmed::NoHeight) => "no-height",
            Self::Unconfirmed(Unconfirmed::OutOfRange) => "out-of-range",
            Self::Unconfirmed(Unconfirmed::Failed(Failure::Status(status))) => status.label(),
            Self::Unconfirmed(Unconfirmed::Failed(Failure::Transport)) => "transport",
        }
    }
}

impl FromStr for MissingReason {
    type Err = anyhow::Error;

    /// Inverse of `label`; unknown labels are an error.
    fn from_str(label: &str) -> Result<Self, Self::Err> {
        let failed = |failure| Self::Unconfirmed(Unconfirmed::Failed(failure));
        Ok(match label {
            "404" => Self::NotFound,
            "no-height" => Self::Unconfirmed(Unconfirmed::NoHeight),
            "out-of-range" => Self::Unconfirmed(Unconfirmed::OutOfRange),
            "transport" => failed(Failure::Transport),
            _ => {
                let status = label
                    .parse::<u16>()
                    .ok()
                    .and_then(|code| StatusCode::from_u16(code).ok())
                    .and_then(FailedStatus::new)
                    .with_context(|| format!("unknown missing reason label {label:?}"))?;
                failed(Failure::Status(status))
            },
        })
    }
}

/// Whether every request of a probe or chunk, refinement included, got 200 or 404.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestHealth {
    Answered,
    Failed,
}

impl RequestHealth {
    /// `Failed` when either is.
    pub fn and(self, other: RequestHealth) -> RequestHealth {
        match (self, other) {
            (Self::Answered, Self::Answered) => Self::Answered,
            _ => Self::Failed,
        }
    }
}

/// Exact gaps of a range, ascending and disjoint, and the tail the search could not refine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GapSearch {
    pub gaps: Vec<Range<Height>>,
    pub tail: Option<Tail>,
}

/// The part of a range that could not be refined, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tail {
    pub heights: Range<Height>,
    pub reason: Unconfirmed,
}

/// What a range probe established.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RangeOutcome {
    /// 200: every height present.
    Present,
    /// The range HEAD failed with other than 404.
    Failed(Failure),
    /// The range HEAD said 404 and the search ran; no gaps and no tail means every height present.
    Refined(GapSearch),
}

/// What a point or epoch probe established.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitOutcome {
    Present,
    Missing(NotServed),
}

/// A probe and what it established; the input of the write path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Observation {
    Range {
        kind: RangeKind,
        heights: Range<Height>,
        outcome: RangeOutcome,
    },
    Point {
        kind: PointKind,
        height: Height,
        outcome: UnitOutcome,
    },
    Epoch {
        kind: EpochKind,
        epoch: Epoch,
        outcome: UnitOutcome,
    },
}

impl Observation {
    /// `Failed` when the probe or a refinement request got neither 200 nor 404.
    pub fn health(&self) -> RequestHealth {
        let failed = match self {
            Self::Range { outcome, .. } => match outcome {
                RangeOutcome::Present => false,
                RangeOutcome::Failed(_) => true,
                RangeOutcome::Refined(search) => search
                    .tail
                    .as_ref()
                    .is_some_and(|tail| matches!(tail.reason, Unconfirmed::Failed(_))),
            },
            Self::Point { outcome, .. } | Self::Epoch { outcome, .. } => {
                matches!(outcome, UnitOutcome::Missing(NotServed::Failed(_)))
            },
        };
        if failed {
            RequestHealth::Failed
        } else {
            RequestHealth::Answered
        }
    }
}

#[cfg(test)]
mod tests {
    use RequestHealth::{Answered, Failed};

    use super::*;

    fn all_reasons() -> Vec<MissingReason> {
        let status = |code: u16| {
            Unconfirmed::Failed(Failure::Status(
                FailedStatus::new(StatusCode::from_u16(code).unwrap()).unwrap(),
            ))
        };
        vec![
            MissingReason::NotFound,
            MissingReason::Unconfirmed(Unconfirmed::NoHeight),
            MissingReason::Unconfirmed(Unconfirmed::OutOfRange),
            MissingReason::Unconfirmed(Unconfirmed::Failed(Failure::Transport)),
            MissingReason::Unconfirmed(status(500)),
            MissingReason::Unconfirmed(status(503)),
        ]
    }

    #[test]
    fn failed_status_excludes_success_and_not_found() {
        for code in [200, 204, 404] {
            assert_eq!(FailedStatus::new(StatusCode::from_u16(code).unwrap()), None);
        }
        assert!(FailedStatus::new(StatusCode::INTERNAL_SERVER_ERROR).is_some());
    }

    #[test]
    fn reason_labels_roundtrip() {
        for reason in all_reasons() {
            assert_eq!(reason.label().parse::<MissingReason>().unwrap(), reason);
        }
        for label in ["404", "500", "transport", "no-height", "out-of-range"] {
            assert_eq!(label.parse::<MissingReason>().unwrap().label(), label);
        }
    }

    #[test]
    fn unknown_labels_are_errors() {
        for label in ["", "200", "oops", "99999"] {
            assert!(label.parse::<MissingReason>().is_err(), "{label}");
        }
    }

    #[test]
    fn request_health_and() {
        assert_eq!(Answered.and(Answered), Answered);
        assert_eq!(Answered.and(Failed), Failed);
        assert_eq!(Failed.and(Answered), Failed);
    }

    #[test]
    fn not_served_converts_to_reasons() {
        let failure = Failure::Transport;
        assert_eq!(
            MissingReason::from(NotServed::NotFound),
            MissingReason::NotFound
        );
        assert_eq!(
            MissingReason::from(NotServed::Failed(failure)),
            MissingReason::Unconfirmed(Unconfirmed::Failed(failure))
        );
        assert_eq!(
            Unconfirmed::from(NotServed::NotFound),
            Unconfirmed::NoHeight
        );
        assert_eq!(
            Unconfirmed::from(NotServed::Failed(failure)),
            Unconfirmed::Failed(failure)
        );
    }
}
