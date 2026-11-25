//! Leaves and quorum certificates from any era of the chain.
//!
//! Consensus works only with [`Leaf2`] and [`QuorumCertificate2`]. History can also hold what the
//! chain produced before epochs: the query service, catchup and the light client serve old leaves.
//! The types here hold both eras. They encode exactly like [`Leaf2`], [`QuorumCertificate2`] and
//! [`CertificatePair`], so stored data and other nodes stay compatible. The era comes from the
//! fields:
//!
//! * a QC is pre-epoch when its data has neither an epoch nor a block number;
//! * a leaf is pre-epoch when its justify QC is pre-epoch. These are the leaves before the epoch
//!   upgrade and the first leaf after it.

use std::marker::PhantomData;

use committable::{Commitment, CommitmentBoundsArkless, Committable, RawCommitmentBuilder};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use vbs::version::Version;
use versions::Upgrade;

use super::{
    Leaf, Leaf2, LeafCommitment, LeafEvidence, VidCommitment, ViewChangeEvidence2, ViewNumber,
    genesis_block,
};
use crate::{
    data::EpochNumber,
    drb::DrbResult,
    message::UpgradeLock,
    simple_certificate::{
        CertificatePair, NextEpochQuorumCertificate2, QuorumCertificate, QuorumCertificate2,
        SimpleCertificate, SuccessThreshold, Threshold, TimeoutCertificate3, UpgradeCertificate,
    },
    simple_vote::{
        QuorumData2, QuorumMarker, TimeoutData2, VersionedVoteData, ViewSyncFinalizeData2,
        Voteable, quorum_data_commit, view_and_relay_commit,
    },
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    utils::option_epoch_from_block_number,
};

/// The data a quorum signed before epochs: only the leaf.
///
/// It encodes like a [`QuorumData2`] whose epoch and block number are `null`, and it has the same
/// commitment, so the signatures over that data still verify.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreEpochQuorumData<TYPES: NodeType> {
    /// Commitment to the leaf.
    pub leaf_commit: Commitment<Leaf2<TYPES>>,
}

/// A QC formed before epochs.
pub type PreEpochQuorumCertificate<TYPES> =
    SimpleCertificate<TYPES, PreEpochQuorumData<TYPES>, SuccessThreshold>;

/// A QC from either era.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoricQuorumCertificate<TYPES: NodeType> {
    /// A QC that carries its epoch and block number.
    Epoch(QuorumCertificate2<TYPES>),
    /// A QC formed before epochs.
    PreEpoch(PreEpochQuorumCertificate<TYPES>),
}

/// A leaf whose justify QC was formed before epochs.
///
/// These are the leaves before the epoch upgrade and the first leaf after it. It encodes like
/// [`Leaf2`] and has the same commitment.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound(deserialize = ""))]
pub struct PreEpochLeaf<TYPES: NodeType> {
    view_number: ViewNumber,
    justify_qc: PreEpochQuorumCertificate<TYPES>,
    next_epoch_justify_qc: Option<NextEpochQuorumCertificate2<TYPES>>,
    parent_commitment: Commitment<Leaf2<TYPES>>,
    block_header: TYPES::BlockHeader,
    upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
    block_payload: Option<TYPES::BlockPayload>,
    view_change_evidence: Option<PreEpochViewChangeEvidence<TYPES>>,
    #[serde(with = "serde_bytes")]
    next_drb_result: Option<DrbResult>,
    with_epoch: bool,
}

/// A leaf from either era.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoricLeaf<TYPES: NodeType> {
    /// A leaf whose justify QC carries its epoch.
    Epoch(Leaf2<TYPES>),
    /// A leaf whose justify QC was formed before epochs.
    PreEpoch(PreEpochLeaf<TYPES>),
}

/// The QCs attesting to a leaf, from either era.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoricCertificatePair<TYPES: NodeType> {
    /// QCs that carry their epoch.
    Epoch(CertificatePair<TYPES>),
    /// A QC formed before epochs. There is never a next-epoch QC with it.
    PreEpoch(PreEpochQuorumCertificate<TYPES>),
}

/// The timeout or view sync certificate on a pre-epoch leaf.
///
/// It encodes like [`ViewChangeEvidence2`] and has the same commitment, but the data of a timeout
/// or view sync certificate may lack an epoch, as it did before the epoch upgrade.
#[derive(derive_more::Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound(deserialize = ""))]
pub enum PreEpochViewChangeEvidence<TYPES: NodeType> {
    /// Holds a timeout certificate.
    Timeout(PreEpochTimeoutCertificate<TYPES>),
    /// Holds a view sync finalize certificate.
    ViewSync(PreEpochViewSyncFinalizeCertificate<TYPES>),
    /// Holds a [`TimeoutCertificate3`]. The layout of [`ViewChangeEvidence2`] has it, although no
    /// pre-epoch leaf carries one.
    Timeout3(TimeoutCertificate3<TYPES>),
}

