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

/// Create an `EpochMembershipCoordinator<TestTypes>` with `num_nodes` validators.
pub async fn make_membership(num_nodes: usize) -> EpochMembershipCoordinator<TestTypes> {
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

    let members = gen_node_lists(num_nodes as u64, num_nodes as u64, &TestNodeStakes::default()).0;

    let membership = Arc::new(RwLock::new(StrictMembership::<
        TestTypes,
        StaticStakeTable<BLSPubKey, SchnorrPubKey>,
    >::new::<MemoryImpl>(
        members.clone(),
        members.clone(),
        TestStorage::default(),
        network,
        members[0].stake_table_entry.public_key(),
        u64::MAX,
    )));

    membership
        .write()
        .await
        .set_first_epoch(EpochNumber::genesis(), [0u8; 32]);

    EpochMembershipCoordinator::new(membership, u64::MAX, &TestStorage::<TestTypes>::default())
}
