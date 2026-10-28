use std::time::Duration;

use crate::Lane;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hello {
    Ok,
    BackOff(Duration),
    OkQuery,
    Unknown,
}

impl Hello {
    /// The lane an accepting hello opens, if any.
    pub fn lane(self) -> Option<Lane> {
        match self {
            Hello::Ok => Some(Lane::Consensus),
            Hello::OkQuery => Some(Lane::Query),
            Hello::BackOff(_) | Hello::Unknown => None,
        }
    }

    pub fn backoff_duration(&self) -> Option<Duration> {
        if let Self::BackOff(d) = self {
            Some(*d)
        } else {
            None
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
            2 => Some(Self::OkQuery),
            _ => Some(Self::Unknown),
        }
    }

    pub fn to_bytes(self) -> HelloBytes {
        match self {
            Self::Ok => HelloBytes::Ok([0]),
            Self::OkQuery => HelloBytes::Ok([2]),
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
    BackOff([u8; 9]),
}

impl AsRef<[u8]> for HelloBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Ok(a) => &a[..],
            Self::BackOff(a) => &a[..],
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use quickcheck::{Arbitrary, Gen, quickcheck};

    use super::Hello;

    impl Arbitrary for Hello {
        fn arbitrary(g: &mut Gen) -> Self {
            match u8::arbitrary(g) % 3 {
                0 => Self::Ok,
                1 => Self::OkQuery,
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
