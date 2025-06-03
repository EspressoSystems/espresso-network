// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Network message types
//!
//! This module contains types used to represent the various types of messages that
//! `HotShot` nodes can send among themselves.

use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    sync::Arc,
};

use async_lock::RwLock;
use committable::Committable;
use hotshot_utils::anytrace::*;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use vbs::{
    version::{StaticVersionType, Version},
    BinarySerializer, Serializer,
};

use crate::{
    data::{
        vid_disperse::{ADVZDisperseShare, VidDisperseShare2},
        DaProposal, DaProposal2, Leaf, Leaf2, QuorumProposal, QuorumProposal2,
        QuorumProposalWrapper, UpgradeProposal,
    },
    epoch_membership::EpochMembership,
    request_response::ProposalRequestPayload,
    simple_certificate::{
        DaCertificate, DaCertificate2, EpochRootQuorumCertificate, NextEpochQuorumCertificate2,
        QuorumCertificate2, UpgradeCertificate, ViewSyncCommitCertificate,
        ViewSyncCommitCertificate2, ViewSyncFinalizeCertificate, ViewSyncFinalizeCertificate2,
        ViewSyncPreCommitCertificate, ViewSyncPreCommitCertificate2,
    },
    simple_vote::{
        DaVote, DaVote2, EpochRootQuorumVote, HasEpoch, QuorumVote, QuorumVote2, TimeoutVote,
        TimeoutVote2, UpgradeVote, ViewSyncCommitVote, ViewSyncCommitVote2, ViewSyncFinalizeVote,
        ViewSyncFinalizeVote2, ViewSyncPreCommitVote, ViewSyncPreCommitVote2,
    },
    traits::{
        election::Membership,
        network::{DataRequest, ResponseMessage, ViewMessage},
        node_implementation::{ConsensusTime, NodeType, Versions},
        signature_key::SignatureKey,
    },
    utils::mnemonic,
    vote::HasViewNumber,
};

/// Incoming message
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = "", serialize = ""))]
pub struct Message<TYPES: NodeType> {
    /// The sender of this message
    pub sender: TYPES::SignatureKey,

    /// The message kind
    pub kind: MessageKind<TYPES>,
}

impl<TYPES: NodeType> fmt::Debug for Message<TYPES> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Message")
            .field("sender", &mnemonic(&self.sender))
            .field("kind", &self.kind)
            .finish()
    }
}

impl<TYPES: NodeType> HasViewNumber<TYPES> for Message<TYPES> {
    /// get the view number out of a message
    fn view_number(&self) -> TYPES::View {
        self.kind.view_number()
    }
}

/// A wrapper type for implementing `PassType` on a vector of `Message`.
#[derive(Clone, Debug)]
pub struct Messages<TYPES: NodeType>(pub Vec<Message<TYPES>>);

/// A message type agnostic description of a message's purpose
#[derive(PartialEq, Copy, Clone)]
pub enum MessagePurpose {
    /// Message with a [quorum/DA] proposal.
    Proposal,
    /// Message with most recent [quorum/DA] proposal the server has
    LatestProposal,
    /// Message with most recent view sync certificate the server has
    LatestViewSyncCertificate,
    /// Message with a quorum vote.
    Vote,
    /// Message with a view sync vote.
    ViewSyncVote,
    /// Message with a view sync certificate.
    ViewSyncCertificate,
    /// Message with a DAC.
    DaCertificate,
    /// Message for internal use
    Internal,
    /// Data message
    Data,
    /// VID disperse, like [`Proposal`].
    VidDisperse,
    /// Message with an upgrade proposal.
    UpgradeProposal,
    /// Upgrade vote.
    UpgradeVote,
    /// A message to be passed through to external listeners
    External,
}

// TODO (da) make it more customized to the consensus layer, maybe separating the specific message
// data from the kind enum.
/// Enum representation of any message type
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "", serialize = ""))]
pub enum MessageKind<TYPES: NodeType> {
    /// Messages related to the consensus protocol
    Consensus(SequencingMessage<TYPES>),
    /// Messages relating to sharing data between nodes
    Data(DataMessage<TYPES>),
    /// A (still serialized) message to be passed through to external listeners
    External(Vec<u8>),
}

/// List of keys to send a message to, or broadcast to all known keys
pub enum RecipientList<K: SignatureKey> {
    /// Broadcast to all
    Broadcast,
    /// Send a message directly to a key
    Direct(K),
    /// Send a message directly to many keys
    Many(Vec<K>),
}