/// A timeout certificate whose data may lack an epoch.
pub type PreEpochTimeoutCertificate<TYPES> =
    SimpleCertificate<TYPES, PreEpochTimeoutData, SuccessThreshold>;

/// A view sync finalize certificate whose data may lack an epoch.
pub type PreEpochViewSyncFinalizeCertificate<TYPES> =
    SimpleCertificate<TYPES, PreEpochViewSyncFinalizeData, SuccessThreshold>;

/// The data of a [`PreEpochTimeoutCertificate`]. It encodes like [`TimeoutData2`] and has the same
/// commitment, but the epoch may be missing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PreEpochTimeoutData {
    /// View the timeout is for.
    pub view: ViewNumber,
    /// The epoch, if the signers had one.
    pub epoch: Option<EpochNumber>,
}

/// The data of a [`PreEpochViewSyncFinalizeCertificate`]. It encodes like
/// [`ViewSyncFinalizeData2`] and has the same commitment, but the epoch may be missing.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PreEpochViewSyncFinalizeData {
    /// The relay this vote is intended for.
    pub relay: u64,
    /// The view number we are trying to sync on.
    pub round: ViewNumber,
    /// The epoch, if the signers had one.
    pub epoch: Option<EpochNumber>,
}

impl<TYPES: NodeType> HistoricQuorumCertificate<TYPES> {
    /// The view of the leaf this QC signs.
    pub fn view_number(&self) -> ViewNumber {
        match self {
            Self::Epoch(qc) => qc.view_number,
            Self::PreEpoch(qc) => qc.view_number,
        }
    }

    /// The leaf this QC signs.
    pub fn leaf_commit(&self) -> Commitment<Leaf2<TYPES>> {
        match self {
            Self::Epoch(qc) => qc.data.leaf_commit,
            Self::PreEpoch(qc) => qc.data.leaf_commit,
        }
    }

    /// The QC, if it carries its epoch.
    pub fn as_qc2(&self) -> Option<&QuorumCertificate2<TYPES>> {
        match self {
            Self::Epoch(qc) => Some(qc),
            Self::PreEpoch(_) => None,
        }
    }

    /// The QC, if it carries its epoch.
    pub fn into_qc2(self) -> Option<QuorumCertificate2<TYPES>> {
        match self {
            Self::Epoch(qc) => Some(qc),
            Self::PreEpoch(_) => None,
        }
    }
}

impl<TYPES: NodeType> PreEpochLeaf<TYPES> {
    /// The genesis leaf of a chain that started before epochs. `version` is passed to the block
    /// header, as in [`Leaf2::genesis`].
    pub async fn genesis(
        validated_state: &TYPES::ValidatedState,
        instance_state: &TYPES::InstanceState,
        version: Version,
    ) -> Self {
        let (block_header, payload) =
            genesis_block::<TYPES>(validated_state, instance_state, version).await;
        let genesis_view = ViewNumber::genesis();
        let null_data = PreEpochQuorumData {
            leaf_commit: Commitment::<Leaf2<TYPES>>::default_commitment_no_preimage(),
        };
        Self {
            view_number: genesis_view,
            justify_qc: SimpleCertificate::new(
                null_data,
                null_data.commit(),
                genesis_view,
                None,
                PhantomData,
            ),
            next_epoch_justify_qc: None,
            parent_commitment: null_data.leaf_commit,
            block_header,
            upgrade_certificate: None,
            block_payload: Some(payload),
            view_change_evidence: None,
            next_drb_result: None,
            with_epoch: false,
        }
    }

    /// The view this leaf was proposed in.
    pub fn view_number(&self) -> ViewNumber {
        self.view_number
    }

    /// Height of this leaf in the chain.
    pub fn height(&self) -> u64 {
        self.block_header.block_number()
    }

    /// The block header contained in this leaf.
    pub fn block_header(&self) -> &TYPES::BlockHeader {
        &self.block_header
    }

    /// The QC linking this leaf to its parent.
    pub fn justify_qc(&self) -> &PreEpochQuorumCertificate<TYPES> {
        &self.justify_qc
    }

    /// Commitment to this leaf's parent.
    pub fn parent_commitment(&self) -> Commitment<Leaf2<TYPES>> {
        self.parent_commitment
    }

