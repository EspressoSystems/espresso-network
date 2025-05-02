// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::fmt::Display;

use async_broadcast::Sender;
use either::Either;
use hotshot_task::task::TaskEvent;
use hotshot_types::{
    data::{
        DaProposal2, Leaf2, PackedBundle, QuorumProposal2, QuorumProposalWrapper, UpgradeProposal,
        VidCommitment, VidDisperse, VidDisperseShare,
    },
    message::Proposal,
    request_response::ProposalRequestPayload,
    simple_certificate::{
        DaCertificate2, EpochRootQuorumCertificate, NextEpochQuorumCertificate2, QuorumCertificate,
        QuorumCertificate2, TimeoutCertificate, TimeoutCertificate2, UpgradeCertificate,
        ViewSyncCommitCertificate2, ViewSyncFinalizeCertificate2, ViewSyncPreCommitCertificate2,
    },
    simple_vote::{
        DaVote2, EpochRootQuorumVote, QuorumVote2, TimeoutVote2, UpgradeVote, ViewSyncCommitVote2,
        ViewSyncFinalizeVote2, ViewSyncPreCommitVote2,
    },
    traits::{
        block_contents::BuilderFee, network::DataRequest, node_implementation::NodeType,
        signature_key::SignatureKey, BlockPayload,
    },
    utils::BuilderCommitment,
    vote::HasViewNumber,
};
use vec1::Vec1;

use crate::view_sync::ViewSyncPhase;

impl<TYPES: NodeType> TaskEvent for HotShotEvent<TYPES> {
    fn shutdown_event() -> Self {
        HotShotEvent::Shutdown
    }
}

/// Wrapper type for the event to notify tasks that a proposal for a view is missing
/// and the channel to send the event back to
#[derive(Debug, Clone)]
pub struct ProposalMissing<TYPES: NodeType> {
    /// View of missing proposal
    pub view: TYPES::View,
    /// Channel to send the response back to
    pub response_chan: Sender<Option<Proposal<TYPES, QuorumProposal2<TYPES>>>>,
}

impl<TYPES: NodeType> PartialEq for ProposalMissing<TYPES> {
    fn eq(&self, other: &Self) -> bool {
        self.view == other.view
    }
}

impl<TYPES: NodeType> Eq for ProposalMissing<TYPES> {}

/// Marker that the task completed
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct HotShotTaskCompleted;

