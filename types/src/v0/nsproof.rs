use hotshot_query_service::VidCommon;
use hotshot_types::data::VidCommitment;
use serde::{Deserialize, Serialize};

use crate::{
    v0::{NamespaceId, NsIndex, NsPayload, NsTable, Payload, Transaction},
    v0_1::ADVZNsProof,
    v0_3::AvidMNsProof,
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
    V0(ADVZNsProof),
    V1(AvidMNsProof),
}

impl NsProof {
    pub fn new(payload: &Payload, index: &NsIndex, common: &VidCommon) -> Option<NsProof> {
        match common {
            VidCommon::V0(common) => Some(NsProof::V0(ADVZNsProof::new(payload, index, common)?)),
            VidCommon::V1(common) => Some(NsProof::V1(AvidMNsProof::new(payload, index, common)?)),
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
            _ => {
                tracing::error!("Incompatible version of VidCommon and NsProof.");
                None
            },
        }
    }

    pub fn export_all_txs(&self, ns_id: &NamespaceId) -> Vec<Transaction> {
        match self {
            Self::V0(proof) => proof.export_all_txs(ns_id),
            Self::V1(proof) => {
                NsPayload::from_bytes_slice(&proof.0.ns_payload).export_all_txs(ns_id)
            },
        }
    }
}
