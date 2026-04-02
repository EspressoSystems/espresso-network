use std::sync::Arc;

use hotshot_types::{
    epoch_membership::EpochMembershipCoordinator, traits::node_implementation::NodeType,
};
use hotshot_utils::anytrace;
use tokio::task::JoinSet;
use tracing::error;

use crate::message::{ProposalMessage, Unchecked, Validated};

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

    pub fn validate(&mut self, p: ProposalMessage<T, Unchecked>, sender: T::SignatureKey) {
        let stake_table_coordinator = self.stake_table_coordinator.clone();
        self.tasks.spawn(async move {
            p.proposal
                .validate_signature(&stake_table_coordinator)
                .await?;
            Ok(ProposalMessage::validated(sender, p.proposal, p.vid_share))
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

#[derive(Debug, thiserror::Error)]
#[error("validation failed: {0}")]
pub struct ValidationError(#[from] anytrace::Error);
