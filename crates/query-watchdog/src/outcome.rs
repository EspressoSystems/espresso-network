//! What a probe established: typed observations between the node's answers and the missing set.

use std::{fmt, ops::Range, str::FromStr};

use anyhow::Context;
use reqwest::StatusCode;

use crate::kind::{Epoch, EpochKind, Height, PointKind, RangeKind};

/// An HTTP status other than 2xx and 404; built only by `new`, so a 404 is never a failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

impl fmt::Display for FailedStatus {
    /// The status as `StatusCode` prints it, `500 Internal Server Error`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A request that got neither 2xx nor 404; the stored reason of a failed unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestFailure {
    Status(FailedStatus),
    Transport,
}

impl RequestFailure {
    /// The failed status (`"500"`, ...) or `"transport"`.
    pub fn label(&self) -> &str {
        match self {
            Self::Status(status) => status.label(),
            Self::Transport => "transport",
        }
    }
}

/// A request without 2xx as the counters and the image read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotServed {
    NotFound,
    Failed(RequestFailure),
}

impl NotServed {
    /// `"404"` or the failure's label: the `status` label of `fetch_errors_total`, the `errors`
    /// keys and `missing_chunk.last_status`.
    pub fn label(&self) -> &str {
        match self {
            Self::NotFound => "404",
            Self::Failed(failure) => failure.label(),
        }
    }
}

impl FromStr for NotServed {
    type Err = anyhow::Error;

    /// Inverse of `label`; 2xx and unknown labels are errors.
    fn from_str(label: &str) -> Result<Self, Self::Err> {
        Ok(match label {
            "404" => Self::NotFound,
            "transport" => Self::Failed(RequestFailure::Transport),
            _ => {
                let status = label
                    .parse::<u16>()
                    .ok()
                    .and_then(|code| StatusCode::from_u16(code).ok())
                    .and_then(FailedStatus::new)
                    .with_context(|| format!("unknown not-served label {label:?}"))?;
                Self::Failed(RequestFailure::Status(status))
            },
        })
    }
}

/// Whether every request of a probe or chunk, search included, got 200 or 404.
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

/// What one probe established about one unit; every kind decides on the HTTP status alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    Present,
    NotFound,
    Failed(RequestFailure),
}

impl From<NotServed> for Observation {
    /// `NotFound` or `Failed`.
    fn from(not_served: NotServed) -> Self {
        match not_served {
            NotServed::NotFound => Self::NotFound,
            NotServed::Failed(failure) => Self::Failed(failure),
        }
    }
}

impl Observation {
    /// `Failed` for `Failed(_)`.
    pub fn health(&self) -> RequestHealth {
        match self {
            Self::Failed(_) => RequestHealth::Failed,
            Self::Present | Self::NotFound => RequestHealth::Answered,
        }
    }
}

/// Observations of one probed range, piece by piece; a partition of `heights` by construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeFindings {
    heights: Range<Height>,
    /// Ascending, disjoint, inside `heights`; heights not covered are `Present`.
    marks: Vec<(Range<Height>, Observation)>,
}

impl RangeFindings {
    /// Every height `Present` until marked.
    pub fn new(heights: Range<Height>) -> Self {
        Self {
            heights,
            marks: Vec::new(),
        }
    }

    /// Every height observed as `observation` (a range HEAD that answered 200 or failed).
    pub fn uniform(heights: Range<Height>, observation: Observation) -> Self {
        let mut findings = Self::new(heights.clone());
        if observation != Observation::Present {
            findings.mark(heights, observation);
        }
        findings
    }

    /// Marks `piece`; panics unless it is non-empty, inside `heights` and above every earlier mark.
    pub fn mark(&mut self, piece: Range<Height>, observation: Observation) {
        let floor = self.marks.last().map_or(self.heights.start, |(m, _)| m.end);
        assert!(
            piece.start < piece.end && piece.start >= floor && piece.end <= self.heights.end,
            "mark {piece:?} outside {:?} above {floor:?}",
            self.heights
        );
        self.marks.push((piece, observation));
    }

    /// The probed range.
    pub fn heights(&self) -> &Range<Height> {
        &self.heights
    }

    /// Every height once, ascending: the marks and the `Present` pieces between them.
    pub fn pieces(&self) -> impl Iterator<Item = (Range<Height>, Observation)> + '_ {
        let mut pieces = Vec::with_capacity(self.marks.len() * 2 + 1);
        let mut cursor = self.heights.start;
        for (piece, observation) in &self.marks {
            if cursor < piece.start {
                pieces.push((cursor..piece.start, Observation::Present));
            }
            pieces.push((piece.clone(), *observation));
            cursor = piece.end;
        }
        if cursor < self.heights.end {
            pieces.push((cursor..self.heights.end, Observation::Present));
        }
        pieces.into_iter()
    }

    /// `Failed` when any piece is `Failed(_)`.
    pub fn health(&self) -> RequestHealth {
        self.marks
            .iter()
            .fold(RequestHealth::Answered, |health, (_, observation)| {
                health.and(observation.health())
            })
    }
}

/// A probe and what it established; the input of the write path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProbeResult {
    Range {
        kind: RangeKind,
        findings: RangeFindings,
    },
    Point {
        kind: PointKind,
        height: Height,
        observation: Observation,
    },
    Epoch {
        kind: EpochKind,
        epoch: Epoch,
        observation: Observation,
    },
}

