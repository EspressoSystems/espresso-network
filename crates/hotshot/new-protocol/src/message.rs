use committable::{Commitment, Committable};
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, VidDisperseShare2, ViewChangeEvidence2, ViewNumber,
    },
    drb::DrbResult,
    simple_certificate::{
        LightClientStateUpdateCertificateV2, SimpleCertificate, SuccessThreshold,
        UpgradeCertificate, ViewSyncCommitCertificate2, ViewSyncFinalizeCertificate2,
        ViewSyncPreCommitCertificate2,
    },
    simple_vote::{
        HasEpoch, QuorumMarker, SimpleVote, TimeoutVote2, ViewSyncCommitVote2,
        ViewSyncFinalizeVote2, ViewSyncPreCommitVote2,
    },
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    vote::HasViewNumber,
};
use serde::{Deserialize, Serialize};

/// Proposal to append a block.
#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
#[serde(bound(deserialize = ""))]
pub struct Proposal<TYPES: NodeType> {
    /// The block header to append
    pub(crate) block_header: TYPES::BlockHeader,

    /// view number for the proposal
    pub(crate) view_number: ViewNumber,

    /// The epoch number corresponding to the block number. Can be `None` for pre-epoch version.
    pub(crate) epoch: EpochNumber,

    /// certificate that the proposal is chaining from
    pub(crate) justify_qc: Certificate1<TYPES>,

    /// certificate that the proposal is chaining from formed by the next epoch nodes
    pub(crate) next_epoch_justify_qc: Option<Certificate1<TYPES>>,

    /// Possible upgrade certificate, which the leader may optionally attach.
    pub(crate) upgrade_certificate: Option<UpgradeCertificate<TYPES>>,

    /// Possible timeout or view sync certificate. If the `justify_qc` is not for a proposal in the immediately preceding view, then either a timeout or view sync certificate must be attached.
    pub(crate) view_change_evidence: Option<ViewChangeEvidence2<TYPES>>,

    /// The DRB result for the next epoch.
    ///
    /// This is required only for the last block of the epoch. Nodes will verify that it's
    /// consistent with the result from their computations.
    #[serde(with = "serde_bytes")]
    pub(crate) next_drb_result: Option<DrbResult>,

    /// The light client state update certificate for the next epoch.
    /// This is required for the epoch root.
    pub(crate) state_cert: Option<LightClientStateUpdateCertificateV2<TYPES>>,
}

impl<TYPES: NodeType> Committable for Proposal<TYPES> {
    fn commit(&self) -> Commitment<Self> {
        let mut cb = committable::RawCommitmentBuilder::new("Proposal")
            .var_size_bytes(self.block_header.commit().as_ref())
            .u64(*self.view_number)
            .u64(*self.epoch)
            .field("justify qc", self.justify_qc.commit())
            .optional("next_epoch_justify_qc", &self.next_epoch_justify_qc)
            .optional("upgrade certificate", &self.upgrade_certificate);
        match &self.view_change_evidence {
            Some(ViewChangeEvidence2::Timeout(cert)) => {
                cb = cb.field("timeout cert", cert.commit());
            },
            Some(ViewChangeEvidence2::ViewSync(cert)) => {
                cb = cb.field("viewsync cert", cert.commit());
            },
            None => {},
        }
        cb.finalize()
    }
}
impl<TYPES: NodeType> HasViewNumber for Proposal<TYPES> {
    fn view_number(&self) -> ViewNumber {
        self.view_number
    }
}

impl<TYPES: NodeType> HasEpoch for Proposal<TYPES> {
    fn epoch(&self) -> Option<EpochNumber> {
        Some(self.epoch)
    }
}

pub struct ProposalMessage<TYPES: NodeType> {
    pub(crate) proposal: hotshot_types::message::Proposal<TYPES, Proposal<TYPES>>,
    pub(crate) vid_share: hotshot_types::data::VidDisperseShare2<TYPES>,
}

impl<TYPES: NodeType> HasViewNumber for ProposalMessage<TYPES> {
    fn view_number(&self) -> ViewNumber {
        self.proposal.data.view_number
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
/// Data used for a yes vote.
#[serde(bound(deserialize = ""))]
pub struct Vote1Data<TYPES: NodeType> {
    /// Commitment to the leaf
    pub leaf_commit: Commitment<Proposal<TYPES>>,
    /// An epoch to which the data belongs to. Relevant for validating against the correct stake table
    pub epoch: EpochNumber,
    /// Block number of the leaf. It's optional to be compatible with pre-epoch version.
    pub block_number: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
/// Data used for a yes vote.
#[serde(bound(deserialize = ""))]
pub struct Vote2Data<TYPES: NodeType>(Vote1Data<TYPES>);

impl<TYPES: NodeType> Committable for Vote1Data<TYPES> {
    fn commit(&self) -> Commitment<Self> {
        committable::RawCommitmentBuilder::new("Vote1Data")
            .var_size_bytes(self.leaf_commit.as_ref())
            .u64(*self.epoch)
            .u64(self.block_number)
            .constant_str("Vote1")
            .finalize()
    }
}

impl<TYPES: NodeType> Committable for Vote2Data<TYPES> {
    fn commit(&self) -> Commitment<Self> {
        let inner = &self.0;
        committable::RawCommitmentBuilder::new("Vote1Data")
            .var_size_bytes(inner.leaf_commit.as_ref())
            .u64(*inner.epoch)
            .u64(inner.block_number)
            .constant_str("Vote2")
            .finalize()
    }
}

pub struct Vote1<TYPES: NodeType> {
    pub vote: SimpleVote<TYPES, Vote1Data<TYPES>>,
    pub vid_share: VidDisperseShare2<TYPES>,
}

pub type Vote2<TYPES> = SimpleVote<TYPES, Vote2Data<TYPES>>;

pub type Certificate1<TYPES> = SimpleCertificate<TYPES, Vote1Data<TYPES>, SuccessThreshold>;
pub type Certificate2<TYPES> = SimpleCertificate<TYPES, Vote2Data<TYPES>, SuccessThreshold>;

impl<TYPES: NodeType> QuorumMarker for Vote1Data<TYPES> {}
impl<TYPES: NodeType> QuorumMarker for Vote2Data<TYPES> {}

pub enum ConsensusMessage<TYPES: NodeType> {
    Proposal(ProposalMessage<TYPES>),
    Vote1(Vote1<TYPES>),
    Vote2(Vote2<TYPES>),
    Certificate1(Certificate1<TYPES>, TYPES::SignatureKey),
    Certificate2(Certificate2<TYPES>, TYPES::SignatureKey),
    TimeoutVote(TimeoutVote2<TYPES>),
    Transactions(Vec<TYPES::Transaction>),
}

pub enum ViewSyncMessage<TYPES: NodeType> {
    /// Message with a view sync pre-commit vote
    ViewSyncPreCommitVote(ViewSyncPreCommitVote2<TYPES>),

    /// Message with a view sync commit vote
    ViewSyncCommitVote(ViewSyncCommitVote2<TYPES>),

    /// Message with a view sync finalize vote
    ViewSyncFinalizeVote(ViewSyncFinalizeVote2<TYPES>),

    /// Message with a view sync pre-commit certificate
    ViewSyncPreCommitCertificate(ViewSyncPreCommitCertificate2<TYPES>),

    /// Message with a view sync commit certificate
    ViewSyncCommitCertificate(ViewSyncCommitCertificate2<TYPES>),

    /// Message with a view sync finalize certificate
    ViewSyncFinalizeCertificate(ViewSyncFinalizeCertificate2<TYPES>),
}
