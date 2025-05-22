use hotshot_query_service::VidCommon;
use hotshot_types::{data::VidCommitment, vid::avidm::AvidMShare};
use serde::{Deserialize, Serialize};

use crate::{
    v0::{NamespaceId, NsIndex, NsPayload, NsTable, Payload, Transaction},
    v0_1::ADVZNsProof,
    v0_3::{AvidMNsProof, AvidMNsProofV1},
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
    /// V1_1 proof for AvidM, contains both correct and incorrect encoding proofs
    V1_1(AvidMNsProofV1),
}

impl NsProof {
    pub fn new(payload: &Payload, index: &NsIndex, common: &VidCommon) -> Option<NsProof> {
        match common {
            VidCommon::V0(common) => Some(NsProof::V0(ADVZNsProof::new(payload, index, common)?)),
            VidCommon::V1(common) => Some(NsProof::V1(AvidMNsProof::new(payload, index, common)?)),
        }
    }

    pub fn v1_1_new_with_correct_encoding(
        payload: &Payload,
        index: &NsIndex,
        common: &VidCommon,
    ) -> Option<NsProof> {
        match common {
            VidCommon::V1(common) => Some(NsProof::V1_1(AvidMNsProofV1::new_correct_encoding(
                payload, index, common,
            )?)),
            _ => None,
        }
    }

    pub fn v1_1_new_with_incorrect_encoding(
        shares: &[AvidMShare],
        ns_table: &NsTable,
        index: &NsIndex,
        commit: &VidCommitment,
        common: &VidCommon,
    ) -> Option<NsProof> {
        match common {
            VidCommon::V1(common) => Some(NsProof::V1_1(AvidMNsProofV1::new_incorrect_encoding(
                shares, ns_table, index, commit, common,
            )?)),
            _ => None,
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
            (Self::V1_1(proof), VidCommon::V1(common)) => proof.verify(ns_table, commit, common),
            _ => {
                tracing::error!("Incompatible version of VidCommon and NsProof.");
                None
            },
        }
    }

    pub fn export_all_txs(&self, ns_id: &NamespaceId) -> Vec<Transaction> {
        match self {
            Self::V0(proof) => proof.export_all_txs(ns_id),
            Self::V1(AvidMNsProof(proof)) | Self::V1_1(AvidMNsProofV1::CorrectEncoding(proof)) => {
                NsPayload::from_bytes_slice(&proof.ns_payload).export_all_txs(ns_id)
            },
            Self::V1_1(AvidMNsProofV1::IncorrectEncoding(_)) => vec![],
        }
    }
}
