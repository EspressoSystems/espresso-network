use serde::{Deserialize, Serialize};

/// Re-export the AVID-M namespace proof.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvidMNsProof(pub(crate) vid::avid_m::proofs::NsProof);

/// The namespace proof for incorrect encoding.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvidMIncorrectEncodingNsProof(pub(crate) vid::avid_m::proofs::NsAvidMBadEncodingProof);
