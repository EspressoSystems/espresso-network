//! Exact gap search inside a height range that failed a range probe.

use std::ops::Range;

/// A request that cannot be refined; `status` labels the failure (`"500"`, `"transport"`,
/// `"no-height"`). The implementor has counted it.
#[derive(Debug)]
pub struct Unrefined {
    pub status: String,
}

/// Which heights a node serves.
pub trait Presence {
    /// First missing height in `range`; `None` when all are present. With `known_missing`, a range
    /// probe already failed with 404, so the height is read without probing the range again.
    async fn first_missing(
        &mut self,
        range: Range<u64>,
        known_missing: bool,
    ) -> Result<Option<u64>, Unrefined>;
    /// Whether `height` is present.
    async fn is_present(&mut self, height: u64) -> Result<bool, Unrefined>;
}

/// What is known about a range before it is searched; its range probe failed with 404.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Known {
    /// Some height of the range is missing.
    Missing,
    /// The range was an exact gap on the last search: its first height was missing.
    Gap,
}

/// The part of a range that could not be refined, and why.
#[derive(Debug, PartialEq, Eq)]
pub struct Tail {
    pub range: Range<u64>,
    pub status: String,
}

/// Exact gaps found in a range, and the tail that could not be refined.
#[derive(Debug, PartialEq, Eq)]
pub struct GapSearch {
    pub gaps: Vec<Range<u64>>,
    pub unrefined: Option<Tail>,
}

/// Finds the gaps of `range`: first missing height by range probe, gap end by galloping then binary
/// search over single heights, repeated from the gap end. A gap with present heights inside is
/// reported as one gap up to the first present height found by the search. For `Known::Gap`, one
/// probe of the last height keeps the whole range when it is still missing.
pub async fn find_gaps<P: Presence>(
    presence: &mut P,
    range: Range<u64>,
    known: Known,
) -> GapSearch {
    if known == Known::Gap {
        match presence.is_present(range.end - 1).await {
            Ok(false) => return GapSearch::exact(vec![range]),
            Ok(true) => {},
            Err(Unrefined { status }) => return GapSearch::unrefined(Vec::new(), range, status),
        }
    }
    let mut gaps = Vec::new();
    let mut cursor = range.start;
    let mut known_missing = true;
    while cursor < range.end {
        match next_gap(presence, cursor..range.end, known_missing).await {
            Ok(None) => break,
            Ok(Some(gap)) => {
                cursor = gap.end;
                gaps.push(gap);
            },
            Err(Tail { range, status }) => return GapSearch::unrefined(gaps, range, status),
        }
        known_missing = false;
    }
    GapSearch::exact(gaps)
}

impl GapSearch {
    /// Gaps with nothing left unrefined.
    fn exact(gaps: Vec<Range<u64>>) -> Self {
        Self {
            gaps,
            unrefined: None,
        }
    }

    /// Gaps found before `range` could not be refined.
    fn unrefined(gaps: Vec<Range<u64>>, range: Range<u64>, status: String) -> Self {
        Self {
            gaps,
            unrefined: Some(Tail { range, status }),
        }
    }
}

/// The first gap in `range`, `None` when nothing is missing, or the rest that cannot be refined.
async fn next_gap<P: Presence>(
    presence: &mut P,
    range: Range<u64>,
    known_missing: bool,
) -> Result<Option<Range<u64>>, Tail> {
    let first = match presence.first_missing(range.clone(), known_missing).await {
        Ok(None) => return Ok(None),
        Ok(Some(height)) if range.contains(&height) => height,
        Ok(Some(_)) => return Err(Tail::new(range, "out-of-range")),
        Err(Unrefined { status }) => return Err(Tail { range, status }),
    };
    match gap_end(presence, first, range.end).await {
        Ok(end) => Ok(Some(first..end)),
        Err(Unrefined { status }) => Err(Tail {
            range: first..range.end,
            status,
        }),
    }
}

impl Tail {
    /// A tail with a fixed status label.
    fn new(range: Range<u64>, status: &str) -> Self {
        Self {
            range,
            status: status.to_owned(),
        }
    }
}

