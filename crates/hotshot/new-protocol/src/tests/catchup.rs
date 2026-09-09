//! Stake-table catchup walks up from the epochs we already have, so an epoch
//! nobody can serve costs one fetch attempt at the first gap.

use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use hotshot::types::BLSPubKey;
use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    traits::{leaf_fetcher_network::LeafFetcherNetwork, signature_key::SignatureKey},
};
use tokio::time::timeout;

use crate::tests::common::utils::mock_membership_with_leaf_fetcher_network;

const EPOCH_HEIGHT: u64 = 10;
const NUM_NODES: usize = 2;

/// Counts leaf requests and never answers them, so every fetch attempt runs
/// into `Leaf2Fetcher`'s timeout.
struct CountingFetcherNetwork {
    requests: Arc<AtomicUsize>,
}

#[async_trait]
impl LeafFetcherNetwork<TestTypes> for CountingFetcherNetwork {
    async fn send_leaf_request(
        &self,
        _view: ViewNumber,
        _payload: Vec<u8>,
        _recipient: BLSPubKey,
    ) -> anyhow::Result<()> {
        self.requests.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn send_leaf_response(
        &self,
        _view: ViewNumber,
        _payload: Vec<u8>,
        _recipient: BLSPubKey,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Catchup to an epoch far beyond the chain tip fetches the first epoch we
/// don't have and gives up there. The requested epoch reaches us on
/// certificates that don't commit to it, so it must not scale the work.
#[tokio::test]
async fn catchup_to_unreachable_epoch_stops_at_the_first_gap() {
    let requests = Arc::new(AtomicUsize::new(0));
    let (public_key, _) = BLSPubKey::generated_from_seed_indexed([0u8; 32], 0);
    let (coordinator, _storage, _external_events_tx) = mock_membership_with_leaf_fetcher_network(
        NUM_NODES,
        EPOCH_HEIGHT,
        Arc::new(CountingFetcherNetwork {
            requests: Arc::clone(&requests),
        }),
        public_key,
        TestStorage::default(),
    );

    let result = timeout(
        Duration::from_secs(10),
        coordinator.wait_for_stake_table(EpochNumber::new(u64::MAX)),
    )
    .await
    .expect("catchup gives up instead of walking every epoch below the target");

    assert!(result.is_err());
    // One epoch root fetch, attempted once against each peer.
    assert_eq!(requests.load(Ordering::Relaxed), NUM_NODES);
}
