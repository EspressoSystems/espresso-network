use hotshot_query_service_types::availability::VerifiableInclusion;
use hotshot_types::{
    data::{VidCommitment, VidCommon},
    vid::avidm_gf2::avidm_gf2_binding,
};
use serde::{Deserialize, Serialize};
use vbs::version::Version;

use super::{
    Index, NsTable, Payload, Transaction, v0_1::ADVZTxProof, v0_3::AvidMTxProof,
    v0_6::AvidmGf2TxProof,
};
use crate::SeqTypes;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxProof {
    V0(ADVZTxProof),
    V1(AvidMTxProof),
    V2(AvidmGf2TxProof),
}

impl TxProof {
    /// Prove the transaction at `index`. `version` is the protocol version of
    /// the block, which fixes what its V2 commitment binds.
    pub fn new(
        index: &Index,
        payload: &Payload,
        common: &VidCommon,
        version: Version,
    ) -> Option<(Transaction, Self)> {
        match common {
            VidCommon::V0(common) => {
                ADVZTxProof::new(index, payload, common).map(|(tx, proof)| (tx, TxProof::V0(proof)))
            },
            VidCommon::V1(common) => AvidMTxProof::new(index, payload, common)
                .map(|(tx, proof)| (tx, TxProof::V1(proof))),
            VidCommon::V2(common) => {
                AvidmGf2TxProof::new(index, payload, common, avidm_gf2_binding(version))
                    .map(|(tx, proof)| (tx, TxProof::V2(proof)))
            },
        }
    }
}

impl VerifiableInclusion<SeqTypes> for TxProof {
    fn verify(
        &self,
        ns_table: &NsTable,
        tx: &Transaction,
        commit: &VidCommitment,
        common: &VidCommon,
        version: Version,
    ) -> bool {
        match self {
            TxProof::V0(tx_proof) => tx_proof.verify(ns_table, tx, commit, common),
            TxProof::V1(tx_proof) => tx_proof.verify(ns_table, tx, commit, common),
            TxProof::V2(tx_proof) => {
                tx_proof.verify(ns_table, tx, commit, common, avidm_gf2_binding(version))
            },
        }
    }
}
