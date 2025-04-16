// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Events that a `HotShot` instance can emit

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    data::{DaProposal2, Leaf2, QuorumProposalWrapper, UpgradeProposal, VidDisperseShare},
    error::HotShotError,
    message::Proposal,
    simple_certificate::{LightClientStateUpdateCertificate, QuorumCertificate2},
    traits::{node_implementation::NodeType, ValidatedState},
};

/// A status event emitted by a `HotShot` instance
///
/// This includes some metadata, such as the stage and view number that the event was generated in,
/// as well as an inner [`EventType`] describing the event proper.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "TYPES: NodeType"))]
pub struct Event<TYPES: NodeType> {
    /// The view number that this event originates from
    pub view_number: TYPES::View,
    /// The underlying event
    pub event: EventType<TYPES>,
}

/// Decided leaf with the corresponding state and VID info.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "TYPES: NodeType"))]
pub struct LeafInfo<TYPES: NodeType> {
    /// Decided leaf.
    pub leaf: Leaf2<TYPES>,
    /// Validated state.
    pub state: Arc<<TYPES as NodeType>::ValidatedState>,
    /// Optional application-specific state delta.
    pub delta: Option<Arc<<<TYPES as NodeType>::ValidatedState as ValidatedState<TYPES>>::Delta>>,
    /// Optional VID share data.
    pub vid_share: Option<VidDisperseShare<TYPES>>,
    /// Optional light client state update certificate.
    pub state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,
}

impl<TYPES: NodeType> LeafInfo<TYPES> {
    /// Constructor.
    pub fn new(
        leaf: Leaf2<TYPES>,
        state: Arc<<TYPES as NodeType>::ValidatedState>,
        delta: Option<Arc<<<TYPES as NodeType>::ValidatedState as ValidatedState<TYPES>>::Delta>>,
        vid_share: Option<VidDisperseShare<TYPES>>,
        state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,
    ) -> Self {
        Self {
            leaf,
            state,
            delta,
            vid_share,
            state_cert,
        }
    }
}

/// The chain of decided leaves with its corresponding state and VID info.
pub type LeafChain<TYPES> = Vec<LeafInfo<TYPES>>;

/// Utilities for converting between HotShotError and a string.
pub mod error_adaptor {
    use serde::{de::Deserializer, ser::Serializer};

    use super::{Arc, Deserialize, HotShotError, NodeType};

    /// Convert a HotShotError into a string
    ///
    /// # Errors
    /// Returns `Err` if the serializer fails.
    pub fn serialize<S: Serializer, TYPES: NodeType>(
        elem: &Arc<HotShotError<TYPES>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("{elem}"))
    }

    /// Convert a string into a HotShotError
    ///
    /// # Errors
    /// Returns `Err` if the string cannot be deserialized.
    pub fn deserialize<'de, D: Deserializer<'de>, TYPES: NodeType>(
        deserializer: D,
    ) -> Result<Arc<HotShotError<TYPES>>, D::Error> {
        let str = String::deserialize(deserializer)?;
        Ok(Arc::new(HotShotError::FailedToDeserialize(str)))
    }
}

/// The type and contents of a status event emitted by a `HotShot` instance
///
/// This enum does not include metadata shared among all variants, such as the stage and view
/// number, and is thus always returned wrapped in an [`Event`].
#[non_exhaustive]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = "TYPES: NodeType"))]
#[allow(clippy::large_enum_variant)]
pub enum EventType<TYPES: NodeType> {
    /// A view encountered an error and was interrupted
    Error {
        /// The underlying error
        #[serde(with = "error_adaptor")]
        error: Arc<HotShotError<TYPES>>,
    },
    /// A new decision event was issued
    Decide {
        /// The chain of Leaves that were committed by this decision
        ///
        /// This list is sorted in reverse view number order, with the newest (highest view number)
        /// block first in the list.
        ///
        /// This list may be incomplete if the node is currently performing catchup.
        /// Vid Info for a decided view may be missing if this node never saw it's share.
        leaf_chain: Arc<LeafChain<TYPES>>,
        /// The QC signing the most recent leaf in `leaf_chain`.
        ///
        /// Note that the QC for each additional leaf in the chain can be obtained from the leaf
        /// before it using
        qc: Arc<QuorumCertificate2<TYPES>>,
        /// Optional information of the number of transactions in the block, for logging purposes.
        block_size: Option<u64>,
    },
    /// A replica task was canceled by a timeout interrupt
    ReplicaViewTimeout {
        /// The view that timed out
        view_number: TYPES::View,
    },
    /// The view has finished.  If values were decided on, a `Decide` event will also be emitted.
    ViewFinished {
        /// The view number that has just finished
        view_number: TYPES::View,
    },
    /// The view timed out
    ViewTimeout {
        /// The view that timed out
        view_number: TYPES::View,
    },
    /// New transactions were received from the network
    /// or submitted to the network by us
    Transactions {
        /// The list of transactions
        transactions: Vec<TYPES::Transaction>,
    },
    /// DA proposal was received from the network
    /// or submitted to the network by us
    DaProposal {
        /// Contents of the proposal
        proposal: Proposal<TYPES, DaProposal2<TYPES>>,
        /// Public key of the leader submitting the proposal
        sender: TYPES::SignatureKey,
    },
    /// Quorum proposal was received from the network
    /// or submitted to the network by us
    QuorumProposal {
        /// Contents of the proposal
        proposal: Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
        /// Public key of the leader submitting the proposal
        sender: TYPES::SignatureKey,
    },
    /// Upgrade proposal was received from the network
    /// or submitted to the network by us
    UpgradeProposal {
        /// Contents of the proposal
        proposal: Proposal<TYPES, UpgradeProposal<TYPES>>,
        /// Public key of the leader submitting the proposal
        sender: TYPES::SignatureKey,
    },

    /// A message destined for external listeners was received
    ExternalMessageReceived {
        /// Public Key of the message sender
        sender: TYPES::SignatureKey,
        /// Serialized data of the message
        data: Vec<u8>,
    },
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
/// A list of actions that we track for nodes
pub enum HotShotAction {
    /// A quorum vote was sent
    Vote,
    /// View Sync Vote
    ViewSyncVote,
    /// A quorum proposal was sent
    Propose,
    /// DA proposal was sent
    DaPropose,
    /// DA vote was sent
    DaVote,
    /// DA certificate was sent
    DaCert,
    /// VID shares were sent
    VidDisperse,
    /// An upgrade vote was sent
    UpgradeVote,
    /// An upgrade proposal was sent
    UpgradePropose,
}
