use serde::{Deserialize, Serialize};

use super::{AvidMNsProof, TxIndex};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AvidMTxProof {
    pub(crate) tx_index: TxIndex,
    pub(crate) ns_proof: AvidMNsProof,
}

impl AvidMTxProof {
    pub fn tx_index(&self) -> &TxIndex {
        &self.tx_index
    }

    pub fn ns_proof(&self) -> &AvidMNsProof {
        &self.ns_proof
    }
}