    /// The upgrade certificate attached to this leaf's proposal, if any.
    pub fn upgrade_certificate(&self) -> Option<&UpgradeCertificate<TYPES>> {
        self.upgrade_certificate.as_ref()
    }

    /// The next epoch's QC on the parent, as encoded. Only an epoch leaf can need one.
    pub fn next_epoch_justify_qc(&self) -> Option<&NextEpochQuorumCertificate2<TYPES>> {
        self.next_epoch_justify_qc.as_ref()
    }

    /// The timeout or view sync certificate attached to this leaf's proposal, if any.
    pub fn view_change_evidence(&self) -> Option<&PreEpochViewChangeEvidence<TYPES>> {
        self.view_change_evidence.as_ref()
    }

    /// The DRB result for the next epoch, as encoded. Only an epoch leaf can carry one.
    pub fn next_drb_result(&self) -> Option<DrbResult> {
        self.next_drb_result
    }

    /// The `with_epoch` flag as encoded: true only for the first leaf after the epoch upgrade.
    pub fn with_epoch(&self) -> bool {
        self.with_epoch
    }

    /// The block payload, if the leaf holds it.
    pub fn block_payload(&self) -> Option<TYPES::BlockPayload> {
        self.block_payload.clone()
    }

    /// The block payload, if the leaf holds it.
    pub fn block_payload_ref(&self) -> Option<&TYPES::BlockPayload> {
        self.block_payload.as_ref()
    }

    /// Take the block payload out of the leaf.
    pub fn unfill_block_payload(&mut self) -> Option<TYPES::BlockPayload> {
        self.block_payload.take()
    }

    /// A commitment to the block payload.
    pub fn payload_commitment(&self) -> VidCommitment {
        self.block_header.payload_commitment()
    }

    /// The commitment of this leaf: the same value as for the [`Leaf2`] with the same encoding.
    pub fn commit(&self) -> Commitment<Leaf2<TYPES>> {
        LeafCommitment {
            view_number: self.view_number,
            parent_commitment: self.parent_commitment,
            block_header: &self.block_header,
            justify_qc: &self.justify_qc,
            upgrade_certificate: &self.upgrade_certificate,
            with_epoch: self.with_epoch,
            next_epoch_justify_qc: &self.next_epoch_justify_qc,
            next_drb_result: &self.next_drb_result,
            view_change_evidence: &self.view_change_evidence,
        }
        .commit()
    }
}

impl<TYPES: NodeType> PreEpochQuorumCertificate<TYPES> {
    /// The QC on the genesis leaf of a chain that started before epochs.
    pub async fn genesis(
        validated_state: &TYPES::ValidatedState,
        instance_state: &TYPES::InstanceState,
        version: Version,
    ) -> Self {
        let leaf = PreEpochLeaf::genesis(validated_state, instance_state, version).await;
        let data = PreEpochQuorumData {
            leaf_commit: leaf.commit(),
        };
        let genesis_view = ViewNumber::genesis();
        let upgrade_lock = UpgradeLock::<TYPES>::new(Upgrade::trivial(version));
        let vote_commitment: [u8; 32] =
            VersionedVoteData::new_infallible(data, genesis_view, &upgrade_lock)
                .commit()
                .into();
        Self::new(
            data,
            Commitment::from_raw(vote_commitment),
            genesis_view,
            None,
            PhantomData,
        )
    }
}

impl<TYPES: NodeType> HistoricLeaf<TYPES> {
    /// The view this leaf was proposed in.
    pub fn view_number(&self) -> ViewNumber {
        match self {
            Self::Epoch(leaf) => leaf.view_number(),
            Self::PreEpoch(leaf) => leaf.view_number(),
        }
    }

    /// Height of this leaf in the chain.
    pub fn height(&self) -> u64 {
        match self {
            Self::Epoch(leaf) => leaf.height(),
            Self::PreEpoch(leaf) => leaf.height(),
        }
    }

    /// The block header contained in this leaf.
    pub fn block_header(&self) -> &TYPES::BlockHeader {
        match self {
            Self::Epoch(leaf) => leaf.block_header(),
            Self::PreEpoch(leaf) => leaf.block_header(),
        }
    }

    /// The epoch of this leaf's block, or `None` for a block from before epochs.
    ///
    /// The first leaf after the epoch upgrade has an epoch, although its justify QC does not.
    pub fn epoch(&self, epoch_height: u64) -> Option<EpochNumber> {
        match self {
            Self::Epoch(leaf) => leaf.epoch(epoch_height),
            Self::PreEpoch(leaf) => {
                option_epoch_from_block_number(leaf.with_epoch, leaf.height(), epoch_height)
            },
        }
    }

