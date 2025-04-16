// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{hash::Hash, marker::PhantomData};

use hotshot_types::{
    traits::{node_implementation::NodeType, signature_key::BuilderSignatureKey, BlockPayload},
    utils::BuilderCommitment,
    vid::advz::ADVZCommitment,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(bound = "")]
pub struct AvailableBlockInfo<TYPES: NodeType> {
    pub block_hash: BuilderCommitment,
    pub block_size: u64,
    pub offered_fee: u64,
    pub signature:
        <<TYPES as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderSignature,
    pub sender: <TYPES as NodeType>::BuilderSignatureKey,
    pub _phantom: PhantomData<TYPES>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(bound = "")]
pub struct AvailableBlockData<TYPES: NodeType> {
    pub block_payload: TYPES::BlockPayload,
    pub metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    pub signature:
        <<TYPES as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderSignature,
    pub sender: <TYPES as NodeType>::BuilderSignatureKey,
}

impl<TYPES: NodeType> AvailableBlockData<TYPES> {
    pub fn validate_signature(&self) -> bool {
        // verify the signature over the message, construct the builder commitment
        let builder_commitment = self.block_payload.builder_commitment(&self.metadata);
        self.sender
            .validate_builder_signature(&self.signature, builder_commitment.as_ref())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(bound = "")]
pub struct AvailableBlockHeaderInputV1<TYPES: NodeType> {
    // TODO Add precompute back.
    // signature over vid_commitment, BlockPayload::Metadata, and offered_fee
    pub fee_signature:
        <<TYPES as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderSignature,
    pub sender: <TYPES as NodeType>::BuilderSignatureKey,
}

impl<TYPES: NodeType> AvailableBlockHeaderInputV1<TYPES> {
    pub fn validate_signature(
        &self,
        offered_fee: u64,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) -> bool {
        self.sender
            .validate_fee_signature(&self.fee_signature, offered_fee, metadata)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(bound = "")]
pub struct AvailableBlockHeaderInputV2<TYPES: NodeType> {
    // signature over vid_commitment, BlockPayload::Metadata, and offered_fee
    pub fee_signature:
        <<TYPES as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderSignature,
    pub sender: <TYPES as NodeType>::BuilderSignatureKey,
}

/// legacy version of the AvailableBlockHeaderInputV2 type, used on git tag `20280228-patch3`
///
/// this was inadvertently changed to remove some deprecated fields,
/// which resulted in a builder incompatibility.
///
/// This type can be removed after the builder is upgraded in deployment.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(bound = "")]
pub struct AvailableBlockHeaderInputV2Legacy<TYPES: NodeType> {
    pub vid_commitment: ADVZCommitment,
    // signature over vid_commitment, BlockPayload::Metadata, and offered_fee
    pub fee_signature:
        <<TYPES as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderSignature,
    // signature over the current response
    pub message_signature:
        <<TYPES as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::BuilderSignature,
    pub sender: <TYPES as NodeType>::BuilderSignatureKey,
}

impl<TYPES: NodeType> AvailableBlockHeaderInputV2Legacy<TYPES> {
    pub fn validate_signature(
        &self,
        offered_fee: u64,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) -> bool {
        self.sender
            .validate_builder_signature(&self.message_signature, self.vid_commitment.as_ref())
            && self.sender.validate_fee_signature_with_vid_commitment(
                &self.fee_signature,
                offered_fee,
                metadata,
                &hotshot_types::data::VidCommitment::V0(self.vid_commitment),
            )
    }
}

impl<TYPES: NodeType> AvailableBlockHeaderInputV2<TYPES> {
    pub fn validate_signature(
        &self,
        offered_fee: u64,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) -> bool {
        self.sender
            .validate_fee_signature(&self.fee_signature, offered_fee, metadata)
    }
}
