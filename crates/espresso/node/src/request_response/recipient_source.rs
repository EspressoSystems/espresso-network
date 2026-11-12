use std::{
    collections::{BTreeSet, HashSet},
    sync::Arc,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use espresso_types::{PubKey, SeqTypes, traits::SequencerPersistence};
use hotshot_new_protocol::network::Sender;
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::{EpochMembership, EpochMembershipCoordinator},
    traits::election::Membership,
};
use request_response::recipient_source::RecipientSource as RecipientSourceTrait;
use tracing::debug;

use super::request::Request;
use crate::consensus_handle::ConsensusHandle;

#[derive(Clone)]
pub struct RecipientSource<P: SequencerPersistence> {
    pub consensus_handle: Arc<ConsensusHandle<SeqTypes, Arc<P>>>,
    /// A copy of the membership coordinator
    pub memberships: EpochMembershipCoordinator<SeqTypes>,
    /// The public key of the node
    pub public_key: PubKey,
    /// Keys of the validators active at the L1 finalized block at startup that the network did not
    /// hold; responders are these and the keys of the consensus-epoch table that the network holds
    /// at request time, deduplicated, without self.
    pub l1_finalized_peers: Vec<PubKey>,
    /// The network, for the keys it holds at request time.
    pub network: Sender<SeqTypes>,
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

        let table_keys = membership
            .stake_table()
            .map(|entry| entry.stake_table_entry.stake_key);
        Ok(responders(
            table_keys,
            &self.l1_finalized_peers,
            &self.network.peer_keys(),
            &self.public_key,
        ))
    }
}

/// The `table_keys` and `l1_finalized_peers` that are in `held`, deduplicated, without `own_key`.
fn responders(
    table_keys: impl Iterator<Item = PubKey>,
    l1_finalized_peers: &[PubKey],
    held: &HashSet<PubKey>,
    own_key: &PubKey,
) -> Vec<PubKey> {
    let keys: BTreeSet<PubKey> = table_keys
        .chain(l1_finalized_peers.iter().copied())
        .filter(|key| held.contains(key) && key != own_key)
        .collect();
    keys.into_iter().collect()
}

/// The stake table to draw responders from: the current epoch's, else the previous epoch's, else
/// the newest one this node holds.
///
/// The newest covers a node whose consensus is below the epochs it holds tables for, such as one
/// started on an empty database at the genesis epoch. It always resolves, since the genesis pair is
/// never pruned. Without it every request fails before it is sent, including the ones catchup
/// makes to bring the node to the network's epoch.
fn responder_membership(
    memberships: &EpochMembershipCoordinator<SeqTypes>,
    current_epoch: EpochNumber,
) -> Result<EpochMembership<SeqTypes>> {
    let previous_epoch = EpochNumber::new(current_epoch.saturating_sub(1));
    let newest_epoch = memberships.membership().highest_known_epoch();
    [Some(current_epoch), Some(previous_epoch), newest_epoch]
        .into_iter()
        .flatten()
        .find_map(|epoch| {
            memberships
                .stake_table_for_epoch(Some(epoch))
                .inspect_err(|err| debug!(%epoch, %err, "no stake table to draw responders from"))
                .ok()
        })
        .with_context(|| {
            format!(
                "no stake table for epoch {current_epoch}, epoch {previous_epoch} or the newest \
                 epoch held ({newest_epoch:?})"
            )
        })
}

#[cfg(test)]
mod tests {
    use alloy::primitives::U256;
    use espresso_types::{EpochCommittees, v0_3::Fetcher};
    use hotshot_example_types::storage_types::TestStorage;
    use hotshot_types::{
        PeerConfig, ValidatorConfig, signature_key::BLSPubKey, traits::signature_key::SignatureKey,
    };

    use super::*;

    /// Decaf's first epoch, far above the genesis epoch.
    const FIRST_EPOCH: u64 = 1054;

    const EPOCH_HEIGHT: u64 = 100;

    const STAKERS: u64 = 4;

    fn key(i: u64) -> PubKey {
        BLSPubKey::generated_from_seed_indexed([0; 32], i).0
    }

    #[test]
    fn responders_union_of_held_without_self() {
        let own = key(0);
        let table = [own, key(1), key(2)];
        let l1_finalized = [key(2), key(3), key(4), own];
        let held = HashSet::from([key(1), key(3), own]);

        let out = responders(table.into_iter(), &l1_finalized, &held, &own);

        let mut expected = vec![key(1), key(3)];
        expected.sort();
        assert_eq!(out, expected);
    }

    /// A network that holds no peer yields no responders.
    #[test]
    fn responders_without_held_peers_are_empty() {
        let own = key(0);

        let out = responders([key(1)].into_iter(), &[key(2)], &HashSet::new(), &own);

        assert!(out.is_empty());
    }

    /// A membership holding the stake tables for `FIRST_EPOCH` and the epoch after it.
    fn memberships() -> EpochMembershipCoordinator<SeqTypes> {
        let peers: Vec<PeerConfig<SeqTypes>> = (0..STAKERS)
            .map(|i| {
                ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
                    [7u8; 32],
                    i,
                    U256::from(100),
                    true,
                )
                .public_config()
            })
            .collect();
        let committees = EpochCommittees::new(
            peers.clone(),
            peers,
            None,
            Fetcher::mock(),
            EPOCH_HEIGHT,
            EpochNumber::new(FIRST_EPOCH),
        );
        EpochMembershipCoordinator::new(committees, EPOCH_HEIGHT, &TestStorage::default())
    }

    /// A node started on an empty database has consensus at the genesis epoch, below the first
    /// epoch, so neither that epoch nor the one before it has a stake table. Its responders must
    /// still come from the stake tables it holds, or every request fails before it is sent.
    #[tokio::test]
    async fn responders_before_the_first_epoch_come_from_the_newest_stake_table() {
        let membership = responder_membership(&memberships(), EpochNumber::genesis()).unwrap();

        assert_eq!(membership.epoch(), Some(EpochNumber::new(FIRST_EPOCH + 1)));
        assert_eq!(membership.stake_table().len(), STAKERS as usize);
    }

    #[tokio::test]
    async fn responders_come_from_the_current_epoch_when_its_stake_table_is_held() {
        let membership =
            responder_membership(&memberships(), EpochNumber::new(FIRST_EPOCH)).unwrap();

        assert_eq!(membership.epoch(), Some(EpochNumber::new(FIRST_EPOCH)));
    }
}
