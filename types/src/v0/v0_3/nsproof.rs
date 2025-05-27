use serde::{Deserialize, Serialize};

/// Re-export the AVID-M namespace proof.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct AvidMNsProof(pub(crate) vid::avid_m::proofs::NsProof);

/// The namespace proof for both correct and incorrect encoding.
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum AvidMNsProofV1 {
    /// V1 proof for AvidM, contains only correct encoding proof
    CorrectEncoding(vid::avid_m::proofs::NsProof),
    /// V1_1 proof for AvidM, contains both correct and incorrect encoding proofs
    IncorrectEncoding(vid::avid_m::proofs::NsAvidMBadEncodingProof),
}

impl From<AvidMNsProof> for AvidMNsProofV1 {
    fn from(proof: AvidMNsProof) -> Self {
        AvidMNsProofV1::CorrectEncoding(proof.0)
    }
}

impl TryFrom<AvidMNsProofV1> for AvidMNsProof {
    type Error = anyhow::Error;

    fn try_from(proof: AvidMNsProofV1) -> Result<Self, Self::Error> {
        match proof {
            AvidMNsProofV1::CorrectEncoding(proof) => Ok(AvidMNsProof(proof)),
            AvidMNsProofV1::IncorrectEncoding(_) => {
                Err(anyhow::anyhow!("incorrect encoding proof"))
            },
        }
    }
}