/// End of the gap that starts at the missing height `first`, below `end`.
async fn gap_end<P: Presence>(presence: &mut P, first: u64, end: u64) -> Result<u64, Unrefined> {
    let (mut missing, mut present) = (first, end);
    let mut step = 1u64;
    while let Some(probe) = first.checked_add(step).filter(|probe| *probe < end) {
        if presence.is_present(probe).await? {
            present = probe;
            break;
        }
        missing = probe;
        step = step.saturating_mul(2);
    }
    while present - missing > 1 {
        let middle = missing + (present - missing) / 2;
        if presence.is_present(middle).await? {
            present = middle;
        } else {
            missing = middle;
        }
    }
    Ok(present)
}

/// The height named by a server 404 message such as `leaf 99999990 not found`.
pub fn parse_missing_height(message: &str) -> Option<u64> {
    message
        .strip_suffix(" not found")?
        .rsplit(' ')
        .next()?
        .parse()
        .ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

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

    impl Presence for Fake {
        async fn first_missing(
            &mut self,
            range: Range<u64>,
            known_missing: bool,
        ) -> Result<Option<u64>, Unrefined> {
            self.first_missing_flags.push(known_missing);
            Ok(self
                .bogus
                .or_else(|| self.missing.range(range).next().copied()))
        }

        async fn is_present(&mut self, height: u64) -> Result<bool, Unrefined> {
            self.single_probes += 1;
            if self.fail_from.is_some_and(|from| height >= from) {
                return Err(Unrefined {
                    status: "fake".to_owned(),
                });
            }
            Ok(!self.missing.contains(&height))
        }
    }

    fn one(range: Range<u64>) -> Vec<Range<u64>> {
        vec![range]
    }

    fn found(gaps: Vec<Range<u64>>, tail: Option<(Range<u64>, &str)>) -> GapSearch {
        GapSearch {
            gaps,
            unrefined: tail.map(|(range, status)| Tail::new(range, status)),
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
            ("two gaps", vec![2, 3, 7, 8], found(vec![2..4, 7..9], None)),
        ];
        for (name, missing, expected) in cases {
            let mut fake = Fake::new(missing);
            let search = find_gaps(&mut fake, 0..10, Known::Missing).await;
            assert_eq!(search, expected, "{name}");
        }
    }

    #[tokio::test]
    async fn find_gaps_long_gap_and_offset_range() {
        let mut fake = Fake::new(1_037..1_900);
        let search = find_gaps(&mut fake, 1_000..2_000, Known::Missing).await;
        assert_eq!(search, found(one(1_037..1_900), None));
    }

    #[tokio::test]
    async fn only_the_first_range_probe_is_known_missing() {
        let mut fake = Fake::new([2, 3, 7, 8]);
        find_gaps(&mut fake, 0..10, Known::Missing).await;
        assert_eq!(fake.first_missing_flags, [true, false, false]);
    }

    #[tokio::test]
    async fn find_gaps_keeps_found_gaps_when_a_later_probe_fails() {
        let mut fake = Fake::new([1, 2, 7, 8]);
        fake.fail_from = Some(6);
        let search = find_gaps(&mut fake, 0..10, Known::Missing).await;
        assert_eq!(search, found(one(1..3), Some((7..10, "fake"))));
    }

    #[tokio::test]
    async fn find_gaps_keeps_the_range_when_a_probe_fails_at_once() {
        let mut fake = Fake::new([3, 4, 5]);
        fake.fail_from = Some(0);
        let search = find_gaps(&mut fake, 0..10, Known::Missing).await;
        assert_eq!(search, found(vec![], Some((3..10, "fake"))));
    }

    #[tokio::test]
    async fn find_gaps_rejects_a_height_outside_the_range() {
        let mut fake = Fake::new([3]);
        fake.bogus = Some(99);
        let search = find_gaps(&mut fake, 0..10, Known::Missing).await;
        assert_eq!(search, found(vec![], Some((0..10, "out-of-range"))));
    }

    #[tokio::test]
    async fn known_gap_costs_one_probe_while_its_last_height_is_missing() {
        let mut fake = Fake::new([3, 4, 5, 6]);
        let search = find_gaps(&mut fake, 3..7, Known::Gap).await;
        assert_eq!(search, found(one(3..7), None));
        assert_eq!(fake.single_probes, 1);
        assert!(fake.first_missing_flags.is_empty());

        let mut repaired = Fake::new([3, 4, 5]);
        let search = find_gaps(&mut repaired, 3..7, Known::Gap).await;
        assert_eq!(search, found(one(3..6), None));
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
            assert_eq!(parse_missing_height(message), expected, "{message}");
        }
    }
}