/// All of the possible events that can be passed between Sequencing `HotShot` tasks
#[derive(Eq, PartialEq, Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum HotShotEvent<TYPES: NodeType> {
    /// Shutdown the task
    Shutdown,
    /// A quorum proposal has been received from the network; handled by the consensus task
    QuorumProposalRecv(
        Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
        TYPES::SignatureKey,
    ),
    /// A quorum vote has been received from the network; handled by the consensus task
    QuorumVoteRecv(QuorumVote2<TYPES>),
    /// A quorum vote for the epoch root has been received from the network; handled by the consensus task
    /// An additional light client state update vote is bundled with the quorum vote
    EpochRootQuorumVoteRecv(EpochRootQuorumVote<TYPES>),
    /// A timeout vote received from the network; handled by consensus task
    TimeoutVoteRecv(TimeoutVote2<TYPES>),
    /// Send a timeout vote to the network; emitted by consensus task replicas
    TimeoutVoteSend(TimeoutVote2<TYPES>),
    /// A DA proposal has been received from the network; handled by the DA task
    DaProposalRecv(Proposal<TYPES, DaProposal2<TYPES>>, TYPES::SignatureKey),
    /// A DA proposal has been validated; handled by the DA task and VID task
    DaProposalValidated(Proposal<TYPES, DaProposal2<TYPES>>, TYPES::SignatureKey),
    /// A DA vote has been received by the network; handled by the DA task
    DaVoteRecv(DaVote2<TYPES>),
    /// A Data Availability Certificate (DAC) has been received by the network; handled by the consensus task
    DaCertificateRecv(DaCertificate2<TYPES>),
    /// A DAC is validated.
    DaCertificateValidated(DaCertificate2<TYPES>),
    /// Send a quorum proposal to the network; emitted by the leader in the consensus task
    QuorumProposalSend(
        Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
        TYPES::SignatureKey,
    ),
    /// Send a quorum vote to the next leader; emitted by a replica in the consensus task after seeing a valid quorum proposal
    QuorumVoteSend(QuorumVote2<TYPES>),
    /// Broadcast a quorum vote to form an eQC; emitted by a replica in the consensus task after seeing a valid quorum proposal
    ExtendedQuorumVoteSend(QuorumVote2<TYPES>),
    /// Send a epoch root quorum vote to the next leader; emitted by a replica in the consensus task after seeing a valid quorum proposal
    EpochRootQuorumVoteSend(EpochRootQuorumVote<TYPES>),
    /// A quorum proposal with the given parent leaf is validated.
    /// The full validation checks include:
    /// 1. The proposal is not for an old view
    /// 2. The proposal has been correctly signed by the leader of the current view
    /// 3. The justify QC is valid
    /// 4. The proposal passes either liveness or safety check.
    QuorumProposalValidated(Proposal<TYPES, QuorumProposalWrapper<TYPES>>, Leaf2<TYPES>),
    /// A quorum proposal is missing for a view that we need.
    QuorumProposalRequestSend(
        ProposalRequestPayload<TYPES>,
        <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ),
    /// A quorum proposal was requested by a node for a view.
    QuorumProposalRequestRecv(
        ProposalRequestPayload<TYPES>,
        <TYPES::SignatureKey as SignatureKey>::PureAssembledSignatureType,
    ),
    /// A quorum proposal was missing for a view. As the leader, we send a reply to the recipient with their key.
    QuorumProposalResponseSend(
        TYPES::SignatureKey,
        Proposal<TYPES, QuorumProposalWrapper<TYPES>>,
    ),
    /// A quorum proposal was requested by a node for a view.
    QuorumProposalResponseRecv(Proposal<TYPES, QuorumProposalWrapper<TYPES>>),
    /// Send a DA proposal to the DA committee; emitted by the DA leader (which is the same node as the leader of view v + 1) in the DA task
    DaProposalSend(Proposal<TYPES, DaProposal2<TYPES>>, TYPES::SignatureKey),
    /// Send a DA vote to the DA leader; emitted by DA committee members in the DA task after seeing a valid DA proposal
    DaVoteSend(DaVote2<TYPES>),
    /// The next leader has collected enough votes to form a QC; emitted by the next leader in the consensus task; an internal event only
    QcFormed(Either<QuorumCertificate<TYPES>, TimeoutCertificate<TYPES>>),
    /// The next leader has collected enough votes to form a QC; emitted by the next leader in the consensus task; an internal event only
    Qc2Formed(Either<QuorumCertificate2<TYPES>, TimeoutCertificate2<TYPES>>),
    /// The next leader has collected enough votes to form an epoch root QC; emitted by the next leader in the consensus task; an internal event only
    EpochRootQcFormed(EpochRootQuorumCertificate<TYPES>),
    /// The next leader has collected enough votes from the next epoch nodes to form a QC; emitted by the next leader in the consensus task; an internal event only
    NextEpochQc2Formed(Either<NextEpochQuorumCertificate2<TYPES>, TimeoutCertificate<TYPES>>),
    /// A validator formed both a current epoch eQC and a next epoch eQC
    ExtendedQc2Formed(QuorumCertificate2<TYPES>),
    /// The DA leader has collected enough votes to form a DAC; emitted by the DA leader in the DA task; sent to the entire network via the networking task
    DacSend(DaCertificate2<TYPES>, TYPES::SignatureKey),
    /// The current view has changed; emitted by the replica in the consensus task or replica in the view sync task; received by almost all other tasks
    ViewChange(TYPES::View, Option<TYPES::Epoch>),
    /// The view and epoch number of the first epoch
    SetFirstEpoch(TYPES::View, TYPES::Epoch),
    /// Timeout for the view sync protocol; emitted by a replica in the view sync task
    ViewSyncTimeout(TYPES::View, u64, ViewSyncPhase),

    /// Receive a `ViewSyncPreCommitVote` from the network; received by a relay in the view sync task
    ViewSyncPreCommitVoteRecv(ViewSyncPreCommitVote2<TYPES>),
    /// Receive a `ViewSyncCommitVote` from the network; received by a relay in the view sync task
    ViewSyncCommitVoteRecv(ViewSyncCommitVote2<TYPES>),
    /// Receive a `ViewSyncFinalizeVote` from the network; received by a relay in the view sync task
    ViewSyncFinalizeVoteRecv(ViewSyncFinalizeVote2<TYPES>),

    /// Send a `ViewSyncPreCommitVote` from the network; emitted by a replica in the view sync task
    ViewSyncPreCommitVoteSend(ViewSyncPreCommitVote2<TYPES>),
    /// Send a `ViewSyncCommitVote` from the network; emitted by a replica in the view sync task
    ViewSyncCommitVoteSend(ViewSyncCommitVote2<TYPES>),
    /// Send a `ViewSyncFinalizeVote` from the network; emitted by a replica in the view sync task
    ViewSyncFinalizeVoteSend(ViewSyncFinalizeVote2<TYPES>),

    /// Receive a `ViewSyncPreCommitCertificate` from the network; received by a replica in the view sync task
    ViewSyncPreCommitCertificateRecv(ViewSyncPreCommitCertificate2<TYPES>),
    /// Receive a `ViewSyncCommitCertificate` from the network; received by a replica in the view sync task
    ViewSyncCommitCertificateRecv(ViewSyncCommitCertificate2<TYPES>),
    /// Receive a `ViewSyncFinalizeCertificate` from the network; received by a replica in the view sync task
    ViewSyncFinalizeCertificateRecv(ViewSyncFinalizeCertificate2<TYPES>),

    /// Send a `ViewSyncPreCommitCertificate` from the network; emitted by a relay in the view sync task
    ViewSyncPreCommitCertificateSend(ViewSyncPreCommitCertificate2<TYPES>, TYPES::SignatureKey),
    /// Send a `ViewSyncCommitCertificate` from the network; emitted by a relay in the view sync task
    ViewSyncCommitCertificateSend(ViewSyncCommitCertificate2<TYPES>, TYPES::SignatureKey),
    /// Send a `ViewSyncFinalizeCertificate` from the network; emitted by a relay in the view sync task
    ViewSyncFinalizeCertificateSend(ViewSyncFinalizeCertificate2<TYPES>, TYPES::SignatureKey),

    /// Trigger the start of the view sync protocol; emitted by view sync task; internal trigger only
    ViewSyncTrigger(TYPES::View),
    /// A consensus view has timed out; emitted by a replica in the consensus task; received by the view sync task; internal event only
    Timeout(TYPES::View, Option<TYPES::Epoch>),
    /// Receive transactions from the network
    TransactionsRecv(Vec<TYPES::Transaction>),
    /// Send transactions to the network
    TransactionSend(TYPES::Transaction, TYPES::SignatureKey),
    /// Event to send block payload commitment and metadata from DA leader to the quorum; internal event only
    SendPayloadCommitmentAndMetadata(
        VidCommitment,
        BuilderCommitment,
        <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
        TYPES::View,
        Vec1<BuilderFee<TYPES>>,
        Option<TYPES::AuctionResult>,
    ),
    /// Event when the transactions task has sequenced transactions. Contains the encoded transactions, the metadata, and the view number
    BlockRecv(PackedBundle<TYPES>),
    /// Send VID shares to VID storage nodes; emitted by the DA leader
    ///
    /// Like [`HotShotEvent::DaProposalSend`].
    VidDisperseSend(Proposal<TYPES, VidDisperse<TYPES>>, TYPES::SignatureKey),
    /// Vid disperse share has been received from the network; handled by the consensus task
    ///
    /// Like [`HotShotEvent::DaProposalRecv`].
    VidShareRecv(
        TYPES::SignatureKey,
        Proposal<TYPES, VidDisperseShare<TYPES>>,
    ),
    /// VID share data is validated.
    VidShareValidated(Proposal<TYPES, VidDisperseShare<TYPES>>),
    /// Upgrade proposal has been received from the network
    UpgradeProposalRecv(Proposal<TYPES, UpgradeProposal<TYPES>>, TYPES::SignatureKey),
    /// Upgrade proposal has been sent to the network
    UpgradeProposalSend(Proposal<TYPES, UpgradeProposal<TYPES>>, TYPES::SignatureKey),
    /// Upgrade vote has been received from the network
    UpgradeVoteRecv(UpgradeVote<TYPES>),
    /// Upgrade vote has been sent to the network
    UpgradeVoteSend(UpgradeVote<TYPES>),
    /// Upgrade certificate has been sent to the network
    UpgradeCertificateFormed(UpgradeCertificate<TYPES>),
    /// A quorum proposal has been preliminarily validated.
    /// The preliminary checks include:
    /// 1. The proposal is not for an old view
    /// 2. The proposal has been correctly signed by the leader of the current view
    /// 3. The justify QC is valid
    QuorumProposalPreliminarilyValidated(Proposal<TYPES, QuorumProposalWrapper<TYPES>>),

    /// Send a VID request to the network; emitted to on of the members of DA committee.
    /// Includes the data request, node's public key and signature as well as public key of DA committee who we want to send to.
    VidRequestSend(
        DataRequest<TYPES>,
        // Sender
        TYPES::SignatureKey,
        // Recipient
        TYPES::SignatureKey,
    ),

    /// Receive a VID request from the network; Received by a node in the DA committee.
    /// Includes the data request and nodes public key.
    VidRequestRecv(DataRequest<TYPES>, TYPES::SignatureKey),

    /// Send a VID response to the network; emitted to the sending node.
    /// Includes nodes public key, recipient public key, and vid disperse
    VidResponseSend(
        /// Sender key
        TYPES::SignatureKey,
        /// Recipient key
        TYPES::SignatureKey,
        Proposal<TYPES, VidDisperseShare<TYPES>>,
    ),

    /// Receive a VID response from the network; received by the node that triggered the VID request.
    VidResponseRecv(
        TYPES::SignatureKey,
        Proposal<TYPES, VidDisperseShare<TYPES>>,
    ),

    /// A replica send us a High QC
    HighQcRecv(
        QuorumCertificate2<TYPES>,
        Option<NextEpochQuorumCertificate2<TYPES>>,
        TYPES::SignatureKey,
    ),

    /// Send our HighQc to the next leader, should go to the same leader as our vote
    HighQcSend(
        QuorumCertificate2<TYPES>,
        Option<NextEpochQuorumCertificate2<TYPES>>,
        TYPES::SignatureKey,
        TYPES::SignatureKey,
    ),

    /// A replica sent us an extended QuorumCertificate and NextEpochQuorumCertificate
    ExtendedQcRecv(
        QuorumCertificate2<TYPES>,
        NextEpochQuorumCertificate2<TYPES>,
        TYPES::SignatureKey,
    ),

    /// Send our extended QuorumCertificate and NextEpochQuorumCertificate to all nodes in the old and new epoch
    ExtendedQcSend(
        QuorumCertificate2<TYPES>,
        NextEpochQuorumCertificate2<TYPES>,
        TYPES::SignatureKey,
    ),

    /// A replica sends us an epoch root QC
    EpochRootQcSend(
        EpochRootQuorumCertificate<TYPES>,
        TYPES::SignatureKey,
        TYPES::SignatureKey,
    ),
    /// A replica receives an epoch root QC
    EpochRootQcRecv(EpochRootQuorumCertificate<TYPES>, TYPES::SignatureKey),
}

