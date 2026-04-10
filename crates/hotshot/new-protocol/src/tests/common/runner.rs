use std::{collections::BTreeSet, fmt, time::Duration};

use committable::Committable;
use hotshot::{traits::NodeImplementation, types::BLSPubKey};
use hotshot_example_types::{block_types::TestTransaction, node_types::TestTypes};
use hotshot_types::{
    data::ViewNumber,
    traits::{network::ConnectedNetwork, signature_key::SignatureKey},
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    time::timeout,
};
use tracing::{debug, info};

use crate::{
    consensus::ConsensusOutput,
    coordinator::{Coordinator, error::Severity},
    helpers::upgrade_lock,
    message::{BlockMessage, Message, MessageType, TransactionMessage, Validated},
    network::Network,
    tests::common::{
        coordinator_builder::build_test_coordinator, network::TestNetwork,
        utils::mock_membership_with_num_nodes,
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
    /// Spins up `self.num_nodes` coordinators connected via `N`.  Each
    /// coordinator self-starts via the genesis bootstrap (no side-channel
    /// injection needed).  Transactions are broadcast over the network using
    /// a dedicated client network instance.
    pub async fn run<N: TestNetwork>(&self) -> Result<(), TestError> {
        let membership = mock_membership_with_num_nodes(self.num_nodes).await;
        let (network_state, networks) = N::create(self.num_nodes).await;

        let mut output_channels: Vec<UnboundedReceiver<BTreeSet<[u8; 32]>>> =
            Vec::with_capacity(self.num_nodes);

        // Spawn one coordinator task per node.
        for (i, network) in networks.into_iter().enumerate() {
            let (output_tx, output_rx) = mpsc::unbounded_channel();
            output_channels.push(output_rx);

            let coord = build_test_coordinator::<N::Impl>(
                i as u64,
                network,
                membership.clone(),
                self.epoch_height,
                self.view_timeout,
            )
            .await;

            tokio::spawn(run_node(coord, output_tx));
        }

        // Create a client network for broadcasting transactions.
        let client_net = network_state.create_client().await;
        let mut client_network =
            Network::<TestTypes, _>::new(client_net, membership.clone(), upgrade_lock());

        // Collect decided leaf commits from each node until all reach the target.
        let mut node_commits: Vec<BTreeSet<[u8; 32]>> = vec![BTreeSet::new(); self.num_nodes];
        let mut progress: usize = 0;
        let mut tx_nonce: u64 = 0;

        let deadline = tokio::time::Instant::now() + self.max_runtime;
        while node_commits.iter().any(|s| s.len() < self.target_decisions) {
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .ok_or(TestError::Timeout)?;

            // Broadcast a transaction over the network.
            tx_nonce = tx_nonce.wrapping_add(1);
            broadcast_transaction(&mut client_network, tx_nonce, self.transaction_size).await;

            // Collect any newly decided commits.
            for (idx, rx) in output_channels.iter_mut().enumerate() {
                if node_commits[idx].len() >= self.target_decisions {
                    continue;
                }
                if let Ok(Some(seq)) = timeout(remaining, rx.recv()).await {
                    node_commits[idx] = seq;
                }
            }

            let new_progress = node_commits.iter().map(|s| s.len()).min().unwrap_or(0);
            if new_progress > progress {
                progress = new_progress;
                info!("decided_counter={progress}/{}", self.target_decisions);
            }
        }

        // Verify all nodes decided the same set of leaves.
        let expected = &node_commits[0];
        for (i, set) in node_commits.iter().enumerate().skip(1) {
            if expected != set {
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
    output_tx: UnboundedSender<BTreeSet<[u8; 32]>>,
) {
    let mut commits: BTreeSet<[u8; 32]> = BTreeSet::new();
    let mut last_view = ViewNumber::genesis();

    loop {
        match coord.next_consensus_input().await {
            Ok(input) => coord.apply_consensus(input).await,
            Err(err) if err.severity == Severity::Critical => break,
            Err(_) => continue,
        };

        while let Some(output) = coord.outbox_mut().pop_front() {
            if let ConsensusOutput::LeafDecided(leaves) = &output {
                for leaf in leaves {
                    let commit: [u8; 32] = leaf.commit().into();
                    if commits.insert(commit) {
                        info!(
                            node = %coord.node_id(),
                            view = %leaf.view_number(),
                            height = %leaf.height(),
                            "decided leaf"
                        );
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

/// Broadcast a random transaction over the network.
async fn broadcast_transaction<N: ConnectedNetwork<BLSPubKey>>(
    client: &mut Network<TestTypes, N>,
    nonce: u64,
    size: usize,
) {
    let tx = random_transaction(nonce, size);
    let (pk, _) = BLSPubKey::generated_from_seed_indexed([1; 32], 9999);
    let msg: Message<TestTypes, Validated> = Message {
        sender: pk,
        message_type: MessageType::Block(BlockMessage::Transactions(TransactionMessage {
            view: ViewNumber::genesis(),
            transactions: vec![tx],
        })),
    };
    // Best-effort: ignore send errors (network may not be fully ready yet).
    let _ = client.broadcast(msg).await;
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
