//! Detecting that the stored state belongs to another network.

use crate::chunk::CHUNK_LEN;

/// A recent block of the watched network, stored with the state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Anchor {
    pub height: u64,
    pub hash: String,
}

/// What the stored anchor and progress say about the live network.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Nothing contradicts the stored state.
    Consistent,
    /// The nodes no longer serve the anchor block (pruned), so the state cannot be checked.
    Unverifiable,
    /// The state belongs to another network or a reset chain; the reason names the evidence.
    Mismatch(String),
}

/// Compares the stored `anchor` with the `live_hash` of the block at its height, and the largest
/// stored follow height with the live tip. A follow height cannot be above the tip, apart from
/// the window start rounding up by less than `CHUNK_LEN`.
pub fn verdict(
    anchor: Option<&Anchor>,
    live_hash: Option<&str>,
    max_follow_height: Option<u64>,
    tip_height: u64,
) -> Verdict {
    if let Some(follow) = max_follow_height.filter(|follow| *follow > tip_height + CHUNK_LEN) {
        return Verdict::Mismatch(format!(
            "stored follow height {follow} is above the tip {tip_height}"
        ));
    }
    match (anchor, live_hash) {
        (Some(anchor), Some(live)) if live != anchor.hash => Verdict::Mismatch(format!(
            "block {} has hash {live}, stored {}",
            anchor.height, anchor.hash
        )),
        (Some(_), None) => Verdict::Unverifiable,
        _ => Verdict::Consistent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor() -> Anchor {
        Anchor {
            height: 500,
            hash: "abc".to_owned(),
        }
    }

    #[test]
    fn verdict_table() {
        let mismatch = |verdict: Verdict| matches!(verdict, Verdict::Mismatch(_));
        assert_eq!(verdict(None, None, None, 1_000), Verdict::Consistent);
        assert_eq!(
            verdict(Some(&anchor()), Some("abc"), Some(990), 1_000),
            Verdict::Consistent
        );
        assert!(mismatch(verdict(
            Some(&anchor()),
            Some("other"),
            Some(990),
            1_000
        )));
        assert_eq!(
            verdict(Some(&anchor()), None, Some(990), 1_000),
            Verdict::Unverifiable
        );
        // The window start may round up past the tip by less than a chunk.
        assert_eq!(verdict(None, None, Some(1_050), 1_000), Verdict::Consistent);
        assert!(mismatch(verdict(None, None, Some(1_101), 1_000)));
        // A chain that went backwards is a mismatch even when the anchor is unverifiable.
        assert!(mismatch(verdict(Some(&anchor()), None, Some(5_000), 1_000)));
    }
}
