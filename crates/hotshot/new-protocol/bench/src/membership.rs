use std::{collections::HashMap, sync::Arc, time::Duration};

use async_lock::RwLock;
use hotshot::{
    traits::implementations::MemoryNetwork,
    types::{BLSPubKey, SchnorrPubKey},
};
use hotshot_example_types::{
    membership::{static_committee::StaticStakeTable, strict_membership::StrictMembership},
    node_types::{MemoryImpl, TestTypes},
    storage_types::TestStorage,
};
use hotshot_testing::{node_stake::TestNodeStakes, test_builder::gen_node_lists};
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        election::Membership, network::TestableNetworkingImplementation,
        signature_key::StakeTableEntryType,
    },
};

/// Create an `EpochMembershipCoordinator` with `num_nodes` validators.
///
/// Uses a dummy `MemoryNetwork` internally (only needed for the membership trait
/// constructor). Epoch height is set to `u64::MAX` to effectively disable epoch
/// transitions.
pub async fn make_membership(num_nodes: usize) -> EpochMembershipCoordinator<TestTypes> {
    let n = num_nodes as u64;

    // The membership constructor requires a network instance. We use a MemoryNetwork
    // solely for this purpose — it is not used for actual communication.
    let network =
        <MemoryNetwork<BLSPubKey> as TestableNetworkingImplementation<TestTypes>>::generator(
            num_nodes,
            0,
            1,
            num_nodes,
            None,
            Duration::from_secs(1),
            &mut HashMap::new(),
        )(0)
        .await;

    let members = gen_node_lists(n, n, &TestNodeStakes::default()).0;
    let epoch_height = u64::MAX;

    let membership = Arc::new(RwLock::new(StrictMembership::<
        TestTypes,
        StaticStakeTable<BLSPubKey, SchnorrPubKey>,
    >::new::<MemoryImpl>(
        members.clone(),
        members.clone(),
        TestStorage::default(),
        network,
        members[0].stake_table_entry.public_key(),
        epoch_height,
    )));

    // Initialize epoch data so membership works with epoch-aware versions (VID2 etc.).
    membership
        .write()
        .await
        .set_first_epoch(EpochNumber::genesis(), [0u8; 32]);

    EpochMembershipCoordinator::new(membership, epoch_height, &TestStorage::default())
}
