use super::{AvidMNsProof, TxIndex};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AvidMTxProof {
    pub(crate) tx_index: TxIndex,
    pub(crate) ns_proof: AvidMNsProof,
}
