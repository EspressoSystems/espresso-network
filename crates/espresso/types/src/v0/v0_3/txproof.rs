use serde::{Deserialize, Serialize};

use super::{AvidMNsProof, TxIndex};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AvidMTxProof {
    pub(crate) tx_index: TxIndex,
    pub(crate) ns_proof: AvidMNsProof,
}
