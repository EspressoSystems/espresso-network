use minicbor::{Decode, Encode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
#[cbor(transparent)]
pub struct Id(u64);

impl From<u64> for Id {
    fn from(n: u64) -> Self {
        Self(n)
    }
}

impl From<Id> for u64 {
    fn from(n: Id) -> Self {
        n.0
    }
}

impl std::hash::Hash for Id {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        h.write_u64(self.0)
    }
}

impl nohash_hasher::IsEnabled for Id {}