impl<TYPES: NodeType> HotShotEvent<TYPES> {
    #[allow(clippy::too_many_lines)]
    /// Return the view number for a hotshot event if present
    pub fn view_number(&self) -> Option<TYPES::View> {
        match self {
            HotShotEvent::QuorumVoteSend(v)
            | HotShotEvent::QuorumVoteRecv(v)
            | HotShotEvent::ExtendedQuorumVoteSend(v) => Some(v.view_number()),
            HotShotEvent::EpochRootQuorumVoteRecv(v) | HotShotEvent::EpochRootQuorumVoteSend(v) => {
                Some(v.view_number())
            },
            HotShotEvent::TimeoutVoteRecv(v) | HotShotEvent::TimeoutVoteSend(v) => {
                Some(v.view_number())
            },
            HotShotEvent::QuorumProposalRecv(proposal, _)
            | HotShotEvent::QuorumProposalSend(proposal, _)
            | HotShotEvent::QuorumProposalValidated(proposal, _)
            | HotShotEvent::QuorumProposalResponseRecv(proposal)
            | HotShotEvent::QuorumProposalResponseSend(_, proposal)
            | HotShotEvent::QuorumProposalPreliminarilyValidated(proposal) => {
                Some(proposal.data.view_number())
            },
            HotShotEvent::DaProposalRecv(proposal, _)
            | HotShotEvent::DaProposalValidated(proposal, _)
            | HotShotEvent::DaProposalSend(proposal, _) => Some(proposal.data.view_number()),
            HotShotEvent::DaVoteRecv(vote) | HotShotEvent::DaVoteSend(vote) => {
                Some(vote.view_number())
            },
            HotShotEvent::QcFormed(cert) => match cert {
                either::Left(qc) => Some(qc.view_number()),
                either::Right(tc) => Some(tc.view_number()),
            },
            HotShotEvent::Qc2Formed(cert) => match cert {
                either::Left(qc) => Some(qc.view_number()),
                either::Right(tc) => Some(tc.view_number()),
            },
            HotShotEvent::NextEpochQc2Formed(cert) => match cert {
                either::Left(qc) => Some(qc.view_number()),
                either::Right(tc) => Some(tc.view_number()),
            },
            HotShotEvent::EpochRootQcFormed(root_qc) => Some(root_qc.view_number()),
            HotShotEvent::ExtendedQc2Formed(cert) => Some(cert.view_number()),
            HotShotEvent::ViewSyncCommitVoteSend(vote)
            | HotShotEvent::ViewSyncCommitVoteRecv(vote) => Some(vote.view_number()),
            HotShotEvent::ViewSyncPreCommitVoteRecv(vote)
            | HotShotEvent::ViewSyncPreCommitVoteSend(vote) => Some(vote.view_number()),
            HotShotEvent::ViewSyncFinalizeVoteRecv(vote)
            | HotShotEvent::ViewSyncFinalizeVoteSend(vote) => Some(vote.view_number()),
            HotShotEvent::ViewSyncPreCommitCertificateRecv(cert)
            | HotShotEvent::ViewSyncPreCommitCertificateSend(cert, _) => Some(cert.view_number()),
            HotShotEvent::ViewSyncCommitCertificateRecv(cert)
            | HotShotEvent::ViewSyncCommitCertificateSend(cert, _) => Some(cert.view_number()),
            HotShotEvent::ViewSyncFinalizeCertificateRecv(cert)
            | HotShotEvent::ViewSyncFinalizeCertificateSend(cert, _) => Some(cert.view_number()),
            HotShotEvent::SendPayloadCommitmentAndMetadata(_, _, _, view_number, ..) => {
                Some(*view_number)
            },
            HotShotEvent::BlockRecv(packed_bundle) => Some(packed_bundle.view_number),
            HotShotEvent::Shutdown
            | HotShotEvent::TransactionSend(..)
            | HotShotEvent::TransactionsRecv(_) => None,
            HotShotEvent::VidDisperseSend(proposal, _) => Some(proposal.data.view_number()),
            HotShotEvent::VidShareRecv(_, proposal) | HotShotEvent::VidShareValidated(proposal) => {
                Some(proposal.data.view_number())
            },
            HotShotEvent::UpgradeProposalRecv(proposal, _)
            | HotShotEvent::UpgradeProposalSend(proposal, _) => Some(proposal.data.view_number()),
            HotShotEvent::UpgradeVoteRecv(vote) | HotShotEvent::UpgradeVoteSend(vote) => {
                Some(vote.view_number())
            },
            HotShotEvent::QuorumProposalRequestSend(req, _)
            | HotShotEvent::QuorumProposalRequestRecv(req, _) => Some(req.view_number),
            HotShotEvent::ViewChange(view_number, _)
            | HotShotEvent::ViewSyncTimeout(view_number, ..)
            | HotShotEvent::ViewSyncTrigger(view_number)
            | HotShotEvent::Timeout(view_number, ..) => Some(*view_number),
            HotShotEvent::DaCertificateRecv(cert) | HotShotEvent::DacSend(cert, _) => {
                Some(cert.view_number())
            },
            HotShotEvent::DaCertificateValidated(cert) => Some(cert.view_number),
            HotShotEvent::UpgradeCertificateFormed(cert) => Some(cert.view_number()),
            HotShotEvent::VidRequestSend(request, ..)
            | HotShotEvent::VidRequestRecv(request, _) => Some(request.view),
            HotShotEvent::VidResponseSend(_, _, proposal)
            | HotShotEvent::VidResponseRecv(_, proposal) => Some(proposal.data.view_number()),
            HotShotEvent::HighQcRecv(qc, ..)
            | HotShotEvent::HighQcSend(qc, ..)
            | HotShotEvent::ExtendedQcRecv(qc, ..)
            | HotShotEvent::ExtendedQcSend(qc, ..) => Some(qc.view_number()),
            HotShotEvent::EpochRootQcSend(cert, ..) | HotShotEvent::EpochRootQcRecv(cert, _) => {
                Some(cert.view_number())
            },
            HotShotEvent::SetFirstEpoch(..) => None,
        }
    }
}