impl<TYPES: NodeType> MessageKind<TYPES> {
    // Can't implement `From<I::ConsensusMessage>` directly due to potential conflict with
    // `From<DataMessage>`.
    /// Construct a [`MessageKind`] from [`SequencingMessage`].
    pub fn from_consensus_message(m: SequencingMessage<TYPES>) -> Self {
        Self::Consensus(m)
    }
}

impl<TYPES: NodeType> From<DataMessage<TYPES>> for MessageKind<TYPES> {
    fn from(m: DataMessage<TYPES>) -> Self {
        Self::Data(m)
    }
}

impl<TYPES: NodeType> ViewMessage<TYPES> for MessageKind<TYPES> {
    fn view_number(&self) -> TYPES::View {
        match &self {
            MessageKind::Consensus(message) => message.view_number(),
            MessageKind::Data(DataMessage::SubmitTransaction(_, v)) => *v,
            MessageKind::Data(DataMessage::RequestData(msg)) => msg.view,
            MessageKind::Data(DataMessage::DataResponse(msg)) => match msg {
                ResponseMessage::Found(m) => m.view_number(),
                ResponseMessage::NotFound | ResponseMessage::Denied => TYPES::View::new(1),
            },
            MessageKind::External(_) => TYPES::View::new(1),
        }
    }
}

impl<TYPES: NodeType> HasEpoch<TYPES> for MessageKind<TYPES> {
    fn epoch(&self) -> Option<TYPES::Epoch> {
        match &self {
            MessageKind::Consensus(message) => message.epoch_number(),
            MessageKind::Data(DataMessage::SubmitTransaction(..) | DataMessage::RequestData(_))
            | MessageKind::External(_) => None,
            MessageKind::Data(DataMessage::DataResponse(msg)) => match msg {
                ResponseMessage::Found(m) => m.epoch_number(),
                ResponseMessage::NotFound | ResponseMessage::Denied => None,
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = "", serialize = ""))]
/// Messages related to both validating and sequencing consensus.
pub enum GeneralConsensusMessage<TYPES: NodeType> {
    /// Message with a quorum proposal.
    Proposal(Proposal<TYPES, QuorumProposal<TYPES>>),

    /// Message with a quorum vote.
    Vote(QuorumVote<TYPES>),

    /// Message with a view sync pre-commit vote
    ViewSyncPreCommitVote(ViewSyncPreCommitVote<TYPES>),

    /// Message with a view sync commit vote
    ViewSyncCommitVote(ViewSyncCommitVote<TYPES>),

    /// Message with a view sync finalize vote
    ViewSyncFinalizeVote(ViewSyncFinalizeVote<TYPES>),

    /// Message with a view sync pre-commit certificate
    ViewSyncPreCommitCertificate(ViewSyncPreCommitCertificate<TYPES>),

    /// Message with a view sync commit certificate
    ViewSyncCommitCertificate(ViewSyncCommitCertificate<TYPES>),

    /// Message with a view sync finalize certificate
    ViewSyncFinalizeCertificate(ViewSyncFinalizeCertificate<TYPES>),

    /// Message with a Timeout vote
    TimeoutVote(TimeoutVote<TYPES>),

    /// Message with an upgrade proposal
    UpgradeProposal(Proposal<TYPES, UpgradeProposal<TYPES>>),

    /// Message with an upgrade vote
    UpgradeVote(UpgradeVote<TYPES>),

    /// A peer node needs a proposal from the leader.
    ProposalRequested(
        ProposalRequestPayload<TYPES>,
        <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ),

    /// A replica has responded with a valid proposal.
    ProposalResponse(Proposal<TYPES, QuorumProposal<TYPES>>),

    /// Message with a quorum proposal.
    Proposal2(Proposal<TYPES, QuorumProposal2<TYPES>>),

    /// Message with a quorum vote.
    Vote2(QuorumVote2<TYPES>),

    /// Message with an epoch root quorum vote.
    EpochRootQuorumVote(EpochRootQuorumVote<TYPES>),

    /// A replica has responded with a valid proposal.
    ProposalResponse2(Proposal<TYPES, QuorumProposal2<TYPES>>),

    /// Message for the next leader containing our highest QC
    HighQc(
        QuorumCertificate2<TYPES>,
        Option<NextEpochQuorumCertificate2<TYPES>>,
    ),

