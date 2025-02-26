use std::sync::Arc;

use async_lock::RwLock;
use async_trait::async_trait;
use espresso_types::{PubKey, SeqTypes};
use hotshot_types::{
    data::EpochNumber,
    traits::{
        election::Membership,
        node_implementation::{ConsensusTime, NodeType},
    },
};
use request_response::recipient_source::RecipientSource as RecipientSourceTrait;

use super::request::Request;

#[derive(Clone, Debug)]
pub struct RecipientSource {
    pub memberships: Arc<RwLock<<SeqTypes as NodeType>::Membership>>,
}

/// Implement the RecipientSourceTrait, which allows the request-response protocol to derive the
/// intended recipients for a given request
#[async_trait]
impl RecipientSourceTrait<Request, PubKey> for RecipientSource {
    async fn get_recipients_for(&self, request: &Request) -> Vec<PubKey> {
        match request {
            Request::Example => {
                // Get the memberships
                let memberships = self.memberships.read().await;

                // Get everyone in the stake table
                memberships
                    .stake_table(Some(EpochNumber::new(0)))
                    .iter()
                    .map(|entry| entry.stake_key)
                    .collect()
            }
        }
    }
}
