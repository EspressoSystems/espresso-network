// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! What consensus starts from: the anchor leaf, the certificates and proposals persisted before a
//! restart, and the epoch roots and DRB results that seed membership.

use std::{collections::BTreeMap, sync::Arc};

use versions::Upgrade;

use crate::{
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, ViewNumber},
    drb::{DrbResult, INITIAL_DRB_RESULT},
    epoch_membership::EpochMembershipCoordinator,
    error::HotShotError,
    message::Proposal,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, QuorumCertificate2, UpgradeCertificate,
    },
    traits::{election::Membership, node_implementation::NodeType, states::ValidatedState},
    utils::epoch_from_block_number,
};

#[derive(Clone, Debug, PartialEq)]
pub struct InitializerEpochInfo<TYPES: NodeType> {
    pub epoch: EpochNumber,
    pub drb_result: DrbResult,
    // pub stake_table: Option<StakeTable>, // TODO: Figure out how to connect this up
    pub block_header: Option<TYPES::BlockHeader>,
}

/// The anchor leaf with its validated state, as supplied to [`HotShotInitializer::load`]. The
/// state may be omitted, in which case a sparse state is constructed from the leaf's header.
pub type InitializerAnchor<TYPES> = (
    Leaf2<TYPES>,
    Option<Arc<<TYPES as NodeType>::ValidatedState>>,
);

/// initializer struct for creating starting block
#[derive(Clone, Debug)]
pub struct HotShotInitializer<TYPES: NodeType> {
    /// Instance-level state.
    instance_state: TYPES::InstanceState,

    /// Epoch height
    epoch_height: u64,

    /// the anchor leaf for the hotshot initializer
    anchor_leaf: Leaf2<TYPES>,

    /// ValidatedState for the anchor leaf
    anchor_state: Arc<TYPES::ValidatedState>,

    /// Starting view number that should be equivalent to the view the node shut down with last.
    start_view: ViewNumber,

    /// The view we last performed an action in.  An action is proposing or voting for
    /// either the quorum or DA.
    last_actioned_view: ViewNumber,

    /// Highest QC that was seen, for genesis it's the genesis QC.  It should be for a view greater
    /// than `inner`s view number for the non genesis case because we must have seen higher QCs
    /// to decide on the leaf.
    high_qc: QuorumCertificate2<TYPES>,

    /// Proposals we have sent out to provide to others for catchup
    saved_proposals: BTreeMap<ViewNumber, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,

    /// Previously decided upgrade certificate; this is necessary if an upgrade has happened and we are not restarting with the new version
    decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,

    /// The last formed light client state update certificate if there's any
    state_cert: Option<LightClientStateUpdateCertificateV2<TYPES>>,

    /// Saved epoch information. This must be sorted ascending by epoch.
    start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
}

impl<TYPES: NodeType> HotShotInitializer<TYPES> {
    /// initialize from genesis
    /// # Errors
    /// If we are unable to apply the genesis block to the default state
    pub async fn from_genesis(
        instance_state: TYPES::InstanceState,
        epoch_height: u64,
        start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
        upgrade: Upgrade,
    ) -> Result<Self, HotShotError<TYPES>> {
        let (validated_state, _state_delta) = TYPES::ValidatedState::genesis(&instance_state);
        let high_qc = QuorumCertificate2::genesis(&validated_state, &instance_state, upgrade).await;

        Ok(Self {
            anchor_leaf: Leaf2::genesis(&validated_state, &instance_state, upgrade.base).await,
            anchor_state: Arc::new(validated_state),
            start_view: ViewNumber::new(0),
            last_actioned_view: ViewNumber::new(0),
            saved_proposals: BTreeMap::new(),
            high_qc,
            decided_upgrade_certificate: None,
            instance_state,
            epoch_height,
            state_cert: None,
            start_epoch_info,
        })
    }