    /// The DRB result for the next epoch, which the last blocks of an epoch carry.
    pub fn next_drb_result(&self) -> Option<DrbResult> {
        match self {
            Self::Epoch(leaf) => leaf.next_drb_result,
            Self::PreEpoch(leaf) => leaf.next_drb_result,
        }
    }

    /// A mutable reference to the block header, like [`Leaf2::block_header_mut`]. Changing the
    /// header changes the leaf's commitment.
    pub fn block_header_mut(&mut self) -> &mut TYPES::BlockHeader {
        match self {
            Self::Epoch(leaf) => leaf.block_header_mut(),
            Self::PreEpoch(leaf) => &mut leaf.block_header,
        }
    }

    /// The QC linking this leaf to its parent.
    pub fn justify_qc(&self) -> HistoricQuorumCertificate<TYPES> {
        match self {
            Self::Epoch(leaf) => leaf.justify_qc().clone().into(),
            Self::PreEpoch(leaf) => leaf.justify_qc().clone().into(),
        }
    }

    /// Commitment to this leaf's parent.
    pub fn parent_commitment(&self) -> Commitment<Leaf2<TYPES>> {
        match self {
            Self::Epoch(leaf) => leaf.parent_commitment(),
            Self::PreEpoch(leaf) => leaf.parent_commitment(),
        }
    }

    /// The upgrade certificate attached to this leaf's proposal, if any.
    pub fn upgrade_certificate(&self) -> Option<&UpgradeCertificate<TYPES>> {
        match self {
            Self::Epoch(leaf) => leaf.upgrade_certificate(),
            Self::PreEpoch(leaf) => leaf.upgrade_certificate(),
        }
    }

    /// The block payload, if the leaf holds it.
    pub fn block_payload(&self) -> Option<TYPES::BlockPayload> {
        match self {
            Self::Epoch(leaf) => leaf.block_payload(),
            Self::PreEpoch(leaf) => leaf.block_payload(),
        }
    }

    /// Take the block payload out of the leaf.
    pub fn unfill_block_payload(&mut self) -> Option<TYPES::BlockPayload> {
        match self {
            Self::Epoch(leaf) => leaf.unfill_block_payload(),
            Self::PreEpoch(leaf) => leaf.unfill_block_payload(),
        }
    }

    /// A commitment to the block payload.
    pub fn payload_commitment(&self) -> VidCommitment {
        match self {
            Self::Epoch(leaf) => leaf.payload_commitment(),
            Self::PreEpoch(leaf) => leaf.payload_commitment(),
        }
    }

    /// The commitment of this leaf: the same value as for the [`Leaf2`] with the same encoding.
    pub fn commit(&self) -> Commitment<Leaf2<TYPES>> {
        match self {
            Self::Epoch(leaf) => leaf.commit(),
            Self::PreEpoch(leaf) => leaf.commit(),
        }
    }

    /// The leaf, if its justify QC carries its epoch.
    pub fn as_leaf2(&self) -> Option<&Leaf2<TYPES>> {
        match self {
            Self::Epoch(leaf) => Some(leaf),
            Self::PreEpoch(_) => None,
        }
    }

    /// The leaf, if its justify QC carries its epoch.
    pub fn into_leaf2(self) -> Option<Leaf2<TYPES>> {
        match self {
            Self::Epoch(leaf) => Some(leaf),
            Self::PreEpoch(_) => None,
        }
    }
}

impl<TYPES: NodeType> HistoricCertificatePair<TYPES> {
    /// The QCs for the parent of `leaf`, taken from the justify QCs in `leaf`.
    pub fn for_parent(leaf: &HistoricLeaf<TYPES>) -> Self {
        match leaf {
            HistoricLeaf::Epoch(leaf) => CertificatePair::for_parent(leaf).into(),
            HistoricLeaf::PreEpoch(leaf) => Self::PreEpoch(leaf.justify_qc().clone()),
        }
    }

    /// The QC signed by the leaf's own quorum.
    pub fn qc(&self) -> HistoricQuorumCertificate<TYPES> {
        match self {
            Self::Epoch(pair) => pair.qc().clone().into(),
            Self::PreEpoch(qc) => qc.clone().into(),
        }
    }

    /// The view of the leaf these QCs sign.
    pub fn view_number(&self) -> ViewNumber {
        match self {
            Self::Epoch(pair) => pair.qc().view_number,
            Self::PreEpoch(qc) => qc.view_number,
        }
    }