    /// Message containing the highest QC and the next epoch QC
    ExtendedQc(
        QuorumCertificate2<TYPES>,
        NextEpochQuorumCertificate2<TYPES>,
    ),

    /// Message for the next leader containing the epoch root QC
    EpochRootQc(EpochRootQuorumCertificate<TYPES>),

    /// Message with a view sync pre-commit vote
    ViewSyncPreCommitVote2(ViewSyncPreCommitVote2<TYPES>),

    /// Message with a view sync commit vote
    ViewSyncCommitVote2(ViewSyncCommitVote2<TYPES>),

    /// Message with a view sync finalize vote
    ViewSyncFinalizeVote2(ViewSyncFinalizeVote2<TYPES>),

    /// Message with a view sync pre-commit certificate
    ViewSyncPreCommitCertificate2(ViewSyncPreCommitCertificate2<TYPES>),

    /// Message with a view sync commit certificate
    ViewSyncCommitCertificate2(ViewSyncCommitCertificate2<TYPES>),

    /// Message with a view sync finalize certificate
    ViewSyncFinalizeCertificate2(ViewSyncFinalizeCertificate2<TYPES>),

    /// Message with a Timeout vote
    TimeoutVote2(TimeoutVote2<TYPES>),
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Hash, Eq)]
#[serde(bound(deserialize = "", serialize = ""))]
/// Messages related to the sequencing consensus protocol for the DA committee.
pub enum DaConsensusMessage<TYPES: NodeType> {
    /// Proposal for data availability committee
    DaProposal(Proposal<TYPES, DaProposal<TYPES>>),

    /// vote for data availability committee
    DaVote(DaVote<TYPES>),

    /// Certificate data is available
    DaCertificate(DaCertificate<TYPES>),

    /// Initiate VID dispersal.
    ///
    /// Like [`DaProposal`]. Use `Msg` suffix to distinguish from `VidDisperse`.
    VidDisperseMsg(Proposal<TYPES, ADVZDisperseShare<TYPES>>),

    /// Proposal for data availability committee
    DaProposal2(Proposal<TYPES, DaProposal2<TYPES>>),

    /// vote for data availability committee
    DaVote2(DaVote2<TYPES>),

    /// Certificate data is available
    DaCertificate2(DaCertificate2<TYPES>),

    /// Initiate VID dispersal.
    ///
    /// Like [`DaProposal`]. Use `Msg` suffix to distinguish from `VidDisperse`.
    VidDisperseMsg2(Proposal<TYPES, VidDisperseShare2<TYPES>>),
}

/// Messages for sequencing consensus.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = "", serialize = ""))]
#[allow(clippy::large_enum_variant)]
pub enum SequencingMessage<TYPES: NodeType> {
    /// Messages related to validating and sequencing consensus
    General(GeneralConsensusMessage<TYPES>),

    /// Messages related to the sequencing consensus protocol for the DA committee.
    Da(DaConsensusMessage<TYPES>),
}

