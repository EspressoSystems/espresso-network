// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Decided leaf information and persisted consensus actions.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    data::{Leaf2, VidDisperseShare},
    simple_certificate::LightClientStateUpdateCertificateV2,
    traits::{ValidatedState, node_implementation::NodeType},
};

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
    pub state_cert: Option<LightClientStateUpdateCertificateV2<TYPES>>,
}

impl<TYPES: NodeType> LeafInfo<TYPES> {
    /// Constructor.
    pub fn new(
        leaf: Leaf2<TYPES>,
        state: Arc<<TYPES as NodeType>::ValidatedState>,
        delta: Option<Arc<<<TYPES as NodeType>::ValidatedState as ValidatedState<TYPES>>::Delta>>,
        vid_share: Option<VidDisperseShare<TYPES>>,
        state_cert: Option<LightClientStateUpdateCertificateV2<TYPES>>,
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
/// A list of actions that we track for nodes
pub enum HotShotAction {
    /// A quorum vote was sent
    Vote,
    /// A timeout vote was sent
    TimeoutVote,
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
