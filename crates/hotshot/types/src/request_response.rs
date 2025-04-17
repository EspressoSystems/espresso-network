// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types for the request/response implementations. This module incorporates all
//! of the shared types for all of the network backends.

use committable::{Committable, RawCommitmentBuilder};
use serde::{Deserialize, Serialize};

use crate::traits::{node_implementation::NodeType, signature_key::SignatureKey};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
/// A signed request for a proposal.
pub struct ProposalRequestPayload<TYPES: NodeType> {
    /// The view number that we're requesting a proposal for.
    pub view_number: TYPES::View,

    /// Our public key. The ensures that the recipient can reply to
    /// us directly.
    pub key: TYPES::SignatureKey,
}

impl<TYPES: NodeType> Committable for ProposalRequestPayload<TYPES> {
    fn commit(&self) -> committable::Commitment<Self> {
        RawCommitmentBuilder::new("signed proposal request commitment")
            .u64_field("view number", *self.view_number)
            .var_size_bytes(&self.key.to_bytes())
            .finalize()
    }
}
