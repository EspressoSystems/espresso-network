use std::sync::Arc;

use committable::Committable;
use hotshot::types::SignatureKey;
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, VidCommitment, VidDisperseShare2, ViewNumber,
        vid_disperse::vid_total_weight,
    },
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    message::Proposal as SignedProposal,
    simple_vote::HasEpoch,
    stake_table::StakeTableEntries,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::{Certificate, HasViewNumber},
};
use hotshot_utils::anytrace;
use tokio::task::JoinSet;
use tracing::error;

use crate::{
    helpers::upgrade_lock,
    message::{Proposal, ProposalMessage, Unchecked, Validated},
};

type Result<T> = std::result::Result<T, ValidationError>;

/// A proposal validator checks proposal signature and integrity.
pub struct ProposalValidator<T: NodeType> {
    /// Validation tasks.
    tasks: JoinSet<Result<ProposalMessage<T, Validated>>>,

    /// The actual validation logic.
    validator: Arc<Validator<T>>,
}

struct Validator<T: NodeType> {
    membership_coordinator: EpochMembershipCoordinator<T>,
}

impl<T: NodeType> ProposalValidator<T> {
    pub fn new(c: EpochMembershipCoordinator<T>) -> Self {
        Self {
            tasks: JoinSet::new(),
            validator: Arc::new(Validator {
                membership_coordinator: c,
            }),
        }
    }

    pub fn validate(&mut self, p: ProposalMessage<T, Unchecked>) {
        let v = self.validator.clone();
        self.tasks.spawn(async move {
            v.commitments(&p.vid_share, &p.proposal.data)?;
            v.vid_share(&p.vid_share, p.proposal.data.epoch).await?;
            v.signature(&p.proposal).await?;
            v.justify_qc(&p.proposal.data).await?;
            Ok(ProposalMessage::validated(p.proposal, p.vid_share))
        });
    }

    pub async fn next(&mut self) -> Option<Result<ProposalMessage<T, Validated>>> {
        loop {
            match self.tasks.join_next().await {
                Some(Ok(prop)) => return Some(prop),
                Some(Err(err)) => {
                    error!(%err, "proposal validation task panic");
                },
                None => return None,
            }
        }
    }
}

impl<T: NodeType> Validator<T> {
    /// Check that the VID commitment matches the proposal's.
    fn commitments(&self, vid: &VidDisperseShare2<T>, prop: &Proposal<T>) -> Result<()> {
        if let VidCommitment::V2(commitment) = prop.block_header.payload_commitment() {
            if commitment == vid.payload_commitment {
                Ok(())
            } else {
                Err(ValidationError::InvalidVidCommitmentVersion)
            }
        } else {
            Err(ValidationError::VidCommitmentDoesNotMatchProposal)
        }
    }

    /// Verify the VID share.
    async fn vid_share(&self, share: &VidDisperseShare2<T>, epoch: EpochNumber) -> Result<()> {
        let stake_table = self.membership(epoch).await?.stake_table().await;
        if share.verify(vid_total_weight(&stake_table, Some(epoch))) {
            Ok(())
        } else {
            Err(ValidationError::VidShareNotVerified)
        }
    }

    /// Verify the proposal signature.
    async fn signature(&self, proposal: &SignedProposal<T, Proposal<T>>) -> Result<()> {
        let view = proposal.data.view_number();
        let epoch = proposal.data.epoch;
        let membership = self.membership(epoch).await?;
        let leader = match membership.leader(view).await {
            Ok(leader) => leader,
            Err(err) => return Err(ValidationError::NoLeader(view, epoch, err)),
        };
        let leaf: Leaf2<T> = proposal.data.clone().into();
        if leader.validate(&proposal.signature, leaf.commit().as_ref()) {
            Ok(())
        } else {
            Err(ValidationError::InvalidProposalSignature)
        }
    }

    /// Verify the QC of the proposal
    async fn justify_qc(&self, proposal: &Proposal<T>) -> Result<()> {
        let Some(epoch) = proposal.justify_qc.epoch() else {
            return Err(ValidationError::MissingEpoch(
                proposal.view_number,
                "justify_qc",
            ));
        };
        let membership = self.membership(epoch).await?;
        let entries = StakeTableEntries::<T>::from(membership.stake_table().await).0;
        let threshold = membership.success_threshold().await;
        match proposal
            .justify_qc
            .is_valid_cert(&entries, threshold, &upgrade_lock::<T>())
        {
            Ok(()) => Ok(()),
            Err(e) => Err(ValidationError::InvalidJustifyQc(e)),
        }
    }

    async fn membership(&self, epoch: EpochNumber) -> Result<EpochMembership<T>> {
        self.membership_coordinator
            .membership_for_epoch(Some(epoch))
            .await
            .map_err(|err| ValidationError::NoMembershipForEpoch(epoch, err))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("invalid proposal signature")]
    InvalidProposalSignature,

    #[error("invalid proposal justify qc: {0}")]
    InvalidJustifyQc(#[source] anytrace::Error),

    #[error("vid share does not match proposal")]
    VidCommitmentDoesNotMatchProposal,

    #[error("failed to verify vid share")]
    VidShareNotVerified,

    #[error("vid commitment not v2")]
    InvalidVidCommitmentVersion,

    #[error("missing epoch number in view {0} ({1})")]
    MissingEpoch(ViewNumber, &'static str),

    #[error("failed to get membership for epoch {0}: {1}")]
    NoMembershipForEpoch(EpochNumber, #[source] anytrace::Error),

    #[error("failed to get leader for view {0}, epoch {1}: {2}")]
    NoLeader(ViewNumber, EpochNumber, #[source] anytrace::Error),
}
