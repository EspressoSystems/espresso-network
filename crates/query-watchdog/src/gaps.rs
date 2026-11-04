//! Exact gap search inside a height range that failed a range probe.

use std::ops::Range;

use crate::{
    kind::Height,
    outcome::{GapSearch, Tail, Unconfirmed},
};

/// Which heights a node serves.
pub trait Presence {
    /// First missing height in `heights`; `None` when all are present. With `known_missing`, the
    /// range HEAD already said 404, so the height is read without probing again.
    async fn first_missing(
        &mut self,
        heights: Range<Height>,
        known_missing: bool,
    ) -> Result<Option<Height>, Unconfirmed>;
    /// Whether `height` is present.
    async fn is_present(&mut self, height: Height) -> Result<bool, Unconfirmed>;
}

/// What is known about a range before it is searched; its range probe failed with 404.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    /// Some height of the range is missing.
    Missing,
    /// The range was an exact gap on the last search: its first height was missing.
    Gap,
}

/// Finds the gaps of `heights`: first missing height by range probe, gap end by galloping then
/// binary search over single heights, repeated from the gap end. A gap with present heights inside
/// is reported as one gap up to the first present height found by the search. For `Known::Gap`,
/// probes of the first and the last height keep the whole range when both are still missing; a
/// one-height range needs none.
pub async fn find_gaps<P: Presence>(
    presence: &mut P,
    heights: Range<Height>,
    known: Known,
) -> GapSearch {
    if known == Known::Gap && heights.end.0 - heights.start.0 == 1 {
        return exact(vec![heights]);
    }
    if known == Known::Gap {
        match gap_still_missing(presence, &heights).await {
            Ok(true) => return exact(vec![heights]),
            Ok(false) => {},
            Err(reason) => return with_tail(Vec::new(), heights, reason),
        }
    }
    let mut gaps = Vec::new();
    let mut cursor = heights.start;
    let mut known_missing = true;
    while cursor < heights.end {
        match next_gap(presence, cursor..heights.end, known_missing).await {
            Ok(None) => break,
            Ok(Some(gap)) => {
                cursor = gap.end;
                gaps.push(gap);
            },
            Err(tail) => {
                return GapSearch {
                    gaps,
                    tail: Some(tail),
                };
            },
        }
        known_missing = false;
    }
    exact(gaps)
}

/// Gaps with nothing left unrefined.
fn exact(gaps: Vec<Range<Height>>) -> GapSearch {
    GapSearch { gaps, tail: None }
}

/// Gaps found before `heights` could not be refined.
fn with_tail(gaps: Vec<Range<Height>>, heights: Range<Height>, reason: Unconfirmed) -> GapSearch {
    GapSearch {
        gaps,
        tail: Some(Tail { heights, reason }),
    }
}

/// Whether the first and the last height of a known gap are both still missing, which keeps the
/// whole row; a present first height is checked first, so a refilled front costs one request.
async fn gap_still_missing<P: Presence>(
    presence: &mut P,
    heights: &Range<Height>,
) -> Result<bool, Unconfirmed> {
    Ok(!presence.is_present(heights.start).await?
        && !presence.is_present(Height(heights.end.0 - 1)).await?)
}

/// The first gap in `heights`, `None` when nothing is missing, or the rest that cannot be refined.
async fn next_gap<P: Presence>(
    presence: &mut P,
    heights: Range<Height>,
    known_missing: bool,
) -> Result<Option<Range<Height>>, Tail> {
    let first = match presence.first_missing(heights.clone(), known_missing).await {
        Ok(None) => return Ok(None),
        Ok(Some(height)) if heights.contains(&height) => height,
        Ok(Some(_)) => {
            return Err(Tail {
                heights,
                reason: Unconfirmed::OutOfRange,
            });
        },
        Err(reason) => return Err(Tail { heights, reason }),
    };
    match gap_end(presence, first, heights.end).await {
        Ok(end) => Ok(Some(first..end)),
        Err(reason) => Err(Tail {
            heights: first..heights.end,
            reason,
        }),
    }
}

/// End of the gap that starts at the missing height `first`, below `end`.
async fn gap_end<P: Presence>(
    presence: &mut P,
    first: Height,
    end: Height,
) -> Result<Height, Unconfirmed> {
    let (mut missing, mut present) = (first.0, end.0);
    let mut step = 1u64;
    while let Some(probe) = first.0.checked_add(step).filter(|probe| *probe < end.0) {
        if presence.is_present(Height(probe)).await? {
            present = probe;
            break;
        }
        missing = probe;
        step = step.saturating_mul(2);
    }
    while present - missing > 1 {
        let middle = missing + (present - missing) / 2;
        if presence.is_present(Height(middle)).await? {
            present = middle;
        } else {
            missing = middle;
        }
    }
    Ok(Height(present))
}

