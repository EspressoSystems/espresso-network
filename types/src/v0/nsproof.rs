use hotshot_query_service::VidCommon;
use hotshot_types::{data::VidCommitment, vid::avidm::AvidMShare};
use serde::{Deserialize, Serialize};

use crate::{
    v0::{NamespaceId, NsIndex, NsPayload, NsTable, Payload, Transaction},
    v0_1::ADVZNsProof,
    v0_3::{AvidMIncorrectEncodingNsProof, AvidMNsProof},
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
}

impl NsProof {
    pub fn new(payload: &Payload, index: &NsIndex, common: &VidCommon) -> Option<NsProof> {
        match common {
            VidCommon::V0(common) => Some(NsProof::V0(ADVZNsProof::new(payload, index, common)?)),
            VidCommon::V1(common) => Some(NsProof::V1(AvidMNsProof::new(payload, index, common)?)),
            _ => todo!("unsupported VidCommon version"),
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
            VidCommon::V1(common) => Some(NsProof::V1IncorrectEncoding(
                AvidMIncorrectEncodingNsProof::new(shares, ns_table, index, commit, common)?,
            )),
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
            (Self::V1IncorrectEncoding(proof), VidCommon::V1(common)) => {
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
        }
    }
}
