use super::{v0_1::ADVZTxProof, v0_3::AvidMTxProof, Index, NsTable, Payload, Transaction};
use hotshot_query_service::VidCommon;
use hotshot_types::data::VidCommitment;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxProof {
    V0(ADVZTxProof),
    V1(AvidMTxProof),
}

impl TxProof {
    pub fn new(
        index: &Index,
        payload: &Payload,
        common: &VidCommon,
    ) -> Option<(Transaction, Self)> {
        match common {
            VidCommon::V0(common) => {
                ADVZTxProof::new(index, payload, common).map(|(tx, proof)| (tx, TxProof::V0(proof)))
            },
            VidCommon::V1(common) => AvidMTxProof::new(index, payload, common)
                .map(|(tx, proof)| (tx, TxProof::V1(proof))),
        }
    }

    pub fn verify(
        &self,
        ns_table: &NsTable,
        tx: &Transaction,
        commit: &VidCommitment,
        common: &VidCommon,
    ) -> Option<bool> {
        match self {
            TxProof::V0(tx_proof) => tx_proof.verify(ns_table, tx, commit, common),
            TxProof::V1(tx_proof) => tx_proof.verify(ns_table, tx, commit, common),
        }
    }
}
