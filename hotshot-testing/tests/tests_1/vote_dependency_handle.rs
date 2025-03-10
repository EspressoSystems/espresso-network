use std::time::Duration;

use async_broadcast::broadcast;
use futures::StreamExt;
use hotshot_example_types::{
    node_types::{MemoryImpl, TestTypes, TestVersions},
    state_types::TestValidatedState,
};
use hotshot_task::dependency_task::HandleDepOutput;
use hotshot_task_impls::{events::HotShotEvent::*, quorum_vote::VoteDependencyHandle};
use hotshot_testing::{
    helpers::build_system_handle,
    predicates::{event::*, Predicate, PredicateResult},
    view_generator::TestViewGenerator,
};
use hotshot_types::{
    consensus::OuterConsensus,
    data::{Leaf2, ViewNumber},
    traits::{consensus_api::ConsensusApi, node_implementation::ConsensusTime},
};
use itertools::Itertools;
use tokio::time::timeout;

const TIMEOUT: Duration = Duration::from_millis(35);

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn test_vote_dependency_handle() {
    use std::sync::Arc;

    hotshot::helpers::initialize_logging();

    // We use a node ID of 2 here arbitrarily. We just need it to build the system handle.
    let node_id = 2;
    // Construct the system handle for the node ID to build all of the state objects.
    let (handle, _, _, node_key_map) =
        build_system_handle::<TestTypes, MemoryImpl, TestVersions>(node_id).await;
    let membership = handle.hotshot.membership_coordinator.clone();
    let mut generator = TestViewGenerator::<TestVersions>::generate(membership, node_key_map);

    // Generate our state for the test
    let mut proposals = Vec::new();
    let mut leaves = Vec::new();
    let mut dacs = Vec::new();
    let mut vids = Vec::new();
    let consensus = handle.hotshot.consensus().clone();
    let mut consensus_writer = consensus.write().await;
    for view in (&mut generator).take(3).collect::<Vec<_>>().await {
        proposals.push(view.quorum_proposal.clone());
        leaves.push(view.leaf.clone());
        dacs.push(view.da_certificate.clone());
        vids.push(view.vid_proposal.clone());
        consensus_writer
            .update_leaf(
                Leaf2::from_quorum_proposal(&view.quorum_proposal.data),
                Arc::new(TestValidatedState::default()),
                None,
            )
            .unwrap();
    }
    drop(consensus_writer);

    // We permute all possible orderings of inputs. Ordinarily we'd use `random!` for this, but
    // the dependency handles do not (yet) work with the existing test suite.
    let all_inputs = vec![
        DaCertificateValidated(dacs[1].clone()),
        QuorumProposalValidated(proposals[1].clone(), leaves[0].clone()),
        VidShareValidated(vids[1].0[0].clone()),
    ]
    .into_iter()
    .permutations(3);

    // For each permutation...
    for inputs in all_inputs.into_iter() {
        // The outputs are static here, but we re-make them since we use `into_iter` below
        let outputs = vec![
            exact(ViewChange(ViewNumber::new(3), None)),
            quorum_vote_send(),
        ];

        let (event_sender, mut event_receiver) = broadcast(1024);
        let view_number = ViewNumber::new(node_id);

        let vote_dependency_handle_state =
            VoteDependencyHandle::<TestTypes, MemoryImpl, TestVersions> {
                public_key: handle.public_key(),
                private_key: handle.private_key().clone(),
                consensus: OuterConsensus::new(consensus.clone()),
                consensus_metrics: Arc::clone(&consensus.read().await.metrics),
                instance_state: handle.hotshot.instance_state(),
                membership_coordinator: handle.hotshot.membership_coordinator.clone(),
                storage: Arc::clone(&handle.storage()),
                view_number,
                sender: event_sender.clone(),
                receiver: event_receiver.clone().deactivate(),
                upgrade_lock: handle.hotshot.upgrade_lock.clone(),
                id: handle.hotshot.id,
                epoch_height: handle.hotshot.config.epoch_height,
            };

        vote_dependency_handle_state
            .handle_dep_result(inputs.clone().into_iter().map(|i| i.into()).collect())
            .await;

        // We need to avoid re-processing the inputs during our output evaluation. This part here is not
        // strictly necessary, but it makes writing the outputs easier.
        let mut output_events = vec![];
        while let Ok(Ok(received_output)) = timeout(TIMEOUT, event_receiver.recv_direct()).await {
            output_events.push(received_output);
        }

        assert_eq!(
            output_events.len(),
            outputs.len(),
            "Output event count differs from expected"
        );

        // Finally, evaluate that the test does what we expected. The control flow of the handle always
        // outputs in the same order.
        for (check, real) in outputs.into_iter().zip(output_events) {
            if check.evaluate(&real).await == PredicateResult::Fail {
                panic!("Output {real} did not match expected output {check:?}");
            }
        }
    }
}
