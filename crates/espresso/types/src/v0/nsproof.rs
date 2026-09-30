use hotshot_types::data::{VidCommitment, VidCommon, VidShare};
use serde::{Deserialize, Serialize};

use crate::{
    v0::{NamespaceId, NsIndex, NsPayload, NsTable, Payload, Transaction},
    v0_1::ADVZNsProof,
    v0_3::{AvidMIncorrectEncodingNsProof, AvidMNsProof},
    v0_6::{AvidmGf2IncorrectEncodingNsProof, AvidmGf2NsProof},
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NamespaceProofQueryData {
    pub proof: Option<NsProof>,
    pub transactions: Vec<Transaction>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ADVZNamespaceProofQueryData {
    pub proof: Option<ADVZNsProof>,
    pub transactions: Vec<Transaction>,
}

/// Each variant represents a specific version of a namespace proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NsProof {
    /// V0 proof for ADVZ
    V0(ADVZNsProof),
    /// V1 proof for AvidM, contains only correct encoding proof
    V1(AvidMNsProof),
    /// Incorrect encoding proof for AvidM (only supported after API version 1.1)
    V1IncorrectEncoding(AvidMIncorrectEncodingNsProof),
    /// V2 proof for AvidmGf2
    V2(AvidmGf2NsProof),
    /// Incorrect encoding proof for AvidmGf2: the namespace is empty for this block
    V2IncorrectEncoding(AvidmGf2IncorrectEncodingNsProof),
}

impl NsProof {
    pub fn new(payload: &Payload, index: &NsIndex, common: &VidCommon) -> Option<NsProof> {
        match common {
            VidCommon::V0(common) => Some(NsProof::V0(ADVZNsProof::new(payload, index, common)?)),
            VidCommon::V1(common) => Some(NsProof::V1(AvidMNsProof::new(payload, index, common)?)),
            VidCommon::V2(common) => {
                Some(NsProof::V2(AvidmGf2NsProof::new(payload, index, common)?))
            },
        }
    }

    /// A proof that the namespace at `index` was dispersed as a non-codeword, from `shares` that
    /// verify against `commit`. Shares of a VID scheme other than `common`'s are ignored.
    pub fn new_with_incorrect_encoding(
        shares: &[VidShare],
        ns_table: &NsTable,
        index: &NsIndex,
        commit: &VidCommitment,
        common: &VidCommon,
    ) -> Option<NsProof> {
        match common {
            VidCommon::V0(_) => None,
            VidCommon::V1(common) => {
                let shares: Vec<_> = shares
                    .iter()
                    .filter_map(|share| match share {
                        VidShare::V1(share) => Some(share.clone()),
                        _ => None,
                    })
                    .collect();
                Some(NsProof::V1IncorrectEncoding(
                    AvidMIncorrectEncodingNsProof::new(&shares, ns_table, index, commit, common)?,
                ))
            },
            VidCommon::V2(common) => {
                let shares: Vec<_> = shares
                    .iter()
                    .filter_map(|share| match share {
                        VidShare::V2(share) => Some(share.clone()),
                        _ => None,
                    })
                    .collect();
                Some(NsProof::V2IncorrectEncoding(
                    AvidmGf2IncorrectEncodingNsProof::new(
                        &shares, ns_table, index, commit, common,
                    )?,
                ))
            },
        }
    }

    pub fn verify(
        &self,
        ns_table: &NsTable,
        commit: &VidCommitment,
        common: &VidCommon,
    ) -> Option<(Vec<Transaction>, NamespaceId)> {
        match (self, common) {
            (Self::V0(proof), VidCommon::V0(common)) => proof.verify(ns_table, commit, common),
            (Self::V1(proof), VidCommon::V1(common)) => proof.verify(ns_table, commit, common),
            (Self::V1IncorrectEncoding(proof), VidCommon::V1(common)) => {
                proof.verify(ns_table, commit, common)
            },
            (Self::V2(proof), VidCommon::V2(_)) => proof.verify(ns_table, commit, common),
            (Self::V2IncorrectEncoding(proof), VidCommon::V2(common)) => {
                proof.verify(ns_table, commit, common)
            },
            _ => {
                tracing::error!("Incompatible version of VidCommon and NsProof.");
                None
            },
        }
    }

    pub fn export_all_txs(&self, ns_id: &NamespaceId) -> Vec<Transaction> {
        match self {
            Self::V0(proof) => proof.export_all_txs(ns_id),
            Self::V1(AvidMNsProof(proof)) => {
                NsPayload::from_bytes_slice(&proof.ns_payload).export_all_txs(ns_id)
            },
            Self::V1IncorrectEncoding(_) => vec![],
            Self::V2(AvidmGf2NsProof(proof)) => {
                NsPayload::from_bytes_slice(&proof.ns_payload).export_all_txs(ns_id)
            },
            Self::V2IncorrectEncoding(_) => vec![],
        }
    }
}
