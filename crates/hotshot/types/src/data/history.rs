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

use committable::{Commitment, Committable, RawCommitmentBuilder};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use super::{Leaf2, LeafCommitment, VidCommitment, ViewChangeEvidence2, ViewNumber};
use crate::{
    data::EpochNumber,
    drb::DrbResult,
    simple_certificate::{
        CertificatePair, NextEpochQuorumCertificate2, QuorumCertificate2, SimpleCertificate,
        SuccessThreshold, UpgradeCertificate,
    },
    simple_vote::{QuorumData2, QuorumMarker},
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
    view_change_evidence: Option<ViewChangeEvidence2<TYPES>>,
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
    pub fn view_change_evidence(&self) -> Option<&ViewChangeEvidence2<TYPES>> {
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
            view_change_evidence,
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
        RawCommitmentBuilder::new("Quorum data")
            .var_size_bytes(self.leaf_commit.as_ref())
            .finalize()
    }
}

impl<TYPES: NodeType> QuorumMarker for PreEpochQuorumData<TYPES> {}

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

/// The layout of [`QuorumData2`], which [`PreEpochQuorumData`] is read from and written as.
#[derive(Serialize, Deserialize)]
#[serde(bound = "")]
struct QuorumDataRaw<TYPES: NodeType> {
    leaf_commit: Commitment<Leaf2<TYPES>>,
    epoch: Option<EpochNumber>,
    block_number: Option<u64>,
}

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
        QuorumCertificate2::deserialize(deserializer).map(Self::from)
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
        Leaf2::deserialize(deserializer).map(Self::from)
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
        CertificatePair::deserialize(deserializer).map(Self::from)
    }
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
