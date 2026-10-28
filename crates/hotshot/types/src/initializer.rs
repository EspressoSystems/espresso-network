// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! What consensus starts from: the anchor leaf, the certificates and proposals persisted before a
//! restart, and the epoch roots and DRB results that seed membership.

use std::{collections::BTreeMap, sync::Arc};

use committable::Committable;
use versions::Upgrade;

use crate::{
    consensus::{VidShares, View, ViewInner},
    data::{EpochNumber, Leaf2, QuorumProposalWrapper, ViewNumber},
    drb::{DrbResult, INITIAL_DRB_RESULT},
    epoch_membership::EpochMembershipCoordinator,
    error::HotShotError,
    message::Proposal,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2, QuorumCertificate2,
        UpgradeCertificate,
    },
    traits::{election::Membership, node_implementation::NodeType, states::ValidatedState},
    utils::{epoch_from_block_number, genesis_epoch_from_version},
};

#[derive(Clone, Debug, PartialEq)]
pub struct InitializerEpochInfo<TYPES: NodeType> {
    pub epoch: EpochNumber,
    pub drb_result: DrbResult,
    // pub stake_table: Option<StakeTable>, // TODO: Figure out how to connect this up
    pub block_header: Option<TYPES::BlockHeader>,
}

/// The anchor leaf with its validated state and optional state delta, as supplied to
/// [`HotShotInitializer::load`]. The state and delta may be omitted, in which case a
/// sparse state is constructed from the leaf's header.
pub type InitializerAnchor<TYPES> = (
    Leaf2<TYPES>,
    Option<Arc<<TYPES as NodeType>::ValidatedState>>,
    Option<Arc<<<TYPES as NodeType>::ValidatedState as ValidatedState<TYPES>>::Delta>>,
);

/// initializer struct for creating starting block
#[derive(Clone, Debug)]
pub struct HotShotInitializer<TYPES: NodeType> {
    /// Instance-level state.
    instance_state: TYPES::InstanceState,

    /// Epoch height
    epoch_height: u64,

    /// Epoch start block
    epoch_start_block: u64,

    /// the anchor leaf for the hotshot initializer
    anchor_leaf: Leaf2<TYPES>,

    /// ValidatedState for the anchor leaf
    anchor_state: Arc<TYPES::ValidatedState>,

    /// ValidatedState::Delta for the anchor leaf, optional.
    anchor_state_delta: Option<Arc<<TYPES::ValidatedState as ValidatedState<TYPES>>::Delta>>,

    /// Starting view number that should be equivalent to the view the node shut down with last.
    start_view: ViewNumber,

    /// The view we last performed an action in.  An action is proposing or voting for
    /// either the quorum or DA.
    last_actioned_view: ViewNumber,

    /// Starting epoch number that should be equivalent to the epoch the node shut down with last.
    start_epoch: Option<EpochNumber>,

    /// Highest QC that was seen, for genesis it's the genesis QC.  It should be for a view greater
    /// than `inner`s view number for the non genesis case because we must have seen higher QCs
    /// to decide on the leaf.
    high_qc: QuorumCertificate2<TYPES>,

    /// Next epoch highest QC that was seen. This is needed to propose during epoch transition after restart.
    next_epoch_high_qc: Option<NextEpochQuorumCertificate2<TYPES>>,

    /// Proposals we have sent out to provide to others for catchup
    saved_proposals: BTreeMap<ViewNumber, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,

    /// Previously decided upgrade certificate; this is necessary if an upgrade has happened and we are not restarting with the new version
    decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,

    /// Undecided leaves that were seen, but not yet decided on.  These allow a restarting node
    /// to vote and propose right away if they didn't miss anything while down.
    undecided_leaves: BTreeMap<ViewNumber, Leaf2<TYPES>>,

    /// Not yet decided state
    undecided_state: BTreeMap<ViewNumber, View<TYPES>>,

