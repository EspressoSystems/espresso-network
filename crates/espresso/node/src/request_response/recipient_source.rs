use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::{PubKey, SeqTypes};
use hotshot_new_protocol::client::ClientApi;
use hotshot_types::{data::EpochNumber, epoch_membership::EpochMembershipCoordinator};
use request_response::recipient_source::RecipientSource as RecipientSourceTrait;
use tracing::warn;

use super::request::Request;

#[derive(Clone)]
pub struct RecipientSource {
    pub client_api: ClientApi<SeqTypes>,
    /// A copy of the membership coordinator
    pub memberships: EpochMembershipCoordinator<SeqTypes>,
    /// The public key of the node
    pub public_key: PubKey,
}

/// Implement the RecipientSourceTrait, which allows the request-response protocol to derive the
/// intended recipients for a given request
#[async_trait]
impl RecipientSourceTrait<Request, PubKey> for RecipientSource {
    async fn get_expected_responders(&self, _request: &Request) -> Result<Vec<PubKey>> {
        let epoch_number = self
            .client_api
            .current_epoch()
            .await
            .context("failed to read the current epoch from the coordinator")?
            .unwrap_or(EpochNumber::genesis());

        // Attempt to get the membership for the current epoch
        let membership = match self.memberships.stake_table_for_epoch(Some(epoch_number)) {
            Ok(membership) => membership,
            Err(e) => {
                warn!(
                    "Failed to get membership for epoch {}: {e:#}. Failing over to previous epoch",
                    epoch_number
                );
                let prev_epoch = epoch_number.saturating_sub(1);
                self.memberships
                    .stake_table_for_epoch(Some(EpochNumber::new(prev_epoch)))
                    .with_context(|| "failed to get stake table for epoch")?
            },
        };

        // Sum all participants in the membership
        Ok(membership
            .stake_table()
            .map(|entry| entry.stake_table_entry.stake_key)
            .filter(|key| *key != self.public_key)
            .collect())
    }
}