    /// The leaf these QCs sign.
    pub fn leaf_commit(&self) -> Commitment<Leaf2<TYPES>> {
        match self {
            Self::Epoch(pair) => pair.leaf_commit(),
            Self::PreEpoch(qc) => qc.data.leaf_commit,
        }
    }

    /// The QCs, if they carry their epoch.
    pub fn as_certificate_pair(&self) -> Option<&CertificatePair<TYPES>> {
        match self {
            Self::Epoch(pair) => Some(pair),
            Self::PreEpoch(_) => None,
        }
    }
}

/// A QC without an epoch and block number becomes [`HistoricQuorumCertificate::PreEpoch`].
impl<TYPES: NodeType> From<QuorumCertificate2<TYPES>> for HistoricQuorumCertificate<TYPES> {
    fn from(qc: QuorumCertificate2<TYPES>) -> Self {
        match pre_epoch_qc(&qc) {
            Some(pre_epoch) => Self::PreEpoch(pre_epoch),
            None => Self::Epoch(qc),
        }
    }
}

impl<TYPES: NodeType> From<PreEpochQuorumCertificate<TYPES>> for HistoricQuorumCertificate<TYPES> {
    fn from(qc: PreEpochQuorumCertificate<TYPES>) -> Self {
        Self::PreEpoch(qc)
    }
}

/// A leaf whose justify QC has no epoch and block number becomes [`HistoricLeaf::PreEpoch`].
impl<TYPES: NodeType> From<Leaf2<TYPES>> for HistoricLeaf<TYPES> {
    fn from(leaf: Leaf2<TYPES>) -> Self {
        let Some(justify_qc) = pre_epoch_qc(&leaf.justify_qc) else {
            return Self::Epoch(leaf);
        };
        let Leaf2 {
            view_number,
            justify_qc: _,
            next_epoch_justify_qc,
            parent_commitment,
            block_header,
            upgrade_certificate,
            block_payload,
            view_change_evidence,
            next_drb_result,
            with_epoch,
        } = leaf;
        Self::PreEpoch(PreEpochLeaf {
            view_number,
            justify_qc,
            next_epoch_justify_qc,
            parent_commitment,
            block_header,
            upgrade_certificate,
            block_payload,
            view_change_evidence: view_change_evidence.map(pre_epoch_evidence),
            next_drb_result,
            with_epoch,
        })
    }
}

impl<TYPES: NodeType> From<PreEpochLeaf<TYPES>> for HistoricLeaf<TYPES> {
    fn from(leaf: PreEpochLeaf<TYPES>) -> Self {
        Self::PreEpoch(leaf)
    }
}

/// A leaf from the first protocol versions, which had no epochs.
impl<TYPES: NodeType> From<Leaf<TYPES>> for PreEpochLeaf<TYPES> {
    fn from(leaf: Leaf<TYPES>) -> Self {
        let parent_commitment: [u8; 32] = leaf.parent_commitment.into();
        Self {
            view_number: leaf.view_number,
            justify_qc: leaf.justify_qc.into(),
            next_epoch_justify_qc: None,
            parent_commitment: Commitment::from_raw(parent_commitment),
            block_header: leaf.block_header,
            upgrade_certificate: leaf.upgrade_certificate,
            block_payload: leaf.block_payload,
            view_change_evidence: None,
            next_drb_result: None,
            with_epoch: false,
        }
    }
}

/// A QC from the first protocol versions, which had no epochs.
impl<TYPES: NodeType> From<QuorumCertificate<TYPES>> for PreEpochQuorumCertificate<TYPES> {
    fn from(qc: QuorumCertificate<TYPES>) -> Self {
        let leaf_commit: [u8; 32] = qc.data.leaf_commit.into();
        let data = PreEpochQuorumData {
            leaf_commit: Commitment::from_raw(leaf_commit),
        };
        with_data(qc, data)
    }
}

/// A pair whose QC has no epoch and block number, and which has no next-epoch QC, becomes
/// [`HistoricCertificatePair::PreEpoch`].
impl<TYPES: NodeType> From<CertificatePair<TYPES>> for HistoricCertificatePair<TYPES> {
    fn from(pair: CertificatePair<TYPES>) -> Self {
        match pre_epoch_qc(pair.qc()) {
            Some(qc) if pair.next_epoch_qc().is_none() => Self::PreEpoch(qc),
            _ => Self::Epoch(pair),
        }
    }
}

/// A single QC, with no next-epoch QC.
impl<TYPES: NodeType> From<HistoricQuorumCertificate<TYPES>> for HistoricCertificatePair<TYPES> {
    fn from(qc: HistoricQuorumCertificate<TYPES>) -> Self {
        match qc {
            HistoricQuorumCertificate::Epoch(qc) => {
                Self::Epoch(CertificatePair::non_epoch_change(qc))
            },
            HistoricQuorumCertificate::PreEpoch(qc) => Self::PreEpoch(qc),
        }
    }
}

