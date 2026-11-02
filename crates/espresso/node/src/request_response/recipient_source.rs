use std::{collections::BTreeSet, sync::Arc};

use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::{PubKey, SeqTypes};
use hotshot_new_protocol::storage::NewProtocolStorage;
use hotshot_types::{
    data::EpochNumber, epoch_membership::EpochMembershipCoordinator,
    traits::node_implementation::NodeImplementation,
};
use request_response::recipient_source::RecipientSource as RecipientSourceTrait;
use tracing::warn;

use super::request::Request;
use crate::consensus_handle::ConsensusHandle;

#[derive(Clone)]
pub struct RecipientSource<I: NodeImplementation<SeqTypes>> {
    /// The consensus adapter handle
    pub consensus_handle: Arc<ConsensusHandle<SeqTypes, I>>,
    /// A copy of the membership coordinator
    pub memberships: EpochMembershipCoordinator<SeqTypes>,
    /// The public key of the node
    pub public_key: PubKey,
    /// `ST(now)` keys from boot; added to the consensus-epoch table's keys, deduplicated, without
    /// self.
    pub l1_finalized_peers: Vec<PubKey>,
}

/// Implement the RecipientSourceTrait, which allows the request-response protocol to derive the
/// intended recipients for a given request
#[async_trait]
impl<I: NodeImplementation<SeqTypes>> RecipientSourceTrait<Request, PubKey> for RecipientSource<I>
where
    I::Storage: NewProtocolStorage<SeqTypes>,
{
    async fn get_expected_responders(&self, _request: &Request) -> Result<Vec<PubKey>> {
        // Get the current epoch number
        let epoch_number = self
            .consensus_handle
            .current_epoch()
            .await
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

        let table_keys = membership
            .stake_table()
            .map(|entry| entry.stake_table_entry.stake_key);
        Ok(responders(
            table_keys,
            &self.l1_finalized_peers,
            &self.public_key,
        ))
    }
}

/// `table_keys` and `l1_finalized_peers` together, deduplicated, without `own_key`.
fn responders(
    table_keys: impl Iterator<Item = PubKey>,
    l1_finalized_peers: &[PubKey],
    own_key: &PubKey,
) -> Vec<PubKey> {
    let keys: BTreeSet<PubKey> = table_keys
        .chain(l1_finalized_peers.iter().copied())
        .filter(|key| key != own_key)
        .collect();
    keys.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use hotshot_types::{signature_key::BLSPubKey, traits::signature_key::SignatureKey};

    use super::*;

    fn key(i: u64) -> PubKey {
        BLSPubKey::generated_from_seed_indexed([0; 32], i).0
    }

    #[test]
    fn responders_union_without_self() {
        let own = key(0);
        let table = [own, key(1), key(2)];
        let l1_finalized = [key(2), key(3), own];

        let out = responders(table.into_iter(), &l1_finalized, &own);

        let mut expected = vec![key(1), key(2), key(3)];
        expected.sort();
        assert_eq!(out, expected);
    }
}
