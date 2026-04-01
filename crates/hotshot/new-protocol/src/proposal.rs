use std::sync::Arc;

use committable::Committable;
use hotshot::types::SignatureKey;
use hotshot_types::{
    data::Leaf2, epoch_membership::EpochMembershipCoordinator, message::Proposal as SignedProposal,
    traits::node_implementation::NodeType, vote::HasViewNumber,
};
use hotshot_utils::{
    anytrace,
    anytrace::{Error, Level},
    line_info,
};
use tokio::task::JoinSet;
use tracing::{error, warn};

use crate::message::{Proposal, ProposalMessage, Unchecked, Validated};

/// A proposal validator checks proposal signatures.
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
            if !validate_proposal_signature(stake_table_coordinator, &p.proposal).await {
                return Err(ValidationError(anytrace::error!(
                    "proposal signature is invalid"
                )));
            }
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
async fn validate_proposal_signature<T: NodeType>(
    stake_table_coordinator: Arc<EpochMembershipCoordinator<T>>,
    proposal: &SignedProposal<T, Proposal<T>>,
) -> bool {
    let view = proposal.data.view_number();
    let epoch = proposal.data.epoch;
    let membership = match stake_table_coordinator
        .membership_for_epoch(Some(epoch))
        .await
    {
        Ok(membership) => membership,
        Err(err) => {
            warn!(%epoch, %err, "failed to get stake table");
            return false;
        },
    };
    let view_leader_key = match membership.leader(view).await {
        Ok(leader) => leader,
        Err(err) => {
            warn!(%view, %epoch, %err, "failed to get leader from stake table");
            return false;
        },
    };
    let proposed_leaf: Leaf2<T> = proposal.data.clone().into();
    let signature = &proposal.signature;
    view_leader_key.validate(signature, proposed_leaf.commit().as_ref())
}

#[derive(Debug, thiserror::Error)]
#[error("validation failed: {0}")]
pub struct ValidationError(#[from] anytrace::Error);
