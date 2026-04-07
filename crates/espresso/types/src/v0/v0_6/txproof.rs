use super::{AvidmGf2NsProof, TxIndex};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AvidmGf2TxProof {
    pub(crate) tx_index: TxIndex,
    pub(crate) ns_proof: AvidmGf2NsProof,
}
