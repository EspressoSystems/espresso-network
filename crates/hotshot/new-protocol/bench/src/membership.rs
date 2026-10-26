use std::sync::Arc;

use hotshot_example_types::{
    membership::{
        TestableMembership, static_committee::StaticStakeTable, strict_membership::StrictMembership,
    },
    node_stake::{TestNodeStakes, gen_node_lists},
    node_types::TestTypes,
    storage_types::TestStorage,
};
use hotshot_new_protocol::client::{ClientLeafFetcherNetwork, CoordinatorClient};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    signature_key::{BLSPubKey, SchnorrPubKey},
    traits::election::Membership,
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

    let membership = StrictMembership::<TestTypes, StaticStakeTable<BLSPubKey, SchnorrPubKey>>::new(
        members.clone(),
        members.clone(),
        public_key,
        u64::MAX,
    );
    // Bench doesn't drive catchup events into the fetcher; install a
    // disconnected receiver so the fetcher's listener is wired but idle.
    let (_tx, rx) = async_broadcast::broadcast(1);
    membership.set_leaf_fetcher(leaf_fetcher_network, TestStorage::default(), public_key, rx);

    membership.set_first_epoch(EpochNumber::genesis(), [0u8; 32]);

    let coordinator =
        EpochMembershipCoordinator::new(membership, u64::MAX, &TestStorage::<TestTypes>::default());
    (coordinator, client)
}