    /// Create a `HotShotInitializer` from the given information.
    ///
    /// The anchor's validated state may be supplied by callers that have it (e.g. a full genesis
    /// state); when absent, a sparse state is constructed from the anchor leaf's header.
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        instance_state: TYPES::InstanceState,
        epoch_height: u64,
        start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
        (anchor_leaf, anchor_state): InitializerAnchor<TYPES>,
        start_view: ViewNumber,
        high_qc: QuorumCertificate2<TYPES>,
        last_actioned_view: ViewNumber,
        saved_proposals: BTreeMap<ViewNumber, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,
        decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
        state_cert: Option<LightClientStateUpdateCertificateV2<TYPES>>,
    ) -> Self {
        let anchor_state = anchor_state.unwrap_or_else(|| {
            Arc::new(TYPES::ValidatedState::from_header(
                anchor_leaf.block_header(),
            ))
        });

        Self {
            instance_state,
            epoch_height,
            anchor_leaf,
            anchor_state,
            high_qc,
            start_view,
            last_actioned_view,
            saved_proposals,
            decided_upgrade_certificate,
            state_cert,
            start_epoch_info,
        }
    }

    /// Instance-level state.
    pub fn instance_state(&self) -> &TYPES::InstanceState {
        &self.instance_state
    }

    /// Epoch height.
    pub fn epoch_height(&self) -> u64 {
        self.epoch_height
    }

    /// The anchor (last decided) leaf.
    pub fn anchor_leaf(&self) -> &Leaf2<TYPES> {
        &self.anchor_leaf
    }

    /// ValidatedState for the anchor leaf.
    pub fn anchor_state(&self) -> &Arc<TYPES::ValidatedState> {
        &self.anchor_state
    }

    /// The view to start consensus from.
    pub fn start_view(&self) -> ViewNumber {
        self.start_view
    }

    /// The view we last proposed or voted in.
    pub fn last_actioned_view(&self) -> ViewNumber {
        self.last_actioned_view
    }

    /// Highest QC that was seen.
    pub fn high_qc(&self) -> &QuorumCertificate2<TYPES> {
        &self.high_qc
    }

    /// Proposals we have sent out to provide to others for catchup.
    pub fn saved_proposals(
        &self,
    ) -> &BTreeMap<ViewNumber, Proposal<TYPES, QuorumProposalWrapper<TYPES>>> {
        &self.saved_proposals
    }

    /// The upgrade certificate consensus has decided, if there is one.
    pub fn decided_upgrade_certificate(&self) -> Option<&UpgradeCertificate<TYPES>> {
        self.decided_upgrade_certificate.as_ref()
    }

    /// The last formed light client state update certificate, if there is any.
    pub fn state_cert(&self) -> Option<&LightClientStateUpdateCertificateV2<TYPES>> {
        self.state_cert.as_ref()
    }

    /// The epoch roots and DRB results persistence holds.
    pub fn start_epoch_info(&self) -> &[InitializerEpochInfo<TYPES>] {
        &self.start_epoch_info
    }
}

/// Seed `coordinator` with the first epoch, and with the epoch roots and DRB results in
/// `start_epoch_info`.
pub async fn load_start_epoch_info<TYPES: NodeType>(
    coordinator: &EpochMembershipCoordinator<TYPES>,
    start_epoch_info: &[InitializerEpochInfo<TYPES>],
    epoch_height: u64,
    epoch_start_block: u64,
) {
    let membership = coordinator.membership();
    let first_epoch_number =
        EpochNumber::new(epoch_from_block_number(epoch_start_block, epoch_height));

    tracing::warn!("Calling set_first_epoch for epoch {first_epoch_number}");
    membership.set_first_epoch(first_epoch_number, INITIAL_DRB_RESULT);

    let mut sorted_epoch_info = start_epoch_info.to_vec();
    sorted_epoch_info.sort_by_key(|info| info.epoch);
    for epoch_info in sorted_epoch_info {
        if let Some(block_header) = &epoch_info.block_header {
            tracing::warn!("Calling add_epoch_root for epoch {}", epoch_info.epoch);

            coordinator
                .add_epoch_root(block_header.clone())
                .await
                .unwrap_or_else(|err| {
                    // REVIEW NOTE: Should we panic here? a failure here seems like it should be fatal
                    tracing::error!(
                        "Failed to add epoch root for epoch {}: {err}",
                        epoch_info.epoch
                    );
                });
        }
    }

    for epoch_info in start_epoch_info {
        tracing::warn!("Calling add_drb_result for epoch {}", epoch_info.epoch);
        membership.add_drb_result(epoch_info.epoch, epoch_info.drb_result);
    }
}
