//! Exact gap search inside a height range that failed a range probe, with HEADs only.

use std::ops::Range;

use crate::{
    kind::Height,
    outcome::{Observation, RangeFindings, RequestFailure},
};

/// Which heights a node serves.
pub trait Presence {
    /// HEAD of the range route over `heights` (non-empty): `true` on 200, `false` on any 404.
    async fn all_present(&mut self, heights: Range<Height>) -> Result<bool, RequestFailure>;
}

/// What is known about a range before it is searched; its range probe failed with 404.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    /// Some height of the range is missing.
    Missing,
    /// The range was an exact gap on the last search: its first height was missing.
    Gap,
}

/// Gaps of `heights`, whose range HEAD answered 404: `first_missing`, then `gap_end`, repeated
/// after each gap from a HEAD of the rest. Gaps are marked `NotFound`; after a failure the rest is
/// marked `Failed(failure)`; every mark lies inside `heights`. `Known::Gap` keeps HEAD's two-end
/// check.
pub async fn find_gaps<P: Presence>(
    presence: &mut P,
    heights: Range<Height>,
    known: Known,
) -> RangeFindings {
    let mut findings = RangeFindings::new(heights.clone());
    if known == Known::Gap {
        let still_missing = match heights.end.0 - heights.start.0 {
            1 => Ok(true),
            _ => gap_still_missing(presence, &heights).await,
        };
        match still_missing {
            Ok(true) => return RangeFindings::uniform(heights, Observation::NotFound),
            Ok(false) => {},
            Err(failure) => return RangeFindings::uniform(heights, Observation::Failed(failure)),
        }
    }
    let mut cursor = heights.start;
    let mut rest_known_missing = true;
    while cursor < heights.end {
        let rest = cursor..heights.end;
        match next_gap(presence, &rest, rest_known_missing).await {
            Ok(None) => break,
            Ok(Some(gap)) => {
                cursor = gap.end;
                findings.mark(gap, Observation::NotFound);
            },
            Err(failure) => {
                findings.mark(rest, Observation::Failed(failure));
                break;
            },
        }
        rest_known_missing = false;
    }
    findings
}

/// Whether the first and the last height of a known gap are both still missing, which keeps the
/// whole row; a present first height is checked first, so a refilled front costs one request.
async fn gap_still_missing<P: Presence>(
    presence: &mut P,
    heights: &Range<Height>,
) -> Result<bool, RequestFailure> {
    let last = Height(heights.end.0 - 1);
    Ok(!single(presence, heights.start).await? && !single(presence, last).await?)
}

/// The first gap in `rest`, `None` when every height is present. A HEAD of `rest` comes first
/// unless it is already known to be 404.
async fn next_gap<P: Presence>(
    presence: &mut P,
    rest: &Range<Height>,
    known_missing: bool,
) -> Result<Option<Range<Height>>, RequestFailure> {
    if !known_missing && presence.all_present(rest.clone()).await? {
        return Ok(None);
    }
    let first = first_missing(presence, rest).await?;
    let end = gap_end(presence, first, rest.end).await?;
    Ok(Some(first..end))
}

