use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::{PubKey, SeqTypes, traits::SequencerPersistence};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
};
use request_response::recipient_source::RecipientSource as RecipientSourceTrait;
use tracing::warn;

use super::request::Request;
use crate::consensus_handle::ConsensusHandle;

#[derive(Clone)]
pub struct RecipientSource<P: SequencerPersistence> {
    pub consensus_handle: Arc<ConsensusHandle<SeqTypes, Arc<P>>>,
    /// A copy of the membership coordinator
    pub memberships: EpochMembershipCoordinator<SeqTypes>,
    /// The public key of the node
    pub public_key: PubKey,
}

/// Implement the RecipientSourceTrait, which allows the request-response protocol to derive the
/// intended recipients for a given request
#[async_trait]
impl<P: SequencerPersistence> RecipientSourceTrait<Request, PubKey> for RecipientSource<P> {
    async fn get_expected_responders(&self, _request: &Request) -> Result<Vec<PubKey>> {
        let current_epoch = self
            .consensus_handle
            .current_epoch()
            .await
            .unwrap_or(EpochNumber::genesis());
        let membership = responder_membership(&self.memberships, current_epoch)?;

        // Sum all participants in the membership
        Ok(membership
            .stake_table()
            .map(|entry| entry.stake_table_entry.stake_key)
            .filter(|key| *key != self.public_key)
            .collect())
    }
}

/// The stake table to draw responders from: the current epoch's, else the previous epoch's.
fn responder_membership(
    memberships: &EpochMembershipCoordinator<SeqTypes>,
    current_epoch: EpochNumber,
) -> Result<EpochMembership<SeqTypes>> {
    match memberships.stake_table_for_epoch(Some(current_epoch)) {
        Ok(membership) => Ok(membership),
        Err(e) => {
            warn!(
                "Failed to get membership for epoch {}: {e:#}. Failing over to previous epoch",
                current_epoch
            );
            let prev_epoch = current_epoch.saturating_sub(1);
            memberships
                .stake_table_for_epoch(Some(EpochNumber::new(prev_epoch)))
                .with_context(|| "failed to get stake table for epoch")
        },
    }
}
