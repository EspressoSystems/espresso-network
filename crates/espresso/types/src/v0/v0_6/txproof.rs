use serde::{Deserialize, Serialize};

use super::{AvidmGf2NsProof, TxIndex};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AvidmGf2TxProof {
    pub(crate) tx_index: TxIndex,
    pub(crate) ns_proof: AvidmGf2NsProof,
}

impl AvidmGf2TxProof {
    pub fn tx_index(&self) -> &TxIndex {
        &self.tx_index
    }

    pub fn ns_proof(&self) -> &AvidmGf2NsProof {
        &self.ns_proof
    }
}