/// First missing height of `heights`, given that their HEAD answered 404: binary search over
/// prefixes `[heights.start, mid)`, at most ceil(log2(len)) HEADs (7 for 100 heights). `lo` is the
/// end of a prefix known present, `hi` the end of a prefix known to hold a 404.
async fn first_missing<P: Presence>(
    presence: &mut P,
    heights: &Range<Height>,
) -> Result<Height, RequestFailure> {
    let (mut lo, mut hi) = (heights.start.0, heights.end.0);
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if presence.all_present(heights.start..Height(mid)).await? {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    Ok(Height(lo))
}

/// End of the gap that starts at the missing height `first`, below `end`: gallop, then binary
/// search over single heights.
async fn gap_end<P: Presence>(
    presence: &mut P,
    first: Height,
    end: Height,
) -> Result<Height, RequestFailure> {
    let (mut missing, mut present) = (first.0, end.0);
    let mut step = 1u64;
    while let Some(probe) = first.0.checked_add(step).filter(|probe| *probe < end.0) {
        if single(presence, Height(probe)).await? {
            present = probe;
            break;
        }
        missing = probe;
        step = step.saturating_mul(2);
    }
    while present - missing > 1 {
        let middle = missing + (present - missing) / 2;
        if single(presence, Height(middle)).await? {
            present = middle;
        } else {
            missing = middle;
        }
    }
    Ok(Height(present))
}

/// HEAD of one height.
async fn single<P: Presence>(presence: &mut P, height: Height) -> Result<bool, RequestFailure> {
    presence.all_present(height..height.next()).await
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use pretty_assertions::assert_eq;

    use super::*;

    #[derive(Default)]
    struct Fake {
        missing: BTreeSet<u64>,
        /// The HEAD with this 1-based sequence number fails.
        fail_on: Option<usize>,
        /// Every requested range, in order.
        requests: Vec<Range<Height>>,
        /// Every height answers present: the gap was refilled after the range HEAD.
        refilled: bool,
    }

    impl Fake {
        fn new(missing: impl IntoIterator<Item = u64>) -> Self {
            Self {
                missing: missing.into_iter().collect(),
                ..Self::default()
            }
        }
    }

    const FAILURE: RequestFailure = RequestFailure::Transport;

    impl Presence for Fake {
        async fn all_present(&mut self, heights: Range<Height>) -> Result<bool, RequestFailure> {
            self.requests.push(heights.clone());
            if self.fail_on == Some(self.requests.len()) {
                return Err(FAILURE);
            }
            Ok(self.refilled
                || self
                    .missing
                    .range(heights.start.0..heights.end.0)
                    .next()
                    .is_none())
        }
    }

    fn h(range: Range<u64>) -> Range<Height> {
        Height(range.start)..Height(range.end)
    }

    fn findings(heights: Range<u64>, marks: &[(Range<u64>, Observation)]) -> RangeFindings {
        let mut findings = RangeFindings::new(h(heights));
        for (piece, observation) in marks {
            findings.mark(h(piece.clone()), *observation);
        }
        findings
    }

    fn gap(range: Range<u64>) -> (Range<u64>, Observation) {
        (range, Observation::NotFound)
    }

    #[tokio::test]
    async fn find_gaps_table() {
        let cases = [
            ("contiguous gap", vec![3, 4, 5, 6], vec![gap(3..7)]),
            ("gap at start", vec![0, 1, 2, 3], vec![gap(0..4)]),
            ("gap at end", vec![6, 7, 8, 9], vec![gap(6..10)]),
            ("single height", vec![5], vec![gap(5..6)]),
            ("whole chunk", (0..10).collect(), vec![gap(0..10)]),
            ("two gaps", vec![2, 3, 7, 8], vec![gap(2..4), gap(7..9)]),
        ];
        for (name, missing, marks) in cases {
            let mut fake = Fake::new(missing);
            let found = find_gaps(&mut fake, h(0..10), Known::Missing).await;
            assert_eq!(found, findings(0..10, &marks), "{name}");
        }
    }

    #[tokio::test]
    async fn find_gaps_long_gap_and_offset_range() {
        let mut fake = Fake::new(1_037..1_900);
        let found = find_gaps(&mut fake, h(1_000..2_000), Known::Missing).await;
        assert_eq!(found, findings(1_000..2_000, &[gap(1_037..1_900)]));
    }

    #[tokio::test]
    async fn only_the_first_range_probe_is_known_missing() {
        let mut fake = Fake::new([2, 3, 7, 8]);
        find_gaps(&mut fake, h(0..10), Known::Missing).await;
        assert!(!fake.requests.contains(&h(0..10)));
    }

    #[tokio::test]
    async fn gaps_bisection_requests() {
        let mut fake = Fake::new(37..41);
        let found = find_gaps(&mut fake, h(0..100), Known::Missing).await;
        let expected = [
            0..50,
            0..25,
            0..37,
            0..43,
            0..40,
            0..38,
            38..39,
            39..40,
            41..42,
            40..41,
            41..100,
        ];
        assert_eq!(
            fake.requests,
            expected.into_iter().map(h).collect::<Vec<_>>()
        );
        assert_eq!(found, findings(0..100, &[gap(37..41)]));
    }

    #[tokio::test]
    async fn gaps_refill_race() {
        let mut fake = Fake::new([5]);
        fake.refilled = true;
        let found = find_gaps(&mut fake, h(0..100), Known::Missing).await;
        assert_eq!(found, findings(0..100, &[gap(99..100)]));
        assert_eq!(fake.requests.len(), 7);
    }

    #[tokio::test]
    async fn find_gaps_keeps_found_gaps_when_a_later_probe_fails() {
        let mut fake = Fake::new([1, 2, 7, 8]);
        // Requests 1 to 3 find height 1, 4 and 5 end the gap at 3, 6 is the HEAD of the rest, 7
        // is its first bisection step.
        fake.fail_on = Some(7);
        let found = find_gaps(&mut fake, h(0..10), Known::Missing).await;
        let failed = Observation::Failed(FAILURE);
        let pieces: Vec<_> = found.pieces().collect();
        assert_eq!(pieces[0], (h(0..1), Observation::Present));
        assert_eq!(pieces[1], (h(1..3), Observation::NotFound));
        assert_eq!(pieces.last().unwrap().1, failed);
    }

    #[tokio::test]
    async fn gaps_failure_in_bisection() {
        let mut fake = Fake::new([5]);
        fake.fail_on = Some(2);
        let found = find_gaps(&mut fake, h(0..100), Known::Missing).await;
        assert_eq!(
            found,
            RangeFindings::uniform(h(0..100), Observation::Failed(FAILURE))
        );

        let mut fake = Fake::new(10..20);
        // The last request of a full run is the HEAD of the rest after the gap; it fails here.
        let mut probe = Fake::new(10..20);
        find_gaps(&mut probe, h(0..100), Known::Missing).await;
        fake.fail_on = Some(probe.requests.len());
        let found = find_gaps(&mut fake, h(0..100), Known::Missing).await;
        let mut expected = RangeFindings::new(h(0..100));
        expected.mark(h(10..20), Observation::NotFound);
        expected.mark(h(20..100), Observation::Failed(FAILURE));
        assert_eq!(found, expected);
    }

    #[tokio::test]
    async fn find_gaps_keeps_the_range_when_a_probe_fails_at_once() {
        let mut fake = Fake::new([3, 4, 5]);
        fake.fail_on = Some(1);
        let found = find_gaps(&mut fake, h(0..10), Known::Missing).await;
        assert_eq!(
            found,
            RangeFindings::uniform(h(0..10), Observation::Failed(FAILURE))
        );
    }

    #[tokio::test]
    async fn known_gap_costs_two_probes_while_both_ends_are_missing() {
        let mut fake = Fake::new([3, 4, 5, 6]);
        let found = find_gaps(&mut fake, h(3..7), Known::Gap).await;
        assert_eq!(found, findings(3..7, &[gap(3..7)]));
        assert_eq!(fake.requests, vec![h(3..4), h(6..7)]);
    }

    #[tokio::test]
    async fn known_gap_with_a_refilled_end_is_searched_again() {
        let mut front = Fake::new([4, 5, 6]);
        let found = find_gaps(&mut front, h(3..7), Known::Gap).await;
        assert_eq!(found, findings(3..7, &[gap(4..7)]));

        let mut back = Fake::new([3, 4, 5]);
        let found = find_gaps(&mut back, h(3..7), Known::Gap).await;
        assert_eq!(found, findings(3..7, &[gap(3..6)]));
    }

    #[tokio::test]
    async fn known_one_height_gap_needs_no_probe() {
        let mut fake = Fake::new([5]);
        let found = find_gaps(&mut fake, h(5..6), Known::Gap).await;
        assert_eq!(found, findings(5..6, &[gap(5..6)]));
        assert!(fake.requests.is_empty());
    }
}