/// The height named by a server 404 message such as `leaf 99999990 not found`.
pub fn parse_missing_height(message: &str) -> Option<Height> {
    message
        .strip_suffix(" not found")?
        .rsplit(' ')
        .next()?
        .parse()
        .ok()
        .map(Height)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::outcome::Failure;

    #[derive(Default)]
    struct Fake {
        missing: BTreeSet<u64>,
        /// `is_present` fails for heights from here on.
        fail_from: Option<u64>,
        /// `first_missing` always names this height.
        bogus: Option<u64>,
        /// The `known_missing` flag of every `first_missing` call.
        first_missing_flags: Vec<bool>,
        single_probes: usize,
    }

    impl Fake {
        fn new(missing: impl IntoIterator<Item = u64>) -> Self {
            Self {
                missing: missing.into_iter().collect(),
                ..Self::default()
            }
        }
    }

    const FAKE_FAILURE: Unconfirmed = Unconfirmed::Failed(Failure::Transport);

    impl Presence for Fake {
        async fn first_missing(
            &mut self,
            heights: Range<Height>,
            known_missing: bool,
        ) -> Result<Option<Height>, Unconfirmed> {
            self.first_missing_flags.push(known_missing);
            Ok(self
                .bogus
                .or_else(|| {
                    self.missing
                        .range(heights.start.0..heights.end.0)
                        .next()
                        .copied()
                })
                .map(Height))
        }

        async fn is_present(&mut self, height: Height) -> Result<bool, Unconfirmed> {
            self.single_probes += 1;
            if self.fail_from.is_some_and(|from| height.0 >= from) {
                return Err(FAKE_FAILURE);
            }
            Ok(!self.missing.contains(&height.0))
        }
    }

    fn h(range: Range<u64>) -> Range<Height> {
        Height(range.start)..Height(range.end)
    }

    fn one(range: Range<u64>) -> Vec<Range<Height>> {
        vec![h(range)]
    }

    fn found(gaps: Vec<Range<Height>>, tail: Option<(Range<u64>, Unconfirmed)>) -> GapSearch {
        GapSearch {
            gaps,
            tail: tail.map(|(heights, reason)| Tail {
                heights: h(heights),
                reason,
            }),
        }
    }

    #[tokio::test]
    async fn find_gaps_table() {
        let cases = [
            ("no gap", vec![], found(vec![], None)),
            ("contiguous gap", vec![3, 4, 5, 6], found(one(3..7), None)),
            ("gap at start", vec![0, 1, 2, 3], found(one(0..4), None)),
            ("gap at end", vec![6, 7, 8, 9], found(one(6..10), None)),
            ("single height", vec![5], found(one(5..6), None)),
            ("whole chunk", (0..10).collect(), found(one(0..10), None)),
            (
                "two gaps",
                vec![2, 3, 7, 8],
                found(vec![h(2..4), h(7..9)], None),
            ),
        ];
        for (name, missing, expected) in cases {
            let mut fake = Fake::new(missing);
            let search = find_gaps(&mut fake, h(0..10), Known::Missing).await;
            assert_eq!(search, expected, "{name}");
        }
    }

    #[tokio::test]
    async fn find_gaps_long_gap_and_offset_range() {
        let mut fake = Fake::new(1_037..1_900);
        let search = find_gaps(&mut fake, h(1_000..2_000), Known::Missing).await;
        assert_eq!(search, found(one(1_037..1_900), None));
    }

    #[tokio::test]
    async fn only_the_first_range_probe_is_known_missing() {
        let mut fake = Fake::new([2, 3, 7, 8]);
        find_gaps(&mut fake, h(0..10), Known::Missing).await;
        assert_eq!(fake.first_missing_flags, [true, false, false]);
    }

    #[tokio::test]
    async fn find_gaps_keeps_found_gaps_when_a_later_probe_fails() {
        let mut fake = Fake::new([1, 2, 7, 8]);
        fake.fail_from = Some(6);
        let search = find_gaps(&mut fake, h(0..10), Known::Missing).await;
        assert_eq!(search, found(one(1..3), Some((7..10, FAKE_FAILURE))));
    }

    #[tokio::test]
    async fn find_gaps_keeps_the_range_when_a_probe_fails_at_once() {
        let mut fake = Fake::new([3, 4, 5]);
        fake.fail_from = Some(0);
        let search = find_gaps(&mut fake, h(0..10), Known::Missing).await;
        assert_eq!(search, found(vec![], Some((3..10, FAKE_FAILURE))));
    }

    #[tokio::test]
    async fn find_gaps_rejects_a_height_outside_the_range() {
        let mut fake = Fake::new([3]);
        fake.bogus = Some(99);
        let search = find_gaps(&mut fake, h(0..10), Known::Missing).await;
        assert_eq!(
            search,
            found(vec![], Some((0..10, Unconfirmed::OutOfRange)))
        );
    }

    #[tokio::test]
    async fn known_gap_costs_two_probes_while_both_ends_are_missing() {
        let mut fake = Fake::new([3, 4, 5, 6]);
        let search = find_gaps(&mut fake, h(3..7), Known::Gap).await;
        assert_eq!(search, found(one(3..7), None));
        assert_eq!(fake.single_probes, 2);
        assert!(fake.first_missing_flags.is_empty());
    }

    #[tokio::test]
    async fn known_gap_with_a_refilled_end_is_searched_again() {
        let mut front = Fake::new([4, 5, 6]);
        let search = find_gaps(&mut front, h(3..7), Known::Gap).await;
        assert_eq!(search, found(one(4..7), None));

        let mut back = Fake::new([3, 4, 5]);
        let search = find_gaps(&mut back, h(3..7), Known::Gap).await;
        assert_eq!(search, found(one(3..6), None));
    }

    #[tokio::test]
    async fn known_one_height_gap_needs_no_probe() {
        let mut fake = Fake::new([5]);
        let search = find_gaps(&mut fake, h(5..6), Known::Gap).await;
        assert_eq!(search, found(one(5..6), None));
        assert_eq!(fake.single_probes, 0);
        assert!(fake.first_missing_flags.is_empty());
    }

    #[test]
    fn parse_missing_height_table() {
        let cases = [
            ("leaf 99999990 not found", Some(99_999_990)),
            ("VID common 12 not found", Some(12)),
            ("block 7 not found", Some(7)),
            ("leaf abc not found", None),
            ("something else", None),
        ];
        for (message, expected) in cases {
            assert_eq!(
                parse_missing_height(message),
                expected.map(Height),
                "{message}"
            );
        }
    }
}