impl<TYPES: NodeType> Display for HotShotEvent<TYPES> {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HotShotEvent::Shutdown => write!(f, "Shutdown"),
            HotShotEvent::QuorumProposalRecv(proposal, _) => write!(
                f,
                "QuorumProposalRecv(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::QuorumVoteRecv(v) => {
                write!(f, "QuorumVoteRecv(view_number={:?})", v.view_number())
            },
            HotShotEvent::EpochRootQuorumVoteRecv(v) => {
                write!(
                    f,
                    "EpochRootQuorumVoteRecv(view_number={:?})",
                    v.view_number()
                )
            },
            HotShotEvent::EpochRootQuorumVoteSend(v) => {
                write!(
                    f,
                    "EpochRootQuorumVoteSend(view_number={:?})",
                    v.view_number()
                )
            },
            HotShotEvent::ExtendedQuorumVoteSend(v) => {
                write!(
                    f,
                    "ExtendedQuorumVoteSend(view_number={:?})",
                    v.view_number()
                )
            },
            HotShotEvent::TimeoutVoteRecv(v) => {
                write!(f, "TimeoutVoteRecv(view_number={:?})", v.view_number())
            },
            HotShotEvent::TimeoutVoteSend(v) => {
                write!(f, "TimeoutVoteSend(view_number={:?})", v.view_number())
            },
            HotShotEvent::DaProposalRecv(proposal, _) => write!(
                f,
                "DaProposalRecv(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::DaProposalValidated(proposal, _) => write!(
                f,
                "DaProposalValidated(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::DaVoteRecv(vote) => {
                write!(f, "DaVoteRecv(view_number={:?})", vote.view_number())
            },
            HotShotEvent::DaCertificateRecv(cert) => {
                write!(f, "DaCertificateRecv(view_number={:?})", cert.view_number())
            },
            HotShotEvent::DaCertificateValidated(cert) => write!(
                f,
                "DaCertificateValidated(view_number={:?})",
                cert.view_number()
            ),
            HotShotEvent::QuorumProposalSend(proposal, _) => write!(
                f,
                "QuorumProposalSend(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::QuorumVoteSend(vote) => {
                write!(f, "QuorumVoteSend(view_number={:?})", vote.view_number())
            },
            HotShotEvent::QuorumProposalValidated(proposal, _) => write!(
                f,
                "QuorumProposalValidated(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::DaProposalSend(proposal, _) => write!(
                f,
                "DaProposalSend(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::DaVoteSend(vote) => {
                write!(f, "DaVoteSend(view_number={:?})", vote.view_number())
            },
            HotShotEvent::QcFormed(cert) => match cert {
                either::Left(qc) => write!(f, "QcFormed(view_number={:?})", qc.view_number()),
                either::Right(tc) => write!(f, "QcFormed(view_number={:?})", tc.view_number()),
            },
            HotShotEvent::Qc2Formed(cert) => match cert {
                either::Left(qc) => write!(f, "QcFormed2(view_number={:?})", qc.view_number()),
                either::Right(tc) => write!(f, "QcFormed2(view_number={:?})", tc.view_number()),
            },
            HotShotEvent::EpochRootQcFormed(root_qc) => {
                write!(
                    f,
                    "EpochRootQcFormed(view_number={:?})",
                    root_qc.view_number()
                )
            },
            HotShotEvent::NextEpochQc2Formed(cert) => match cert {
                either::Left(qc) => {
                    write!(f, "NextEpochQc2Formed(view_number={:?})", qc.view_number())
                },
                either::Right(tc) => {
                    write!(f, "NextEpochQc2Formed(view_number={:?})", tc.view_number())
                },
            },
            HotShotEvent::ExtendedQc2Formed(cert) => {
                write!(f, "ExtendedQc2Formed(view_number={:?})", cert.view_number())
            },
            HotShotEvent::DacSend(cert, _) => {
                write!(f, "DacSend(view_number={:?})", cert.view_number())
            },
            HotShotEvent::ViewChange(view_number, epoch_number) => {
                write!(
                    f,
                    "ViewChange(view_number={view_number:?}, epoch_number={epoch_number:?})"
                )
            },
            HotShotEvent::ViewSyncTimeout(view_number, ..) => {
                write!(f, "ViewSyncTimeout(view_number={view_number:?})")
            },
            HotShotEvent::ViewSyncPreCommitVoteRecv(vote) => write!(
                f,
                "ViewSyncPreCommitVoteRecv(view_number={:?})",
                vote.view_number()
            ),
            HotShotEvent::ViewSyncCommitVoteRecv(vote) => write!(
                f,
                "ViewSyncCommitVoteRecv(view_number={:?})",
                vote.view_number()
            ),
            HotShotEvent::ViewSyncFinalizeVoteRecv(vote) => write!(
                f,
                "ViewSyncFinalizeVoteRecv(view_number={:?})",
                vote.view_number()
            ),
            HotShotEvent::ViewSyncPreCommitVoteSend(vote) => write!(
                f,
                "ViewSyncPreCommitVoteSend(view_number={:?})",
                vote.view_number()
            ),
            HotShotEvent::ViewSyncCommitVoteSend(vote) => write!(
                f,
                "ViewSyncCommitVoteSend(view_number={:?})",
                vote.view_number()
            ),
            HotShotEvent::ViewSyncFinalizeVoteSend(vote) => write!(
                f,
                "ViewSyncFinalizeVoteSend(view_number={:?})",
                vote.view_number()
            ),
            HotShotEvent::ViewSyncPreCommitCertificateRecv(cert) => {
                write!(
                    f,
                    "ViewSyncPreCommitCertificateRecv(view_number={:?})",
                    cert.view_number()
                )
            },
            HotShotEvent::ViewSyncCommitCertificateRecv(cert) => {
                write!(
                    f,
                    "ViewSyncCommitCertificateRecv(view_number={:?})",
                    cert.view_number()
                )
            },
            HotShotEvent::ViewSyncFinalizeCertificateRecv(cert) => {
                write!(
                    f,
                    "ViewSyncFinalizeCertificateRecv(view_number={:?})",
                    cert.view_number()
                )
            },
            HotShotEvent::ViewSyncPreCommitCertificateSend(cert, _) => {
                write!(
                    f,
                    "ViewSyncPreCommitCertificateSend(view_number={:?})",
                    cert.view_number()
                )
            },
            HotShotEvent::ViewSyncCommitCertificateSend(cert, _) => {
                write!(
                    f,
                    "ViewSyncCommitCertificateSend(view_number={:?})",
                    cert.view_number()
                )
            },
            HotShotEvent::ViewSyncFinalizeCertificateSend(cert, _) => {
                write!(
                    f,
                    "ViewSyncFinalizeCertificateSend(view_number={:?})",
                    cert.view_number()
                )
            },
            HotShotEvent::ViewSyncTrigger(view_number) => {
                write!(f, "ViewSyncTrigger(view_number={view_number:?})")
            },
            HotShotEvent::Timeout(view_number, epoch) => {
                write!(f, "Timeout(view_number={view_number:?}, epoch={epoch:?})")
            },
            HotShotEvent::TransactionsRecv(_) => write!(f, "TransactionsRecv"),
            HotShotEvent::TransactionSend(..) => write!(f, "TransactionSend"),
            HotShotEvent::SendPayloadCommitmentAndMetadata(_, _, _, view_number, ..) => {
                write!(
                    f,
                    "SendPayloadCommitmentAndMetadata(view_number={view_number:?})"
                )
            },
            HotShotEvent::BlockRecv(packed_bundle) => {
                write!(f, "BlockRecv(view_number={:?})", packed_bundle.view_number)
            },
            HotShotEvent::VidDisperseSend(proposal, _) => write!(
                f,
                "VidDisperseSend(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::VidShareRecv(_, proposal) => write!(
                f,
                "VIDShareRecv(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::VidShareValidated(proposal) => write!(
                f,
                "VIDShareValidated(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::UpgradeProposalRecv(proposal, _) => write!(
                f,
                "UpgradeProposalRecv(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::UpgradeProposalSend(proposal, _) => write!(
                f,
                "UpgradeProposalSend(view_number={:?})",
                proposal.data.view_number()
            ),
            HotShotEvent::UpgradeVoteRecv(vote) => {
                write!(f, "UpgradeVoteRecv(view_number={:?})", vote.view_number())
            },
            HotShotEvent::UpgradeVoteSend(vote) => {
                write!(f, "UpgradeVoteSend(view_number={:?})", vote.view_number())
            },
            HotShotEvent::UpgradeCertificateFormed(cert) => write!(
                f,
                "UpgradeCertificateFormed(view_number={:?})",
                cert.view_number()
            ),
            HotShotEvent::QuorumProposalRequestSend(view_number, _) => {
                write!(f, "QuorumProposalRequestSend(view_number={view_number:?})")
            },
            HotShotEvent::QuorumProposalRequestRecv(view_number, _) => {
                write!(f, "QuorumProposalRequestRecv(view_number={view_number:?})")
            },
            HotShotEvent::QuorumProposalResponseSend(_, proposal) => {
                write!(
                    f,
                    "QuorumProposalResponseSend(view_number={:?})",
                    proposal.data.view_number()
                )
            },
            HotShotEvent::QuorumProposalResponseRecv(proposal) => {
                write!(
                    f,
                    "QuorumProposalResponseRecv(view_number={:?})",
                    proposal.data.view_number()
                )
            },
            HotShotEvent::QuorumProposalPreliminarilyValidated(proposal) => {
                write!(
                    f,
                    "QuorumProposalPreliminarilyValidated(view_number={:?}",
                    proposal.data.view_number()
                )
            },
            HotShotEvent::VidRequestSend(request, ..) => {
                write!(f, "VidRequestSend(view_number={:?}", request.view)
            },
            HotShotEvent::VidRequestRecv(request, _) => {
                write!(f, "VidRequestRecv(view_number={:?}", request.view)
            },
            HotShotEvent::VidResponseSend(_, _, proposal) => {
                write!(
                    f,
                    "VidResponseSend(view_number={:?}",
                    proposal.data.view_number()
                )
            },
            HotShotEvent::VidResponseRecv(_, proposal) => {
                write!(
                    f,
                    "VidResponseRecv(view_number={:?}",
                    proposal.data.view_number()
                )
            },
            HotShotEvent::HighQcRecv(qc, ..) => {
                write!(f, "HighQcRecv(view_number={:?}", qc.view_number())
            },
            HotShotEvent::HighQcSend(qc, ..) => {
                write!(f, "HighQcSend(view_number={:?}", qc.view_number())
            },
            HotShotEvent::ExtendedQcRecv(qc, ..) => {
                write!(f, "ExtendedQcRecv(view_number={:?}", qc.view_number())
            },
            HotShotEvent::ExtendedQcSend(qc, ..) => {
                write!(f, "ExtendedQcSend(view_number={:?}", qc.view_number())
            },
            HotShotEvent::EpochRootQcSend(cert, ..) => {
                write!(f, "EpochRootQcSend(view_number={:?}", cert.view_number())
            },
            HotShotEvent::EpochRootQcRecv(cert, ..) => {
                write!(f, "EpochRootQcRecv(view_number={:?}", cert.view_number())
            },
            HotShotEvent::SetFirstEpoch(view, epoch) => {
                write!(
                    f,
                    "SetFirstEpoch(view_number={:?}, epoch={:?})",
                    view, epoch
                )
            },
        }
    }
}
