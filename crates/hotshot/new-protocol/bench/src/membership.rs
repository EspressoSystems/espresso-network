use std::sync::Arc;

use async_lock::RwLock;
use hotshot::types::{BLSPubKey, SchnorrPubKey};
use hotshot_example_types::{
    membership::{static_committee::StaticStakeTable, strict_membership::StrictMembership},
    node_types::TestTypes,
    storage_types::TestStorage,
};
use hotshot_new_protocol::client::{ClientLeafFetcherNetwork, CoordinatorClient};
use hotshot_testing::{node_stake::TestNodeStakes, test_builder::gen_node_lists};
use hotshot_types::{
    data::EpochNumber, epoch_membership::EpochMembershipCoordinator, traits::election::Membership,
};

/// Create an `EpochMembershipCoordinator<TestTypes>` with `num_nodes` validators.
///
/// The membership's `Leaf2Fetcher` routes catchup messages through the
/// returned [`CoordinatorClient`] — this client must be installed on the
/// node's `Coordinator` (`.client(...)`) so messages are dispatched over
/// the Coordinator's owned `Network`.
pub async fn make_membership(
    num_nodes: usize,
    public_key: BLSPubKey,
) -> (
    EpochMembershipCoordinator<TestTypes>,
    CoordinatorClient<TestTypes>,
) {
    let members = gen_node_lists(
        num_nodes as u64,
        num_nodes as u64,
        &TestNodeStakes::default(),
    )
    .0;

    let client = CoordinatorClient::<TestTypes>::default();
    let leaf_fetcher_network = Arc::new(ClientLeafFetcherNetwork::new(client.handle().clone()));

    let membership = Arc::new(RwLock::new(StrictMembership::<
        TestTypes,
        StaticStakeTable<BLSPubKey, SchnorrPubKey>,
    >::new(
        members.clone(),
        members.clone(),
        TestStorage::default(),
        leaf_fetcher_network,
        public_key,
        u64::MAX,
    )));

    membership
        .write()
        .await
        .set_first_epoch(EpochNumber::genesis(), [0u8; 32]);

    let coordinator =
        EpochMembershipCoordinator::new(membership, u64::MAX, &TestStorage::<TestTypes>::default());
    (coordinator, client)
}