/// The same commitment as a [`QuorumData2`] with no epoch and no block number.
impl<TYPES: NodeType> Committable for PreEpochQuorumData<TYPES> {
    fn commit(&self) -> Commitment<Self> {
        quorum_data_commit(&self.leaf_commit, None, None)
    }
}

impl<TYPES: NodeType> QuorumMarker for PreEpochQuorumData<TYPES> {}

/// The same commitment as [`TimeoutData2`], which leaves out the epoch.
impl Committable for PreEpochTimeoutData {
    fn commit(&self) -> Commitment<Self> {
        RawCommitmentBuilder::new("Timeout data")
            .u64(*self.view)
            .finalize()
    }
}

impl QuorumMarker for PreEpochTimeoutData {}

/// The same commitment as [`ViewSyncFinalizeData2`].
impl Committable for PreEpochViewSyncFinalizeData {
    fn commit(&self) -> Commitment<Self> {
        view_and_relay_commit(self.round, self.relay, self.epoch, "View Sync Finalize")
    }
}

impl QuorumMarker for PreEpochViewSyncFinalizeData {}

/// The same fields as for [`ViewChangeEvidence2`].
impl<TYPES: NodeType> LeafEvidence<TYPES> for PreEpochViewChangeEvidence<TYPES> {
    fn add_to_leaf_commitment(
        &self,
        cb: RawCommitmentBuilder<Leaf2<TYPES>>,
    ) -> RawCommitmentBuilder<Leaf2<TYPES>> {
        match self {
            Self::Timeout(cert) => cb.field("timeout cert", cert.commit()),
            Self::ViewSync(cert) => cb.field("viewsync cert", cert.commit()),
            Self::Timeout3(cert) => cb.field("timeout cert v3", cert.commit()),
        }
    }
}

/// Equal when everything but the block payload is equal, like [`Leaf2`].
impl<TYPES: NodeType> PartialEq for PreEpochLeaf<TYPES> {
    fn eq(&self, other: &Self) -> bool {
        let PreEpochLeaf {
            view_number,
            justify_qc,
            next_epoch_justify_qc,
            parent_commitment,
            block_header,
            upgrade_certificate,
            block_payload: _,
            view_change_evidence,
            next_drb_result,
            with_epoch,
        } = self;

        *view_number == other.view_number
            && *justify_qc == other.justify_qc
            && *next_epoch_justify_qc == other.next_epoch_justify_qc
            && *parent_commitment == other.parent_commitment
            && *block_header == other.block_header
            && *upgrade_certificate == other.upgrade_certificate
            && *view_change_evidence == other.view_change_evidence
            && *next_drb_result == other.next_drb_result
            && *with_epoch == other.with_epoch
    }
}

impl<TYPES: NodeType> Eq for PreEpochLeaf<TYPES> {}

/// The layout of [`QuorumData2`], with the epoch and the block number a QC from before epochs
/// lacks. [`PreEpochQuorumData`] is written as it, and a QC from either era is read as it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
struct QuorumDataRaw<TYPES: NodeType> {
    leaf_commit: Commitment<Leaf2<TYPES>>,
    epoch: Option<EpochNumber>,
    block_number: Option<u64>,
}

impl<TYPES: NodeType> Committable for QuorumDataRaw<TYPES> {
    fn commit(&self) -> Commitment<Self> {
        quorum_data_commit(&self.leaf_commit, self.epoch, self.block_number)
    }
}

impl<TYPES: NodeType> QuorumMarker for QuorumDataRaw<TYPES> {}

/// A QC from either era, as encoded.
type QuorumCertificateRaw<TYPES> = SimpleCertificate<TYPES, QuorumDataRaw<TYPES>, SuccessThreshold>;

impl<TYPES: NodeType> Serialize for PreEpochQuorumData<TYPES> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        QuorumDataRaw::<TYPES> {
            leaf_commit: self.leaf_commit,
            epoch: None,
            block_number: None,
        }
        .serialize(serializer)
    }
}

impl<'de, TYPES: NodeType> Deserialize<'de> for PreEpochQuorumData<TYPES> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = QuorumDataRaw::<TYPES>::deserialize(deserializer)?;
        if raw.epoch.is_some() || raw.block_number.is_some() {
            return Err(D::Error::custom(
                "pre-epoch quorum data has an epoch or a block number",
            ));
        }
        Ok(Self {
            leaf_commit: raw.leaf_commit,
        })
    }
}