impl<TYPES: NodeType> SequencingMessage<TYPES> {
    /// Get the view number this message relates to
    fn view_number(&self) -> TYPES::View {
        match &self {
            SequencingMessage::General(general_message) => {
                match general_message {
                    GeneralConsensusMessage::Proposal(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.view_number()
                    },
                    GeneralConsensusMessage::Proposal2(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.view_number()
                    },
                    GeneralConsensusMessage::ProposalRequested(req, _) => req.view_number,
                    GeneralConsensusMessage::ProposalResponse(proposal) => {
                        proposal.data.view_number()
                    },
                    GeneralConsensusMessage::ProposalResponse2(proposal) => {
                        proposal.data.view_number()
                    },
                    GeneralConsensusMessage::Vote(vote_message) => vote_message.view_number(),
                    GeneralConsensusMessage::Vote2(vote_message) => vote_message.view_number(),
                    GeneralConsensusMessage::TimeoutVote(message) => message.view_number(),
                    GeneralConsensusMessage::ViewSyncPreCommitVote(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::ViewSyncCommitVote(message) => message.view_number(),
                    GeneralConsensusMessage::ViewSyncFinalizeVote(message) => message.view_number(),
                    GeneralConsensusMessage::ViewSyncPreCommitCertificate(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::ViewSyncCommitCertificate(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::ViewSyncFinalizeCertificate(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::TimeoutVote2(message) => message.view_number(),
                    GeneralConsensusMessage::ViewSyncPreCommitVote2(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::ViewSyncCommitVote2(message) => message.view_number(),
                    GeneralConsensusMessage::ViewSyncFinalizeVote2(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::ViewSyncPreCommitCertificate2(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::ViewSyncCommitCertificate2(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::ViewSyncFinalizeCertificate2(message) => {
                        message.view_number()
                    },
                    GeneralConsensusMessage::UpgradeProposal(message) => message.data.view_number(),
                    GeneralConsensusMessage::UpgradeVote(message) => message.view_number(),
                    GeneralConsensusMessage::HighQc(qc, _)
                    | GeneralConsensusMessage::ExtendedQc(qc, _) => qc.view_number(),
                    GeneralConsensusMessage::EpochRootQuorumVote(vote) => vote.view_number(),
                    GeneralConsensusMessage::EpochRootQc(root_qc) => root_qc.view_number(),
                }
            },
            SequencingMessage::Da(da_message) => {
                match da_message {
                    DaConsensusMessage::DaProposal(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.view_number()
                    },
                    DaConsensusMessage::DaVote(vote_message) => vote_message.view_number(),
                    DaConsensusMessage::DaCertificate(cert) => cert.view_number,
                    DaConsensusMessage::VidDisperseMsg(disperse) => disperse.data.view_number(),
                    DaConsensusMessage::DaProposal2(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.view_number()
                    },
                    DaConsensusMessage::DaVote2(vote_message) => vote_message.view_number(),
                    DaConsensusMessage::DaCertificate2(cert) => cert.view_number,
                    DaConsensusMessage::VidDisperseMsg2(disperse) => disperse.data.view_number(),
                }
            },
        }
    }

    /// Get the epoch number this message relates to, if applicable
    fn epoch_number(&self) -> Option<TYPES::Epoch> {
        match &self {
            SequencingMessage::General(general_message) => {
                match general_message {
                    GeneralConsensusMessage::Proposal(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.epoch()
                    },
                    GeneralConsensusMessage::Proposal2(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.epoch()
                    },
                    GeneralConsensusMessage::ProposalRequested(..) => None,
                    GeneralConsensusMessage::ProposalResponse(proposal) => proposal.data.epoch(),
                    GeneralConsensusMessage::ProposalResponse2(proposal) => proposal.data.epoch(),
                    GeneralConsensusMessage::Vote(vote_message) => vote_message.epoch(),
                    GeneralConsensusMessage::Vote2(vote_message) => vote_message.epoch(),
                    GeneralConsensusMessage::TimeoutVote(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncPreCommitVote(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncCommitVote(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncFinalizeVote(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncPreCommitCertificate(message) => {
                        message.epoch()
                    },
                    GeneralConsensusMessage::ViewSyncCommitCertificate(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncFinalizeCertificate(message) => {
                        message.epoch()
                    },
                    GeneralConsensusMessage::TimeoutVote2(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncPreCommitVote2(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncCommitVote2(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncFinalizeVote2(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncPreCommitCertificate2(message) => {
                        message.epoch()
                    },
                    GeneralConsensusMessage::ViewSyncCommitCertificate2(message) => message.epoch(),
                    GeneralConsensusMessage::ViewSyncFinalizeCertificate2(message) => {
                        message.epoch()
                    },
                    GeneralConsensusMessage::UpgradeProposal(message) => message.data.epoch(),
                    GeneralConsensusMessage::UpgradeVote(message) => message.epoch(),
                    GeneralConsensusMessage::HighQc(qc, _)
                    | GeneralConsensusMessage::ExtendedQc(qc, _) => qc.epoch(),
                    GeneralConsensusMessage::EpochRootQuorumVote(vote) => vote.epoch(),
                    GeneralConsensusMessage::EpochRootQc(root_qc) => root_qc.epoch(),
                }
            },
            SequencingMessage::Da(da_message) => {
                match da_message {
                    DaConsensusMessage::DaProposal(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.epoch()
                    },
                    DaConsensusMessage::DaVote(vote_message) => vote_message.epoch(),
                    DaConsensusMessage::DaCertificate(cert) => cert.epoch(),
                    DaConsensusMessage::VidDisperseMsg(disperse) => disperse.data.epoch(),
                    DaConsensusMessage::VidDisperseMsg2(disperse) => disperse.data.epoch(),
                    DaConsensusMessage::DaProposal2(p) => {
                        // view of leader in the leaf when proposal
                        // this should match replica upon receipt
                        p.data.epoch()
                    },
                    DaConsensusMessage::DaVote2(vote_message) => vote_message.epoch(),
                    DaConsensusMessage::DaCertificate2(cert) => cert.epoch(),
                }
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = ""))]
#[allow(clippy::large_enum_variant)]
/// TODO: Put `DataResponse` content in a `Box` to make enum smaller
/// Messages related to sending data between nodes
pub enum DataMessage<TYPES: NodeType> {
    /// Contains a transaction to be submitted
    /// TODO rethink this when we start to send these messages
    /// we only need the view number for broadcast
    SubmitTransaction(TYPES::Transaction, TYPES::View),
    /// A request for data
    RequestData(DataRequest<TYPES>),
    /// A response to a data request
    DataResponse(ResponseMessage<TYPES>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(bound(deserialize = ""))]
/// Prepare qc from the leader
pub struct Proposal<
    TYPES: NodeType,
    PROPOSAL: HasViewNumber<TYPES> + HasEpoch<TYPES> + DeserializeOwned,
> {
    // NOTE: optimization could include view number to help look up parent leaf
    // could even do 16 bit numbers if we want
    /// The data being proposed.
    pub data: PROPOSAL,
    /// The proposal must be signed by the view leader
    pub signature: <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    /// Phantom for TYPES
    pub _pd: PhantomData<TYPES>,
}

/// Convert a `Proposal` by converting the underlying proposal type
pub fn convert_proposal<TYPES, PROPOSAL, PROPOSAL2>(
    proposal: Proposal<TYPES, PROPOSAL>,
) -> Proposal<TYPES, PROPOSAL2>
where
    TYPES: NodeType,
    PROPOSAL: HasViewNumber<TYPES> + HasEpoch<TYPES> + DeserializeOwned,
    PROPOSAL2: HasViewNumber<TYPES> + HasEpoch<TYPES> + DeserializeOwned + From<PROPOSAL>,
{
    Proposal {
        data: proposal.data.into(),
        signature: proposal.signature,
        _pd: proposal._pd,
    }
}

impl<TYPES> Proposal<TYPES, QuorumProposal<TYPES>>
where
    TYPES: NodeType,
{
    /// Checks that the signature of the quorum proposal is valid.
    /// # Errors
    /// Returns an error when the proposal signature is invalid.
    pub async fn validate_signature<V: Versions>(
        &self,
        membership: &TYPES::Membership,
        _epoch_height: u64,
        upgrade_lock: &UpgradeLock<TYPES, V>,
    ) -> Result<()> {
        let view_number = self.data.view_number();
        let view_leader_key = membership.leader(view_number, None)?;
        let proposed_leaf = Leaf::from_quorum_proposal(&self.data);

        ensure!(
            view_leader_key.validate(
                &self.signature,
                proposed_leaf.commit(upgrade_lock).await.as_ref()
            ),
            "Proposal signature is invalid."
        );

        Ok(())
    }
}

impl<TYPES> Proposal<TYPES, QuorumProposalWrapper<TYPES>>
where
    TYPES: NodeType,
{
    /// Checks that the signature of the quorum proposal is valid.
    /// # Errors
    /// Returns an error when the proposal signature is invalid.
    pub async fn validate_signature(&self, membership: &EpochMembership<TYPES>) -> Result<()> {
        let view_number = self.data.proposal.view_number();
        let view_leader_key = membership.leader(view_number).await?;
        let proposed_leaf = Leaf2::from_quorum_proposal(&self.data);

        ensure!(
            view_leader_key.validate(&self.signature, proposed_leaf.commit().as_ref()),
            "Proposal signature is invalid."
        );

        Ok(())
    }
}

#[derive(Clone, Debug)]
/// A lock for an upgrade certificate decided by HotShot, which doubles as `PhantomData` for an instance of the `Versions` trait.
pub struct UpgradeLock<TYPES: NodeType, V: Versions> {
    /// a shared lock to an upgrade certificate decided by consensus
    pub decided_upgrade_certificate: Arc<RwLock<Option<UpgradeCertificate<TYPES>>>>,

    /// phantom data for the `Versions` trait
    pub _pd: PhantomData<V>,
}

impl<TYPES: NodeType, V: Versions> UpgradeLock<TYPES, V> {
    #[allow(clippy::new_without_default)]
    /// Create a new `UpgradeLock` for a fresh instance of HotShot
    pub fn new() -> Self {
        Self {
            decided_upgrade_certificate: Arc::new(RwLock::new(None)),
            _pd: PhantomData::<V>,
        }
    }

    #[allow(clippy::new_without_default)]
    /// Create a new `UpgradeLock` from an optional upgrade certificate
    pub fn from_certificate(certificate: &Option<UpgradeCertificate<TYPES>>) -> Self {
        Self {
            decided_upgrade_certificate: Arc::new(RwLock::new(certificate.clone())),
            _pd: PhantomData::<V>,
        }
    }

    pub async fn upgrade_view(&self) -> Option<TYPES::View> {
        let upgrade_certificate = self.decided_upgrade_certificate.read().await;
        upgrade_certificate
            .as_ref()
            .map(|cert| cert.data.new_version_first_view)
    }

    /// Calculate the version applied in a view, based on the provided upgrade lock.
    ///
    /// # Errors
    /// Returns an error if we do not support the version required by the decided upgrade certificate.
    pub async fn version(&self, view: TYPES::View) -> Result<Version> {
        let upgrade_certificate = self.decided_upgrade_certificate.read().await;

        let version = match *upgrade_certificate {
            Some(ref cert) => {
                if view >= cert.data.new_version_first_view {
                    if cert.data.new_version == V::Upgrade::VERSION {
                        V::Upgrade::VERSION
                    } else {
                        bail!("The network has upgraded to a new version that we do not support!");
                    }
                } else {
                    V::Base::VERSION
                }
            },
            None => V::Base::VERSION,
        };

        Ok(version)
    }

    /// Calculate the version applied in a view, based on the provided upgrade lock.
    ///
    /// This function does not fail, since it does not check that the version is supported.
    pub async fn version_infallible(&self, view: TYPES::View) -> Version {
        let upgrade_certificate = self.decided_upgrade_certificate.read().await;

        match *upgrade_certificate {
            Some(ref cert) => {
                if view >= cert.data.new_version_first_view {
                    cert.data.new_version
                } else {
                    cert.data.old_version
                }
            },
            None => V::Base::VERSION,
        }
    }

    /// Return whether epochs are enabled in the given view
    pub async fn epochs_enabled(&self, view: TYPES::View) -> bool {
        self.version_infallible(view).await >= V::Epochs::VERSION
    }

    /// Serialize a message with a version number, using `message.view_number()` and an optional decided upgrade certificate to determine the message's version.
    ///
    /// # Errors
    ///
    /// Errors if serialization fails.
    pub async fn serialize<M: HasViewNumber<TYPES> + Serialize>(
        &self,
        message: &M,
    ) -> Result<Vec<u8>> {
        let view = message.view_number();

        let version = self.version(view).await?;

        let serialized_message = match version {
            // Associated constants cannot be used in pattern matches, so we do this trick instead.
            v if v == V::Base::VERSION => Serializer::<V::Base>::serialize(&message),
            v if v == V::Upgrade::VERSION => Serializer::<V::Upgrade>::serialize(&message),
            v => {
                bail!("Attempted to serialize with version {}, which is incompatible. This should be impossible.", v);
            },
        };

        serialized_message
            .wrap()
            .context(info!("Failed to serialize message!"))
    }

    /// Deserialize a message with a version number, using `message.view_number()` to determine the message's version. This function will fail on improperly versioned messages.
    ///
    /// # Errors
    ///
    /// Errors if deserialization fails.
    pub async fn deserialize<M: HasViewNumber<TYPES> + for<'a> Deserialize<'a>>(
        &self,
        message: &[u8],
    ) -> Result<M> {
        let actual_version = Version::deserialize(message)
            .wrap()
            .context(info!("Failed to read message version!"))?
            .0;

        let deserialized_message: M = match actual_version {
            v if v == V::Base::VERSION => Serializer::<V::Base>::deserialize(message),
            v if v == V::Upgrade::VERSION => Serializer::<V::Upgrade>::deserialize(message),
            v => {
                bail!("Cannot deserialize message with stated version {}", v);
            },
        }
        .wrap()
        .context(info!("Failed to deserialize message!"))?;

        let view = deserialized_message.view_number();

        let expected_version = self.version(view).await?;

        if actual_version != expected_version {
            return Err(error!(format!("Message has invalid version number for its view. Expected: {expected_version}, Actual: {actual_version}, View: {view:?}")));
        };

        Ok(deserialized_message)
    }
}
