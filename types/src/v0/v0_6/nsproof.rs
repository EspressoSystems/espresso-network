use serde::{Deserialize, Serialize};

/// Re-export the AvidmGf2 namespace proof.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvidmGf2NsProof(pub(crate) vid::avidm_gf2::proofs::NsProof);
