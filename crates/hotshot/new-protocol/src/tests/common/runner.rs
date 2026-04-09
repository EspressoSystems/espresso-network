use std::{collections::HashSet, fmt, time::Duration};

use committable::Committable;
use hotshot::{traits::NodeImplementation, types::BLSPubKey};
use hotshot_example_types::{block_types::TestTransaction, node_types::TestTypes};
use hotshot_types::{data::ViewNumber, traits::signature_key::SignatureKey};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::timeout,
};
use tracing::{debug, info};

use crate::{
    consensus::ConsensusOutput,
    coordinator::{Coordinator, error::Severity},
    message::{BlockMessage, Message, MessageType, TransactionMessage, Validated},
    tests::common::{
        coordinator_builder::build_test_coordinator,
        network::TestNetwork,
        utils::{TestData, mock_membership_with_num_nodes},
    },
};

/// Configuration for a multi-node integration test.
pub struct TestRunner {
    /// Number of nodes in the test network.
    pub num_nodes: usize,
    /// Number of leaves each node must decide before the test passes.
    pub target_decisions: usize,
    /// Maximum wall-clock time before the test is considered failed.
    pub max_runtime: Duration,
    /// Epoch height passed to each coordinator (0 = no epochs).
    pub epoch_height: u64,
    /// Per-node view timeout duration.
    pub view_timeout: Duration,
    /// Size (in bytes) of each random transaction.
    pub transaction_size: usize,
}

impl Default for TestRunner {
    fn default() -> Self {
        Self {
            num_nodes: 5,
            target_decisions: 100,
            max_runtime: Duration::from_secs(60),
            epoch_height: 1000,
            view_timeout: Duration::from_secs(2),
            transaction_size: 64 * 1024,
        }
    }
}

#[derive(Debug)]
pub enum TestError {
    Timeout,
    ChainDivergence { node: usize },
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "timed out waiting for all nodes to decide"),
            Self::ChainDivergence { node } => {
                write!(f, "node {node} decided a different chain prefix")
            },
        }
    }
}

impl TestRunner {
    /// Run the integration test using the given network backend.
    ///
    /// This spins up `self.num_nodes` coordinators connected via `N`, bootstraps
    /// consensus with an initial proposal, feeds transactions, and waits until
    /// every node has decided `self.target_decisions` leaves.  Finally it verifies
    /// that all nodes decided the same chain prefix.
    pub async fn run<N: TestNetwork>(&self) -> Result<(), TestError> {
        let membership = mock_membership_with_num_nodes(self.num_nodes).await;
        let (_network_state, networks) = N::create(self.num_nodes).await;

        let mut input_channels: Vec<UnboundedSender<Message<TestTypes, Validated>>> =
            Vec::with_capacity(self.num_nodes);
        let mut output_channels: Vec<UnboundedReceiver<Vec<[u8; 32]>>> =
            Vec::with_capacity(self.num_nodes);

        // Spawn one coordinator task per node.
        for (i, network) in networks.into_iter().enumerate() {
            let (input_tx, input_rx) = mpsc::unbounded_channel();
            let (output_tx, output_rx) = mpsc::unbounded_channel();
            input_channels.push(input_tx);
            output_channels.push(output_rx);

            let coord = build_test_coordinator::<N::Impl>(
                i as u64,
                network,
                membership.clone(),
                self.epoch_height,
                self.view_timeout,
            )
            .await;

            tokio::spawn(run_node(coord, input_rx, output_tx));
        }

        // Bootstrap: inject the first proposal (with per-recipient VID share).
        let test_data = TestData::new_with_num_nodes(3, self.num_nodes).await;
        for (i, chan) in input_channels.iter().enumerate() {
            let node_key = BLSPubKey::generated_from_seed_indexed([0; 32], i as u64).0;
            let proposal_msg = test_data.views[0].proposal_input(&node_key);
            chan.send(proposal_msg).unwrap();
        }

        // Collect decided commits from each node until all reach the target.
        let mut node_commits: Vec<Vec<[u8; 32]>> = vec![Vec::new(); self.num_nodes];
        let mut progress: usize = 0;
        let mut tx_nonce: u64 = 0;

        let deadline = tokio::time::Instant::now() + self.max_runtime;
        while node_commits.iter().any(|s| s.len() < self.target_decisions) {
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .ok_or(TestError::Timeout)?;

            // Feed a new transaction to every node.
            tx_nonce = tx_nonce.wrapping_add(1);
            broadcast_transaction(
                &input_channels,
                random_transaction(tx_nonce, self.transaction_size),
            );

            // Collect any newly decided commits.
            for (idx, rx) in output_channels.iter_mut().enumerate() {
                if node_commits[idx].len() >= self.target_decisions {
                    continue;
                }
                if let Ok(Some(seq)) = timeout(remaining, rx.recv()).await {
                    let have = node_commits[idx].len();
                    let common = have.min(seq.len());
                    if node_commits[idx][..common] != seq[..common] {
                        return Err(TestError::ChainDivergence { node: idx });
                    }
                    if seq.len() > have {
                        node_commits[idx].extend_from_slice(&seq[have..]);
                    }
                }
            }

            let new_progress = node_commits.iter().map(|s| s.len()).min().unwrap_or(0);
            if new_progress > progress {
                progress = new_progress;
                info!("decided_counter={progress}/{}", self.target_decisions);
            }
        }

        // Verify all nodes decided the same chain.
        let expected = &node_commits[0][..self.target_decisions];
        for (i, seq) in node_commits.iter().enumerate().skip(1) {
            if expected != &seq[..self.target_decisions] {
                return Err(TestError::ChainDivergence { node: i });
            }
        }

        Ok(())
    }
}

/// Event loop for a single node.  Processes coordinator inputs, collects
/// decided leaf commits, and forwards them to the test runner.
async fn run_node<I: NodeImplementation<TestTypes>>(
    mut coord: Coordinator<TestTypes, I>,
    mut input_rx: UnboundedReceiver<Message<TestTypes, Validated>>,
    output_tx: UnboundedSender<Vec<[u8; 32]>>,
) {
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
}

fn random_transaction(nonce: u64, size: usize) -> TestTransaction {
    let mut bytes = vec![0u8; size];
    let mut rng = StdRng::seed_from_u64(nonce);
    rng.fill_bytes(&mut bytes);
    if size >= 8 {
        bytes[size - 8..].copy_from_slice(&nonce.to_le_bytes());
    }
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