impl<TYPES: NodeType> Serialize for HistoricQuorumCertificate<TYPES> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Epoch(qc) => qc.serialize(serializer),
            Self::PreEpoch(qc) => qc.serialize(serializer),
        }
    }
}

impl<'de, TYPES: NodeType> Deserialize<'de> for HistoricQuorumCertificate<TYPES> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        QuorumCertificateRaw::deserialize(deserializer).map(historic_qc)
    }
}

impl<TYPES: NodeType> Serialize for HistoricLeaf<TYPES> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Epoch(leaf) => leaf.serialize(serializer),
            Self::PreEpoch(leaf) => leaf.serialize(serializer),
        }
    }
}

impl<'de, TYPES: NodeType> Deserialize<'de> for HistoricLeaf<TYPES> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        LeafRaw::deserialize(deserializer).map(LeafRaw::into_historic)
    }
}

/// The layout of [`Leaf2`], with the QC and evidence of a leaf from either era. A
/// [`HistoricLeaf`] is read as it.
#[derive(Deserialize)]
#[serde(bound(deserialize = ""))]
struct LeafRaw<TYPES: NodeType> {
    view_number: ViewNumber,
    justify_qc: QuorumCertificateRaw<TYPES>,
    next_epoch_justify_qc: Option<NextEpochQuorumCertificate2<TYPES>>,
    parent_commitment: Commitment<Leaf2<TYPES>>,
    block_header: TYPES::BlockHeader,
    upgrade_certificate: Option<UpgradeCertificate<TYPES>>,
    block_payload: Option<TYPES::BlockPayload>,
    view_change_evidence: Option<PreEpochViewChangeEvidence<TYPES>>,
    #[serde(with = "serde_bytes")]
    next_drb_result: Option<DrbResult>,
    with_epoch: bool,
}

impl<TYPES: NodeType> LeafRaw<TYPES> {
    /// The leaf, in the era of its justify QC.
    fn into_historic(self) -> HistoricLeaf<TYPES> {
        let Self {
            view_number,
            justify_qc,
            next_epoch_justify_qc,
            parent_commitment,
            block_header,
            upgrade_certificate,
            block_payload,
            view_change_evidence,
            next_drb_result,
            with_epoch,
        } = self;
        match historic_qc(justify_qc) {
            HistoricQuorumCertificate::PreEpoch(justify_qc) => {
                HistoricLeaf::PreEpoch(PreEpochLeaf {
                    view_number,
                    justify_qc,
                    next_epoch_justify_qc,
                    parent_commitment,
                    block_header,
                    upgrade_certificate,
                    block_payload,
                    view_change_evidence,
                    next_drb_result,
                    with_epoch,
                })
            },
            HistoricQuorumCertificate::Epoch(justify_qc) => HistoricLeaf::Epoch(Leaf2 {
                view_number,
                justify_qc,
                next_epoch_justify_qc,
                parent_commitment,
                block_header,
                upgrade_certificate,
                block_payload,
                view_change_evidence: view_change_evidence.map(evidence2),
                next_drb_result,
                with_epoch,
            }),
        }
    }
}

/// The layout of [`CertificatePair`], which a pre-epoch [`HistoricCertificatePair`] is written as.
#[derive(Serialize)]
#[serde(bound = "")]
struct PreEpochPairRaw<'a, TYPES: NodeType> {
    qc: &'a PreEpochQuorumCertificate<TYPES>,
    next_epoch_qc: Option<&'a NextEpochQuorumCertificate2<TYPES>>,
}

impl<TYPES: NodeType> Serialize for HistoricCertificatePair<TYPES> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Epoch(pair) => pair.serialize(serializer),
            Self::PreEpoch(qc) => PreEpochPairRaw {
                qc,
                next_epoch_qc: None,
            }
            .serialize(serializer),
        }
    }
}

impl<'de, TYPES: NodeType> Deserialize<'de> for HistoricCertificatePair<TYPES> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        CertificatePairRaw::deserialize(deserializer).map(CertificatePairRaw::into_historic)
    }
}

/// The layout of [`CertificatePair`], with a QC from either era. A [`HistoricCertificatePair`] is
/// read as it.
#[derive(Deserialize)]
#[serde(bound(deserialize = ""))]
struct CertificatePairRaw<TYPES: NodeType> {
    qc: QuorumCertificateRaw<TYPES>,
    next_epoch_qc: Option<NextEpochQuorumCertificate2<TYPES>>,
}