    /// Saved VID shares
    saved_vid_shares: VidShares<TYPES>,

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
        epoch_start_block: u64,
        start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
        upgrade: Upgrade,
    ) -> Result<Self, HotShotError<TYPES>> {
        let (validated_state, state_delta) = TYPES::ValidatedState::genesis(&instance_state);
        let high_qc = QuorumCertificate2::genesis(&validated_state, &instance_state, upgrade).await;

        Ok(Self {
            anchor_leaf: Leaf2::genesis(&validated_state, &instance_state, upgrade.base).await,
            anchor_state: Arc::new(validated_state),
            anchor_state_delta: Some(Arc::new(state_delta)),
            start_view: ViewNumber::new(0),
            start_epoch: genesis_epoch_from_version(upgrade.base),
            last_actioned_view: ViewNumber::new(0),
            saved_proposals: BTreeMap::new(),
            high_qc,
            next_epoch_high_qc: None,
            decided_upgrade_certificate: None,
            undecided_leaves: BTreeMap::new(),
            undecided_state: BTreeMap::new(),
            instance_state,
            saved_vid_shares: BTreeMap::new(),
            epoch_height,
            state_cert: None,
            epoch_start_block,
            start_epoch_info,
        })
    }

    /// Use saved proposals to update undecided leaves and state
    #[must_use]
    fn update_undecided(self) -> Self {
        let mut undecided_leaves = self.undecided_leaves.clone();
        let mut undecided_state = self.undecided_state.clone();

        for proposal in self.saved_proposals.values() {
            // skip proposals unless they're newer than the anchor leaf
            if proposal.data.view_number() <= self.anchor_leaf.view_number() {
                continue;
            }

            undecided_leaves.insert(
                proposal.data.view_number(),
                Leaf2::from_quorum_proposal(&proposal.data),
            );
        }

        for leaf in undecided_leaves.values() {
            let view_inner = ViewInner::Leaf {
                leaf: leaf.commit(),
                state: Arc::new(TYPES::ValidatedState::from_header(leaf.block_header())),
                delta: None,
                epoch: leaf.epoch(self.epoch_height),
            };
            let view = View { view_inner };

            undecided_state.insert(leaf.view_number(), view);
        }

        Self {
            undecided_leaves,
            undecided_state,
            ..self
        }
    }

    /// Create a `HotShotInitializer` from the given information.
    ///
    /// The anchor's validated state and state delta may be supplied by callers that have
    /// them (e.g. a full genesis state); when absent, a sparse state is constructed from
    /// the anchor leaf's header. `undecided_leaves` and `undecided_state` are always
    /// populated from `saved_proposals`.
    #[allow(clippy::too_many_arguments)]
    pub fn load(
        instance_state: TYPES::InstanceState,
        epoch_height: u64,
        epoch_start_block: u64,
        start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
        (anchor_leaf, anchor_state, anchor_state_delta): InitializerAnchor<TYPES>,
        (start_view, start_epoch): (ViewNumber, Option<EpochNumber>),
        (high_qc, next_epoch_high_qc): (
            QuorumCertificate2<TYPES>,
            Option<NextEpochQuorumCertificate2<TYPES>>,
        ),
        last_actioned_view: ViewNumber,
        saved_proposals: BTreeMap<ViewNumber, Proposal<TYPES, QuorumProposalWrapper<TYPES>>>,
        saved_vid_shares: VidShares<TYPES>,
        decided_upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
        state_cert: Option<LightClientStateUpdateCertificateV2<TYPES>>,
    ) -> Self {
        let anchor_state = anchor_state.unwrap_or_else(|| {
            Arc::new(TYPES::ValidatedState::from_header(
                anchor_leaf.block_header(),
            ))
        });

        let initializer = Self {
            instance_state,
            epoch_height,
            epoch_start_block,
            anchor_leaf,
            anchor_state,
            anchor_state_delta,
            high_qc,
            start_view,
            start_epoch,
            last_actioned_view,
            saved_proposals,
            saved_vid_shares,
            next_epoch_high_qc,
            decided_upgrade_certificate,
            undecided_leaves: BTreeMap::new(),
            undecided_state: BTreeMap::new(),
            state_cert,
            start_epoch_info,
        };

        initializer.update_undecided()
    }

    /// Instance-level state.
    pub fn instance_state(&self) -> &TYPES::InstanceState {
        &self.instance_state
    }

    /// Epoch height.
    pub fn epoch_height(&self) -> u64 {
        self.epoch_height
    }

    /// Epoch start block.
    pub fn epoch_start_block(&self) -> u64 {
        self.epoch_start_block
    }

    /// The anchor (last decided) leaf.
    pub fn anchor_leaf(&self) -> &Leaf2<TYPES> {
        &self.anchor_leaf
    }

    /// ValidatedState for the anchor leaf.
    pub fn anchor_state(&self) -> &Arc<TYPES::ValidatedState> {
        &self.anchor_state
    }

    /// ValidatedState::Delta for the anchor leaf.
    pub fn anchor_state_delta(
        &self,
    ) -> Option<&Arc<<TYPES::ValidatedState as ValidatedState<TYPES>>::Delta>> {
        self.anchor_state_delta.as_ref()
    }

    /// The view to start consensus from.
    pub fn start_view(&self) -> ViewNumber {
        self.start_view
    }

    /// The epoch to start consensus from.
    pub fn start_epoch(&self) -> Option<EpochNumber> {
        self.start_epoch
    }

    /// The view we last proposed or voted in.
    pub fn last_actioned_view(&self) -> ViewNumber {
        self.last_actioned_view
    }

    /// Highest QC that was seen.
    pub fn high_qc(&self) -> &QuorumCertificate2<TYPES> {
        &self.high_qc
    }

    /// Next epoch highest QC that was seen.
    pub fn next_epoch_high_qc(&self) -> Option<&NextEpochQuorumCertificate2<TYPES>> {
        self.next_epoch_high_qc.as_ref()
    }

    /// Proposals we have sent out to provide to others for catchup.
    pub fn saved_proposals(
        &self,
    ) -> &BTreeMap<ViewNumber, Proposal<TYPES, QuorumProposalWrapper<TYPES>>> {
        &self.saved_proposals
    }

    pub fn saved_vid_shares(&self) -> &VidShares<TYPES> {
        &self.saved_vid_shares
    }

    /// Undecided leaves rebuilt from the saved proposals.
    pub fn undecided_leaves(&self) -> &BTreeMap<ViewNumber, Leaf2<TYPES>> {
        &self.undecided_leaves
    }

    /// Undecided views rebuilt from the saved proposals.
    pub fn undecided_state(&self) -> &BTreeMap<ViewNumber, View<TYPES>> {
        &self.undecided_state
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
