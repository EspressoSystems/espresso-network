use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use derive_more::{Display, From, Into};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Hash,
    CanonicalSerialize,
    CanonicalDeserialize,
)]
pub struct Transaction {
    pub namespace: NamespaceId,
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
}

#[derive(
    Clone,
    Copy,
    Serialize,
    Debug,
    Display,
    PartialEq,
    Eq,
    Hash,
    Into,
    From,
    Default,
    CanonicalDeserialize,
    CanonicalSerialize,
    PartialOrd,
    Ord,
)]
#[display("{_0}")]
pub struct NamespaceId(pub u64);