impl<TYPES: NodeType> CertificatePairRaw<TYPES> {
    /// The QCs, in the era of `qc`.
    fn into_historic(self) -> HistoricCertificatePair<TYPES> {
        match self.next_epoch_qc {
            None => historic_qc(self.qc).into(),
            Some(next_epoch_qc) => HistoricCertificatePair::Epoch(CertificatePair::new(
                qc2(self.qc),
                Some(next_epoch_qc),
            )),
        }
    }
}

/// `qc` in the era its data shows: pre-epoch when it has neither an epoch nor a block number.
fn historic_qc<TYPES: NodeType>(
    qc: QuorumCertificateRaw<TYPES>,
) -> HistoricQuorumCertificate<TYPES> {
    let QuorumDataRaw {
        leaf_commit,
        epoch: None,
        block_number: None,
    } = qc.data
    else {
        return HistoricQuorumCertificate::Epoch(qc2(qc));
    };
    HistoricQuorumCertificate::PreEpoch(with_data(qc, PreEpochQuorumData { leaf_commit }))
}

/// `qc` as a [`QuorumCertificate2`].
fn qc2<TYPES: NodeType>(qc: QuorumCertificateRaw<TYPES>) -> QuorumCertificate2<TYPES> {
    let QuorumDataRaw {
        leaf_commit,
        epoch,
        block_number,
    } = qc.data;
    with_data(
        qc,
        QuorumData2 {
            leaf_commit,
            epoch,
            block_number,
        },
    )
}

/// `evidence` as a [`ViewChangeEvidence2`].
fn evidence2<TYPES: NodeType>(
    evidence: PreEpochViewChangeEvidence<TYPES>,
) -> ViewChangeEvidence2<TYPES> {
    match evidence {
        PreEpochViewChangeEvidence::Timeout(cert) => {
            let data = TimeoutData2 {
                view: cert.data.view,
                epoch: cert.data.epoch,
            };
            ViewChangeEvidence2::Timeout(with_data(cert, data))
        },
        PreEpochViewChangeEvidence::ViewSync(cert) => {
            let data = ViewSyncFinalizeData2 {
                relay: cert.data.relay,
                round: cert.data.round,
                epoch: cert.data.epoch,
            };
            ViewChangeEvidence2::ViewSync(with_data(cert, data))
        },
        PreEpochViewChangeEvidence::Timeout3(cert) => ViewChangeEvidence2::Timeout3(cert),
    }
}

/// `evidence` as a [`PreEpochViewChangeEvidence`].
fn pre_epoch_evidence<TYPES: NodeType>(
    evidence: ViewChangeEvidence2<TYPES>,
) -> PreEpochViewChangeEvidence<TYPES> {
    match evidence {
        ViewChangeEvidence2::Timeout(cert) => {
            let data = PreEpochTimeoutData {
                view: cert.data.view,
                epoch: cert.data.epoch,
            };
            PreEpochViewChangeEvidence::Timeout(with_data(cert, data))
        },
        ViewChangeEvidence2::ViewSync(cert) => {
            let data = PreEpochViewSyncFinalizeData {
                relay: cert.data.relay,
                round: cert.data.round,
                epoch: cert.data.epoch,
            };
            PreEpochViewChangeEvidence::ViewSync(with_data(cert, data))
        },
        ViewChangeEvidence2::Timeout3(cert) => PreEpochViewChangeEvidence::Timeout3(cert),
    }
}

/// `cert` over `data`, with the same vote commitment, view and signatures. `data` must have the
/// commitment the signers signed.
fn with_data<TYPES: NodeType, V: Voteable<TYPES>, W: Voteable<TYPES>, T: Threshold<TYPES>>(
    cert: SimpleCertificate<TYPES, V, T>,
    data: W,
) -> SimpleCertificate<TYPES, W, T> {
    let vote_commitment: [u8; 32] = cert.vote_commitment().into();
    SimpleCertificate::new(
        data,
        Commitment::from_raw(vote_commitment),
        cert.view_number,
        cert.signatures,
        PhantomData,
    )
}

/// `qc` as a pre-epoch QC, if it has neither an epoch nor a block number.
fn pre_epoch_qc<TYPES: NodeType>(
    qc: &QuorumCertificate2<TYPES>,
) -> Option<PreEpochQuorumCertificate<TYPES>> {
    let QuorumData2 {
        leaf_commit,
        epoch: None,
        block_number: None,
    } = qc.data
    else {
        return None;
    };
    let vote_commitment: [u8; 32] = qc.vote_commitment().into();
    Some(SimpleCertificate::new(
        PreEpochQuorumData { leaf_commit },
        Commitment::from_raw(vote_commitment),
        qc.view_number,
        qc.signatures.clone(),
        PhantomData,
    ))
}