impl ProbeResult {
    /// The probe's health: the findings' or the observation's.
    pub fn health(&self) -> RequestHealth {
        match self {
            Self::Range { findings, .. } => findings.health(),
            Self::Point { observation, .. } | Self::Epoch { observation, .. } => {
                observation.health()
            },
        }
    }

    /// The health that may pause the sweep: epoch probes never do, since old-epoch objects answer
    /// 500 while the node catches up.
    pub fn gate_health(&self) -> RequestHealth {
        match self {
            Self::Range { .. } | Self::Point { .. } => self.health(),
            Self::Epoch { .. } => RequestHealth::Answered,
        }
    }

    /// Some unit observed `Present`; a representative result that is starts recovery.
    pub fn found_present(&self) -> bool {
        match self {
            Self::Range { findings, .. } => findings
                .pieces()
                .any(|(_, observation)| observation == Observation::Present),
            Self::Point { observation, .. } | Self::Epoch { observation, .. } => {
                *observation == Observation::Present
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use RequestHealth::{Answered, Failed};

    use super::*;

    fn h(range: Range<u64>) -> Range<Height> {
        Height(range.start)..Height(range.end)
    }

    fn status(code: u16) -> RequestFailure {
        RequestFailure::Status(FailedStatus::new(StatusCode::from_u16(code).unwrap()).unwrap())
    }

    #[test]
    fn failed_status_excludes_success_and_not_found() {
        for code in [200, 204, 404] {
            assert_eq!(FailedStatus::new(StatusCode::from_u16(code).unwrap()), None);
        }
        assert!(FailedStatus::new(StatusCode::INTERNAL_SERVER_ERROR).is_some());
    }

    #[test]
    fn not_served_labels_roundtrip() {
        for label in ["404", "500", "503", "transport"] {
            assert_eq!(label.parse::<NotServed>().unwrap().label(), label);
        }
        for label in ["", "200", "no-height", "out-of-range", "oops", "99999"] {
            assert!(label.parse::<NotServed>().is_err(), "{label}");
        }
    }

    #[test]
    fn request_health_and() {
        assert_eq!(Answered.and(Answered), Answered);
        assert_eq!(Answered.and(Failed), Failed);
        assert_eq!(Failed.and(Answered), Failed);
    }

    #[test]
    fn observation_health() {
        assert_eq!(Observation::Present.health(), Answered);
        assert_eq!(Observation::NotFound.health(), Answered);
        assert_eq!(Observation::Failed(status(500)).health(), Failed);
    }

    #[test]
    fn findings_pieces_partition_the_range() {
        use Observation::{NotFound, Present};
        let marked = |marks: &[(Range<u64>, Observation)]| {
            let mut findings = RangeFindings::new(h(0..10));
            for (piece, observation) in marks {
                findings.mark(h(piece.clone()), *observation);
            }
            findings
        };
        let cases = [
            (marked(&[]), vec![(h(0..10), Present)]),
            (
                RangeFindings::uniform(h(0..10), NotFound),
                vec![(h(0..10), NotFound)],
            ),
            (
                RangeFindings::uniform(h(0..10), Present),
                vec![(h(0..10), Present)],
            ),
            (
                marked(&[(3..5, NotFound)]),
                vec![(h(0..3), Present), (h(3..5), NotFound), (h(5..10), Present)],
            ),
            (
                marked(&[(0..2, NotFound), (8..10, NotFound)]),
                vec![
                    (h(0..2), NotFound),
                    (h(2..8), Present),
                    (h(8..10), NotFound),
                ],
            ),
            (
                marked(&[(0..4, NotFound), (4..10, Observation::Failed(status(500)))]),
                vec![
                    (h(0..4), NotFound),
                    (h(4..10), Observation::Failed(status(500))),
                ],
            ),
        ];
        for (findings, expected) in cases {
            assert_eq!(findings.pieces().collect::<Vec<_>>(), expected);
        }
    }

    #[test]
    fn findings_health_follows_the_marks() {
        let mut findings = RangeFindings::new(h(0..10));
        findings.mark(h(2..4), Observation::NotFound);
        assert_eq!(findings.health(), Answered);
        findings.mark(h(4..6), Observation::Failed(RequestFailure::Transport));
        assert_eq!(findings.health(), Failed);
    }

    #[test]
    #[should_panic]
    fn mark_past_the_end_panics() {
        RangeFindings::new(h(0..10)).mark(h(5..11), Observation::NotFound);
    }

    #[test]
    #[should_panic]
    fn mark_below_the_start_panics() {
        RangeFindings::new(h(5..10)).mark(h(4..6), Observation::NotFound);
    }

    #[test]
    #[should_panic]
    fn mark_below_the_previous_mark_panics() {
        let mut findings = RangeFindings::new(h(0..10));
        findings.mark(h(5..8), Observation::NotFound);
        findings.mark(h(2..4), Observation::NotFound);
    }

    #[test]
    fn found_present_reads_pieces() {
        let result = |findings| ProbeResult::Range {
            kind: RangeKind::Leaf,
            findings,
        };
        let mut partial = RangeFindings::new(h(0..10));
        partial.mark(h(0..5), Observation::NotFound);
        assert!(result(partial).found_present());
        assert!(!result(RangeFindings::uniform(h(0..10), Observation::NotFound)).found_present());
    }
}
