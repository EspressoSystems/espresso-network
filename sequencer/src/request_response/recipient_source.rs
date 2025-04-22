use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::{PubKey, SeqTypes};
use hotshot::{traits::NodeImplementation, SystemContext};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::{ConsensusTime, Versions},
};
use request_response::recipient_source::RecipientSource as RecipientSourceTrait;
use tracing::warn;

use super::request::Request;

/// A type alias for the consensus context
type Consensus<I, V> = Arc<SystemContext<SeqTypes, I, V>>;

#[derive(Clone)]
pub struct RecipientSource<I: NodeImplementation<SeqTypes>, V: Versions> {
    /// A copy of the consensus context
    pub consensus: Consensus<I, V>,
    /// A copy of the membership coordinator
    pub memberships: EpochMembershipCoordinator<SeqTypes>,
    /// The public key of the node
    pub public_key: PubKey,
}

/// Implement the RecipientSourceTrait, which allows the request-response protocol to derive the
/// intended recipients for a given request
#[async_trait]
impl<I: NodeImplementation<SeqTypes>, V: Versions> RecipientSourceTrait<Request, PubKey>
    for RecipientSource<I, V>
{
    async fn get_expected_responders(&self, _request: &Request) -> Result<Vec<PubKey>> {
        // Get the current epoch number
        let epoch_number = self
            .consensus
            .consensus()
            .read()
            .await
            .cur_epoch()
            .unwrap_or(EpochNumber::genesis());

        // Attempt to get the membership for the current epoch
        let membership = match self
            .memberships
            .membership_for_epoch(Some(epoch_number))
            .await
        {
            Ok(membership) => membership,
            Err(e) => {
                warn!(
                    "Failed to get membership for epoch {}: {e:#}. Failing over to genesis",
                    epoch_number
                );
                self.memberships
                    .membership_for_epoch(Some(EpochNumber::genesis()))
                    .await
                    .with_context(|| "failed to get stake table for epoch")?
            },
        };

        // Sum all participants in the membership
        Ok(membership
            .stake_table()
            .await
            .iter()
            .map(|entry| entry.stake_table_entry.stake_key)
            .filter(|key| *key != self.public_key)
            .collect())
    }
}
