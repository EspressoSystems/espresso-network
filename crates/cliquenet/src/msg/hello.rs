use std::time::Duration;

use crate::LaneId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hello {
    Ok,
    BackOff(Duration),
    OkLane(LaneId),
    Unknown,
}

impl Hello {
    pub fn is_ok(&self) -> bool {
        matches!(self, Self::Ok)
    }

    pub fn backoff_duration(&self) -> Option<Duration> {
        if let Self::BackOff(d) = self {
            Some(*d)
        } else {
            None
        }
    }

    /// The answer to `theirs` from a party that would otherwise answer `self`
    /// and serves `lanes`.
    ///
    /// A party accepts a lane by echoing the hello that asked for it, which is
    /// what the connecting side checks for.
    pub fn answer(self, theirs: Option<Hello>, lanes: &[LaneId], backoff: Duration) -> Hello {
        match (self, theirs) {
            (Hello::Ok, Some(Hello::OkLane(id))) if lanes.contains(&id) => Hello::OkLane(id),
            (Hello::Ok, Some(Hello::OkLane(_))) => Hello::BackOff(backoff),
            (ours, _) => ours,
        }
    }
}

impl Hello {
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes.first()? {
            0 => Some(Self::Ok),
            1 => {
                let d = u64::from_be_bytes(bytes.get(1..9)?.try_into().ok()?);
                Some(Self::BackOff(Duration::from_secs(d)))
            },
            2 => Some(Self::OkLane(LaneId(*bytes.get(1)?))),
            _ => Some(Self::Unknown),
        }
    }

    pub fn to_bytes(self) -> HelloBytes {
        match self {
            Self::Ok => HelloBytes::Ok([0]),
            Self::OkLane(id) => HelloBytes::Lane([2, id.0]),
            Self::BackOff(d) => {
                let mut b = [1; 9];
                b[1..].copy_from_slice(&d.as_secs().to_be_bytes());
                HelloBytes::BackOff(b)
            },
            Self::Unknown => unreachable!("nothing constructs Hello::Unknown"),
        }
    }
}

pub(crate) enum HelloBytes {
    Ok([u8; 1]),
    Lane([u8; 2]),
    BackOff([u8; 9]),
}

impl AsRef<[u8]> for HelloBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Ok(a) => &a[..],
            Self::Lane(a) => &a[..],
            Self::BackOff(a) => &a[..],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use quickcheck::{Arbitrary, Gen, quickcheck};

    use super::Hello;
    use crate::LaneId;

    impl Arbitrary for Hello {
        fn arbitrary(g: &mut Gen) -> Self {
            match u8::arbitrary(g) % 3 {
                0 => Self::Ok,
                1 => Self::OkLane(LaneId(u8::arbitrary(g))),
                _ => Self::BackOff(Duration::from_secs(u64::arbitrary(g))),
            }
        }
    }

    quickcheck! {
        fn prop_to_bytes_from_bytes_id(h1: Hello) -> bool {
            let b = h1.to_bytes();
            let h2 = Hello::from_bytes(b.as_ref());
            Some(h1) == h2
        }
    }
}
