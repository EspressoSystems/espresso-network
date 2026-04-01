use std::sync::Arc;

use hotshot_types::{
    data::{
        EpochNumber, QuorumProposal2, VidCommitment, VidDisperseShare2, ViewNumber,
        vid_disperse::vid_total_weight,
    },
    epoch_membership::EpochMembershipCoordinator,
    traits::{block_contents::BlockHeader, node_implementation::NodeType},
    vote::HasViewNumber,
};
use hotshot_utils::anytrace;
use tokio::task::JoinSet;
use tracing::error;

use crate::message::{ProposalMessage, Unchecked, Validated};

/// A proposal validator checks proposal signature and integrity.
pub struct ProposalValidator<T: NodeType> {
    tasks: JoinSet<Result<ProposalMessage<T, Validated>, ValidationError>>,
    stake_table_coordinator: Arc<EpochMembershipCoordinator<T>>,
}

impl<T: NodeType> ProposalValidator<T> {
    pub fn new(c: EpochMembershipCoordinator<T>) -> Self {
        Self {
            tasks: JoinSet::new(),
            stake_table_coordinator: Arc::new(c),
        }
    }

    pub fn validate(&mut self, p: ProposalMessage<T, Unchecked>) {
        let stake_table_coordinator = self.stake_table_coordinator.clone();
        self.tasks.spawn(async move {
            p.proposal
                .validate_signature(&stake_table_coordinator)
                .await?;

            if !vid_matches_proposal(&p.vid_share, &p.proposal.data) {
                return Err(ValidationError::VidCommitmentDoesNotMatchProposal);
            }

            if !p.vid_share.is_consistent() {
                return Err(ValidationError::VidShareInconsistent);
            }

            let Some(epoch) = p.proposal.data.epoch else {
                return Err(ValidationError::MissingEpochNumber(p.view_number()));
            };

            verify_vid_share(&stake_table_coordinator, &p.vid_share, epoch).await?;

            Ok(ProposalMessage::validated(p.proposal, p.vid_share))
        });
    }

    pub async fn next(&mut self) -> Option<Result<ProposalMessage<T, Validated>, ValidationError>> {
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

    pub fn num_tasks(&self) -> usize {
        self.tasks.len()
    }
}

fn vid_matches_proposal<T>(share: &VidDisperseShare2<T>, proposal: &QuorumProposal2<T>) -> bool
where
    T: NodeType,
{
    if let VidCommitment::V2(vid_comm) = proposal.block_header.payload_commitment() {
        vid_comm == share.payload_commitment
    } else {
        false
    }
}

async fn verify_vid_share<T: NodeType>(
    coord: &EpochMembershipCoordinator<T>,
    share: &VidDisperseShare2<T>,
    epoch: EpochNumber,
) -> Result<(), ValidationError> {
    match coord.membership_for_epoch(Some(epoch)).await {
        Ok(stake_table) => {
            let total_weight = vid_total_weight(&stake_table.stake_table().await, Some(epoch));
            if share.verify(total_weight) {
                Ok(())
            } else {
                Err(ValidationError::VidShareNotVerified)
            }
        },
        Err(err) => Err(ValidationError::NoMembershipForEpoch(epoch, err)),
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("invalid proposal signature: {0}")]
    Signature(#[from] anytrace::Error),

    #[error("vid share does not match proposal")]
    VidCommitmentDoesNotMatchProposal,

    #[error("inconsistent vid share")]
    VidShareInconsistent,

    #[error("failed to verify vid share")]
    VidShareNotVerified,

    #[error("failed to get membership for epoch {0}: {1}")]
    NoMembershipForEpoch(EpochNumber, anytrace::Error),

    #[error("proposal in view {0} has no epoch number")]
    MissingEpochNumber(ViewNumber),
}
