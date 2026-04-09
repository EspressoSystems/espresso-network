use std::{collections::HashSet, sync::Arc, time::Duration};

use committable::Committable;
use hotshot::{
    traits::{
        NodeImplementation,
        implementations::{MasterMap, MemoryNetwork},
    },
    types::BLSPubKey,
};
use hotshot_example_types::{
    block_types::TestTransaction,
    node_types::{TEST_VERSIONS, TestTypes},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{EpochNumber, Leaf2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{network::Topic, signature_key::SignatureKey},
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::timeout,
};
use tracing::{debug, info};

use crate::{
    block::{BlockBuilder, BlockBuilderConfig},
    consensus::{Consensus, ConsensusOutput},
    coordinator::{Coordinator, error::Severity, timer::Timer},
    epoch::EpochManager,
    helpers::upgrade_lock,
    message::{BlockMessage, Message, MessageType, TransactionMessage, Validated},
    network::Network,
    outbox::Outbox,
    proposal::ProposalValidator,
    state::StateManager,
    tests::common::utils::{TestData, mock_membership_with_num_nodes},
    vid::{VidDisperser, VidReconstructor},
    vote::VoteCollector,
};

const NUM_NODES: usize = 5;
const TARGET_DECISIONS: usize = 100;
const MAX_RUNTIME: Duration = Duration::from_secs(60);
const TRANSACTION_SIZE: usize = 64 * 1024;

#[tokio::test(flavor = "multi_thread")]
async fn five_nodes_decide_same_chain_over_memory_network() {
    let membership = mock_membership_with_num_nodes(NUM_NODES).await;
    let group: Arc<MasterMap<BLSPubKey>> = MasterMap::new();

    let mut input_channels: Vec<UnboundedSender<Message<TestTypes, Validated>>> =
        Vec::with_capacity(NUM_NODES);
    let mut output_channels: Vec<UnboundedReceiver<Vec<[u8; 32]>>> = Vec::with_capacity(NUM_NODES);

    // Spawn one coordinator task per node.
    for i in 0..NUM_NODES {
        let (input_tx, mut input_rx) = mpsc::unbounded_channel();
        let (output_tx, output_rx) = mpsc::unbounded_channel();
        input_channels.push(input_tx);
        output_channels.push(output_rx);

        let coord = build_coordinator(i as u64, group.clone(), membership.clone()).await;

        tokio::spawn(async move {
            let mut coord = coord;
            let mut commits: Vec<[u8; 32]> = Vec::new();
            let mut seen: HashSet<[u8; 32]> = HashSet::new();
            let mut last_view = ViewNumber::genesis();

            loop {
                // Drain any externally injected messages (bootstrap + transactions).
                while let Ok(msg) = input_rx.try_recv() {
                    if let Some(input) = coord.on_network_message(msg.into_unchecked()).await {
                        coord.apply_consensus(input).await;
                    }
                }

                match coord.next_consensus_input().await {
                    Ok(input) => coord.apply_consensus(input).await,
                    Err(err) if err.severity == Severity::Critical => break,
                    Err(_) => continue,
                };

                while let Some(output) = coord.outbox_mut().pop_front() {
                    if let ConsensusOutput::LeafDecided(leaves) = &output {
                        for leaf in leaves {
                            let commit: [u8; 32] = leaf.commit().into();
                            if seen.insert(commit) {
                                info!(
                                    node = %coord.node_id(),
                                    view = %leaf.view_number(),
                                    height = %leaf.height(),
                                    "decided leaf"
                                );
                                commits.push(commit);
                            }
                        }
                        let _ = output_tx.send(commits.clone());
                    } else if let ConsensusOutput::ViewChanged(view, epoch) = &output
                        && *view > last_view
                    {
                        debug!(
                            node = %coord.node_id(),
                            view = %view,
                            epoch = %epoch,
                            "view changed"
                        );
                        last_view = *view;
                    }

                    let _ = coord.process_consensus_output(output).await;
                }
            }
        });
    }

    // Bootstrap: inject the first proposal (with per-recipient VID share).
    let test_data = TestData::new_with_num_nodes(3, NUM_NODES).await;
    for (i, chan) in input_channels.iter().enumerate() {
        let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], i as u64).0;
        let proposal_msg = test_data.views[0].proposal_input(&node_key);
        chan.send(proposal_msg).unwrap();
    }

    // Collect decided commits from each node until all reach the target.
    let mut node_commits: Vec<Vec<[u8; 32]>> = vec![Vec::new(); NUM_NODES];
    let mut progress: usize = 0;
    let mut tx_nonce: u64 = 0;

    let deadline = tokio::time::Instant::now() + MAX_RUNTIME;
    while node_commits.iter().any(|s| s.len() < TARGET_DECISIONS) {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .expect("timed out waiting for all nodes to decide");

        // Feed a new transaction to every node.
        tx_nonce = tx_nonce.wrapping_add(1);
        broadcast_transaction(&input_channels, random_transaction(tx_nonce));

        // Collect any newly decided commits.
        for (idx, rx) in output_channels.iter_mut().enumerate() {
            if node_commits[idx].len() >= TARGET_DECISIONS {
                continue;
            }
            if let Ok(Some(seq)) = timeout(remaining, rx.recv()).await {
                let have = node_commits[idx].len();
                let common = have.min(seq.len());
                assert_eq!(
                    &node_commits[idx][..common],
                    &seq[..common],
                    "node {idx} decided prefix diverged"
                );
                if seq.len() > have {
                    node_commits[idx].extend_from_slice(&seq[have..]);
                }
            }
        }

        let new_progress = node_commits.iter().map(|s| s.len()).min().unwrap_or(0);
        if new_progress > progress {
            progress = new_progress;
            info!("decided_counter={progress}/{TARGET_DECISIONS}");
        }
    }

    // Verify all nodes decided the same chain.
    let expected = &node_commits[0][..TARGET_DECISIONS];
    for (i, seq) in node_commits.iter().enumerate().skip(1) {
        assert_eq!(
            expected,
            &seq[..TARGET_DECISIONS],
            "node {i} decided a different chain prefix"
        );
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
struct MemoryNetworkImpl;

impl NodeImplementation<TestTypes> for MemoryNetworkImpl {
    type Network = MemoryNetwork<BLSPubKey>;
    type Storage = TestStorage<TestTypes>;
}

async fn build_coordinator(
    node_index: u64,
    group: Arc<MasterMap<BLSPubKey>>,
    membership: EpochMembershipCoordinator<TestTypes>,
) -> Coordinator<TestTypes, MemoryNetworkImpl> {
    let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], node_index);
    let instance = Arc::new(TestInstanceState::default());

    let epoch_manager = EpochManager::new(1000, membership.clone());

    let vote1_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let vote2_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let timeout_one_honest_collector = VoteCollector::new(membership.clone(), upgrade_lock());
    let checkpoint_collector = VoteCollector::new(membership.clone(), upgrade_lock());

    let consensus = Consensus::new(membership.clone(), public_key, private_key.clone(), 1000);

    let vid_disperser = VidDisperser::new(membership.clone());
    let vid_reconstructor = VidReconstructor::new();

    let block_builder = BlockBuilder::new(
        instance.clone(),
        membership.clone(),
        BlockBuilderConfig::default(),
    );

    let mut state_manager = StateManager::new(instance.clone());
    let genesis_state = TestValidatedState::default();
    let genesis_leaf =
        Leaf2::<TestTypes>::genesis(&genesis_state, &instance, TEST_VERSIONS.test.base).await;
    state_manager.seed_state(ViewNumber::genesis(), Arc::new(genesis_state), genesis_leaf);

    let proposal_validator = ProposalValidator::new(membership.clone());

    let network = MemoryNetwork::new(&public_key, &group, &[Topic::Global], None);
    let network = Network::new(network, membership.clone(), upgrade_lock());

    Coordinator::builder()
        .consensus(consensus)
        .network(network)
        .state_manager(state_manager)
        .vote1_collector(vote1_collector)
        .vote2_collector(vote2_collector)
        .timeout_collector(timeout_collector)
        .timeout_one_honest_collector(timeout_one_honest_collector)
        .checkpoint_collector(checkpoint_collector)
        .vid_disperser(vid_disperser)
        .vid_reconstructor(vid_reconstructor)
        .epoch_manager(epoch_manager)
        .block_builder(block_builder)
        .proposal_validator(proposal_validator)
        .membership_coordinator(membership)
        .outbox(Outbox::new())
        .timer(Timer::new(
            Duration::from_secs(2),
            ViewNumber::genesis(),
            EpochNumber::genesis(),
        ))
        .public_key(public_key)
        .build()
}

fn random_transaction(nonce: u64) -> TestTransaction {
    let mut bytes = vec![0u8; TRANSACTION_SIZE];
    let mut rng = StdRng::seed_from_u64(nonce);
    rng.fill_bytes(&mut bytes);
    bytes[TRANSACTION_SIZE - 8..].copy_from_slice(&nonce.to_le_bytes());
    TestTransaction::new(bytes)
}

fn broadcast_transaction(
    channels: &[UnboundedSender<Message<TestTypes, Validated>>],
    tx: TestTransaction,
) {
    for (i, ch) in channels.iter().enumerate() {
        let (pk, _) = BLSPubKey::generated_from_seed_indexed([0; 32], i as u64);
        ch.send(Message {
            sender: pk,
            message_type: MessageType::Block(BlockMessage::Transactions(TransactionMessage {
                view: ViewNumber::genesis(),
                transactions: vec![tx.clone()],
            })),
        })
        .expect("input channel closed");
    }
}
