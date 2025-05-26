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

/// legacy version of the AvailableBlockHeaderInputV2 type, used on git tag `20250228-patch3`
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

/// either version of the AvailableBlockHeaderInputV2 type. Note that we try to deserialize legacy first,
/// as that has extra fields that are not present in the current version. When presented with a legacy
/// input, we'll first try to validate its signature as the current version.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AvailableBlockHeaderInputV2Either<TYPES: NodeType> {
    Current(AvailableBlockHeaderInputV2<TYPES>),
    Legacy(AvailableBlockHeaderInputV2Legacy<TYPES>),
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

impl<TYPES: NodeType> AvailableBlockHeaderInputV2Either<TYPES> {
    pub fn validate_signature_and_get_input(
        &self,
        offered_fee: u64,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
    ) -> Option<AvailableBlockHeaderInputV2<TYPES>> {
        match self {
            AvailableBlockHeaderInputV2Either::Legacy(legacy) => {
                // Try to validate this as a current signature first, then fall back to legacy validation
                // Note that "legacy" as a variable name might be misleading here, as in the first case
                // we're treating the 'legacy' struct as 'current' with extra fields. This mirrors the previous
                // behavior of the code.
                if legacy.sender.validate_fee_signature(
                    &legacy.fee_signature,
                    offered_fee,
                    metadata,
                ) || legacy.validate_signature(offered_fee, metadata)
                {
                    Some(AvailableBlockHeaderInputV2 {
                        fee_signature: legacy.fee_signature.clone(),
                        sender: legacy.sender.clone(),
                    })
                } else {
                    None
                }
            },
            AvailableBlockHeaderInputV2Either::Current(current) => {
                if current.validate_signature(offered_fee, metadata) {
                    Some(current.clone())
                } else {
                    None
                }
            },
        }
    }
}
