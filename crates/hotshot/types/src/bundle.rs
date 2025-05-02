//! This module provides the `Bundle` type

use serde::{Deserialize, Serialize};

use crate::traits::{
    block_contents::BuilderFee, node_implementation::NodeType, signature_key::BuilderSignatureKey,
    BlockPayload,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "TYPES: NodeType")]
/// The Bundle for a portion of a block, provided by a downstream
/// builder that exists in a bundle auction.
/// This type is maintained by HotShot
pub struct Bundle<TYPES: NodeType> {
    /// The bundle transactions sent by the builder.
    pub transactions: Vec<<TYPES::BlockPayload as BlockPayload<TYPES>>::Transaction>,

    /// The signature over the bundle.
    pub signature: <TYPES::BuilderSignatureKey as BuilderSignatureKey>::BuilderSignature,

    /// The fee for sequencing
    pub sequencing_fee: BuilderFee<TYPES>,
}
