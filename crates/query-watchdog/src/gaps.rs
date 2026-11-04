//! Exact gap search inside a height range that failed a range probe.

use std::ops::Range;

/// A request that cannot be refined: a non-404 status, a transport error or an unreadable 404
/// body. The implementor has counted it.
#[derive(Debug)]
pub struct Unrefined;

/// Which heights a node serves.
pub trait Presence {
    /// First missing height in `range`; `None` when all are present.
    async fn first_missing(&mut self, range: Range<u64>) -> Result<Option<u64>, Unrefined>;
    /// Whether `height` is present.
    async fn is_present(&mut self, height: u64) -> Result<bool, Unrefined>;
}

/// Exact gaps found in a range, and the tail that could not be refined.
#[derive(Debug, PartialEq, Eq)]
pub struct GapSearch {
    pub gaps: Vec<Range<u64>>,
    pub unrefined: Option<Range<u64>>,
}

/// Finds the gaps of `range`: first missing height by range probe, gap end by galloping then binary
/// search over single heights, repeated from the gap end. A gap with present heights inside is
/// reported as one gap up to the first present height found by the search.
pub async fn find_gaps<P: Presence>(presence: &mut P, range: Range<u64>) -> GapSearch {
    let mut search = GapSearch {
        gaps: Vec::new(),
        unrefined: None,
    };
    let mut cursor = range.start;
    while cursor < range.end {
        match next_gap(presence, cursor..range.end).await {
            Ok(None) => break,
            Ok(Some(gap)) => {
                cursor = gap.end;
                search.gaps.push(gap);
            },
            Err(rest) => {
                search.unrefined = Some(rest);
                break;
            },
        }
    }
    search
}

/// The first gap in `range`, `None` when nothing is missing, or the rest that cannot be refined.
async fn next_gap<P: Presence>(
    presence: &mut P,
    range: Range<u64>,
) -> Result<Option<Range<u64>>, Range<u64>> {
    let first = match presence.first_missing(range.clone()).await {
        Ok(None) => return Ok(None),
        Ok(Some(height)) if range.contains(&height) => height,
        Ok(Some(_)) | Err(Unrefined) => return Err(range),
    };
    match gap_end(presence, first, range.end).await {
        Ok(end) => Ok(Some(first..end)),
        Err(Unrefined) => Err(first..range.end),
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

    struct Fake {
        missing: BTreeSet<u64>,
        fail_single: bool,
    }

    impl Fake {
        fn new(missing: impl IntoIterator<Item = u64>) -> Self {
            Self {
                missing: missing.into_iter().collect(),
                fail_single: false,
            }
        }
    }

    impl Presence for Fake {
        async fn first_missing(&mut self, range: Range<u64>) -> Result<Option<u64>, Unrefined> {
            Ok(self.missing.range(range).next().copied())
        }

        async fn is_present(&mut self, height: u64) -> Result<bool, Unrefined> {
            if self.fail_single {
                return Err(Unrefined);
            }
            Ok(!self.missing.contains(&height))
        }
    }

    fn one(range: Range<u64>) -> Vec<Range<u64>> {
        vec![range]
    }

    fn found(gaps: Vec<Range<u64>>, unrefined: Option<Range<u64>>) -> GapSearch {
        GapSearch { gaps, unrefined }
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
            assert_eq!(find_gaps(&mut fake, 0..10).await, expected, "{name}");
        }
    }

    #[tokio::test]
    async fn find_gaps_long_gap_and_offset_range() {
        let mut fake = Fake::new(1_037..1_900);
        let search = find_gaps(&mut fake, 1_000..2_000).await;
        assert_eq!(search, found(one(1_037..1_900), None));
    }

    #[tokio::test]
    async fn find_gaps_keeps_rest_when_a_probe_fails() {
        let mut fake = Fake::new([3, 4, 5]);
        fake.fail_single = true;
        assert_eq!(
            find_gaps(&mut fake, 0..10).await,
            found(vec![], Some(3..10))
        );
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
