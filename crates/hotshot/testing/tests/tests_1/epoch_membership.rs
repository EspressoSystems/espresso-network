// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

use std::{sync::Arc, time::Duration};

use alloy::primitives::U256;
use hotshot_example_types::{
    membership::TestableMembership, node_types::TestTypes, storage_types::TestStorage,
};
use hotshot_types::{
    ValidatorConfig,
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    traits::{election::Membership, node_implementation::NodeType},
};

const EPOCH_HEIGHT: u64 = 10;
const FIRST_EPOCH: u64 = 1;
/// Far enough above `FIRST_EPOCH` that catchup walks back through epochs it has to
/// read from storage.
const TARGET_EPOCH: u64 = 10;
/// Absolute, not derived from the coordinator's constants: shortening one of those
/// must not make these tests vacuous.
const PATIENCE: Duration = Duration::from_secs(600);

fn build_coordinator() -> EpochMembershipCoordinator<TestTypes> {
    let nodes: Vec<_> = (0..4)
        .map(|i| {
            ValidatorConfig::<TestTypes>::generated_from_seed_indexed(
                [0u8; 32],
                i,
                U256::from(1),
                true,
            )
            .public_config()
        })
        .collect();
    let membership = <TestTypes as NodeType>::Membership::new(
        nodes.clone(),
        nodes,
        ValidatorConfig::<TestTypes>::test_default().public_key,
        EPOCH_HEIGHT,
    );
    membership.set_first_epoch(EpochNumber::new(FIRST_EPOCH), [0u8; 32]);
    EpochMembershipCoordinator::new(
        Arc::new(membership),
        EPOCH_HEIGHT,
        &TestStorage::<TestTypes>::default(),
    )
}

/// A catchup task stalled on a storage read must still release the epoch it
/// claimed. While the claim is held, every later caller is refused and consensus
/// cannot obtain the stake table.
#[test_log::test(tokio::test(start_paused = true))]
async fn stalled_catchup_releases_its_epoch() {
    let coordinator = build_coordinator();
    let gate = coordinator.membership().load_gate();
    let _stall = gate.lock().await;
    let epoch = EpochNumber::new(TARGET_EPOCH);

    let Err(started) = coordinator.stake_table_for_epoch(Some(epoch)) else {
        panic!("stake table is unavailable, so this must start catchup");
    };
    assert!(
        started.message.contains("Starting catchup"),
        "expected a fresh catchup, got: {started:?}"
    );

    tokio::time::timeout(PATIENCE, async {
        while coordinator.is_catching_up(epoch) {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
    .await
    .expect("the stalled catchup releases its epoch");

    // The stall is still in place, so this must be a new catchup rather than a
    // refusal pointing at the abandoned one.
    let Err(retried) = coordinator.stake_table_for_epoch(Some(epoch)) else {
        panic!("stake table is still unavailable");
    };
    assert!(
        retried.message.contains("Starting catchup"),
        "expected a fresh catchup, got: {retried:?}"
    );
}

/// `wait_for_catchup` is awaited on the proposal validation and reward paths, so it
/// must return when the catchup it waits on cannot finish, rather than parking for
/// the lifetime of the process.
#[test_log::test(tokio::test(start_paused = true))]
async fn wait_for_catchup_returns_instead_of_parking() {
    let coordinator = build_coordinator();
    let gate = coordinator.membership().load_gate();
    let _stall = gate.lock().await;
    let epoch = EpochNumber::new(TARGET_EPOCH);

    assert!(
        coordinator.stake_table_for_epoch(Some(epoch)).is_err(),
        "stake table is unavailable, so this must start catchup"
    );

    let waited = tokio::time::timeout(PATIENCE, coordinator.wait_for_catchup(epoch))
        .await
        .expect("wait_for_catchup returns rather than parking forever");
    assert!(
        waited.is_err(),
        "the stalled catchup cannot produce a stake table"
    );
}
