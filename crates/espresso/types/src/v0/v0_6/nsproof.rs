use serde::{Deserialize, Serialize};

/// Re-export the AvidmGf2 namespace proof.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvidmGf2NsProof(pub vid::avidm_gf2::proofs::NsProof);

/// The namespace proof for incorrect encoding under AvidmGf2. A proof that verifies means the
/// namespace was dispersed as a non-codeword, so it is empty for that block.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvidmGf2IncorrectEncodingNsProof(pub vid::avidm_gf2::proofs::NsAvidmGf2BadEncodingProof);
