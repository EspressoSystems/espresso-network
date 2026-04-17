use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    time::Duration,
};

use committable::Committable;
use hotshot::{traits::NodeImplementation, types::BLSPubKey};
use hotshot_example_types::{block_types::TestTransaction, node_types::TestTypes};
use hotshot_types::{
    data::ViewNumber,
    traits::{network::ConnectedNetwork, signature_key::SignatureKey},
    vote::HasViewNumber,
};
use rand::{RngCore, SeedableRng, rngs::StdRng};
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
    time::timeout,
};
use tracing::{debug, info};

use crate::{
    consensus::ConsensusOutput,
    coordinator::{Coordinator, error::Severity},
    helpers::upgrade_lock,
    message::{BlockMessage, Message, MessageType, TransactionMessage, Validated},
    network::Network,
    tests::common::{coordinator_builder::build_test_coordinator, network::TestNetwork},
};

/// Action to apply to a node at a specific view.
#[derive(Clone, Debug)]
pub struct NodeChange {
    /// Node index to modify.
    pub idx: usize,
    /// Action to perform.
    pub action: NodeAction,
}

/// Actions that can be applied to a node during a test.
#[derive(Clone, Debug)]
pub enum NodeAction {
    /// Restart: shut down the node and create a fresh coordinator from
    /// genesis (blank state).
    Restart,
    /// Start: bring a node that was initially offline into the network
    /// with a fresh coordinator from genesis.
    Start,
    // TODO: Fix the shutdown test and add this back.
    // /// Shutdown: take the node offline.
    // Shutdown,
}

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
    /// Views that are expected to timeout.  If a node times out in a view
    /// not in this set the test fails.  If a node decides a leaf for a view
    /// in this set the test also fails.
    pub expected_failed_views: BTreeSet<ViewNumber>,
    /// Node indices that are offline for the entire test.  Down nodes do
    /// not run a coordinator.  Views where a down node is leader are
    /// expected to timeout.
    pub down_nodes: BTreeSet<usize>,
    /// View-triggered node changes.  Each entry is `(view, changes)`.
    /// Changes are applied when any node first decides a leaf at or past
    /// the specified view.
    pub node_changes: Vec<(u64, Vec<NodeChange>)>,
}

impl Default for TestRunner {
    fn default() -> Self {
        Self {
            num_nodes: 5,
            target_decisions: 100,
            max_runtime: Duration::from_secs(60),
            epoch_height: 1000,
            view_timeout: Duration::from_secs(5),
            transaction_size: 64 * 1024,
            expected_failed_views: BTreeSet::new(),
            down_nodes: BTreeSet::new(),
            node_changes: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub enum TestError {
    Timeout,
    ChainDivergence {
        node: usize,
    },
    UnexpectedDecide {
        node: usize,
        view: ViewNumber,
    },
    NotEnoughDecided {
        view: ViewNumber,
        decided_count: usize,
        threshold: usize,
    },
    NotEnoughTimedOut {
        view: ViewNumber,
        timeout_count: usize,
        threshold: usize,
    },
}

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => write!(f, "timed out waiting for all nodes to decide"),
            Self::ChainDivergence { node } => {
                write!(f, "node {node} decided a different chain prefix")
            },
            Self::UnexpectedDecide { node, view } => {
                write!(
                    f,
                    "node {node} decided view {view} which was expected to fail"
                )
            },
            Self::NotEnoughDecided {
                view,
                decided_count,
                threshold,
            } => {
                write!(
                    f,
                    "not enough nodes decided view {view} decided_count={decided_count} \
                     threshold={threshold}"
                )
            },
            Self::NotEnoughTimedOut {
                view,
                timeout_count,
                threshold,
            } => {
                write!(
                    f,
                    "not enough nodes timed out view {view} timeout_count={timeout_count} \
                     threshold={threshold}"
                )
            },
        }
    }
}

enum NodeEvent {
    Decided(BTreeMap<ViewNumber, [u8; 32]>),
    TimedOut(ViewNumber),
}

impl TestRunner {
    /// Compute the set of nodes that should be offline at test start.
    ///
    /// This is the union of permanently-down nodes (`down_nodes`) and any
    /// node whose first action in `node_changes` is `Start` (meaning it
    /// begins offline and is brought up later).
    fn initially_down_nodes(&self) -> BTreeSet<usize> {
        let mut down = self.down_nodes.clone();
        let mut first_action_seen: BTreeSet<usize> = BTreeSet::new();
        let mut sorted_changes: Vec<_> = self.node_changes.iter().collect();
        sorted_changes.sort_by_key(|(v, _)| *v);
        for (_, changes) in sorted_changes {
            for change in changes {
                if first_action_seen.insert(change.idx)
                    && matches!(change.action, NodeAction::Start)
                {
                    down.insert(change.idx);
                }
            }
        }
        down
    }

    /// Compute the views that will fail because their leader is a down node.
    /// Leader for view `v` is node `v % num_nodes`.
    fn failed_views_from_down_nodes(&self) -> BTreeSet<ViewNumber> {
        let mut result = BTreeSet::new();
        let mut successful = 0;
        let mut view = 1usize;
        loop {
            let vn = ViewNumber::new(view as u64);
            let leader = view % self.num_nodes;
            if self.down_nodes.contains(&leader) || self.expected_failed_views.contains(&vn) {
                result.insert(vn);
            } else {
                successful += 1;
                if successful >= self.target_decisions {
                    break;
                }
            }
            view += 1;
        }
        result
    }

    /// Run the integration test using the given network backend.
    ///
    /// Spins up `self.num_nodes` coordinators connected via `N`.  Each
    /// coordinator self-starts via the genesis bootstrap (no side-channel
    /// injection needed).  Transactions are broadcast over the network using
    /// a dedicated client network instance.
    ///
    /// When `node_changes` is non-empty, nodes are dynamically started,
    /// restarted, or shut down at the specified views.  Verification is
    /// adjusted to account for the dynamic topology.
    pub async fn run<N: TestNetwork>(&self) -> Result<(), TestError> {
        let initially_down = self.initially_down_nodes();
        let (network_state, networks) = N::create(self.num_nodes, &initially_down).await;

        let mut node_handles: Vec<Option<JoinHandle<()>>> = Vec::with_capacity(self.num_nodes);
        let mut output_channels: Vec<Option<UnboundedReceiver<NodeEvent>>> =
            Vec::with_capacity(self.num_nodes);
        let mut currently_down = initially_down;

        // Spawn one coordinator task per live node.  Each node gets its
        // own membership instance so they don't share internal state.
        for (i, network) in networks.into_iter().enumerate() {
            let Some(network) = network else {
                node_handles.push(None);
                output_channels.push(None);
                continue;
            };

            let (output_tx, output_rx) = mpsc::unbounded_channel();
            output_channels.push(Some(output_rx));

            let membership = network_state.create_membership(self.num_nodes).await;
            let coord = build_test_coordinator::<N::Impl>(
                i as u64,
                network,
                membership,
                self.epoch_height,
                self.view_timeout,
            )
            .await;

            node_handles.push(Some(tokio::spawn(run_node(coord, output_tx))));
        }

        // Create a client network for broadcasting transactions.
        let client_net = network_state.create_client().await;
        let client_membership = network_state.create_membership(self.num_nodes).await;
        let mut client_network =
            Network::<TestTypes, _>::new(client_net, client_membership, upgrade_lock());

        // Build pending changes sorted by view.
        let mut pending_changes: BTreeMap<u64, Vec<NodeChange>> = BTreeMap::new();
        for (view, changes) in &self.node_changes {
            pending_changes
                .entry(*view)
                .or_default()
                .extend(changes.clone());
        }

        let all_expected_failures = self.failed_views_from_down_nodes();

        // Collect decided leaf commits from each node until all live nodes reach the target.
        let mut node_commits: Vec<BTreeMap<ViewNumber, [u8; 32]>> =
            vec![BTreeMap::new(); self.num_nodes];
        let mut node_timeouts: Vec<BTreeSet<ViewNumber>> = vec![BTreeSet::new(); self.num_nodes];
        let mut max_decided_view: u64 = 0;
        let mut tx_nonce: u64 = 0;

        let deadline = tokio::time::Instant::now() + self.max_runtime;
        while node_commits
            .iter()
            .enumerate()
            .any(|(i, s)| !currently_down.contains(&i) && s.len() < self.target_decisions)
        {
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .ok_or(TestError::Timeout)?;

            // Apply pending node changes when progress reaches their view.
            if !self.node_changes.is_empty() {
                let views_to_apply: Vec<u64> = pending_changes
                    .range(..=max_decided_view)
                    .map(|(&v, _)| v)
                    .collect();
                for view in views_to_apply {
                    let changes = pending_changes.remove(&view).unwrap();
                    for change in &changes {
                        info!(
                            node = change.idx,
                            view,
                            action = ?change.action,
                            "applying node change"
                        );
                        match change.action {
                            NodeAction::Restart | NodeAction::Start => {
                                // Kill existing task if any.
                                if let Some(handle) = node_handles[change.idx].take() {
                                    handle.abort();
                                }
                                // Shut down the old network so its internal
                                // buffers don't block broadcasts.
                                network_state.shutdown_node(change.idx).await;
                                // Create a fresh coordinator from genesis.
                                let net = network_state.create_node(change.idx).await;
                                let (tx, rx) = mpsc::unbounded_channel();
                                let membership =
                                    network_state.create_membership(self.num_nodes).await;
                                let coord = build_test_coordinator::<N::Impl>(
                                    change.idx as u64,
                                    net,
                                    membership,
                                    self.epoch_height,
                                    self.view_timeout,
                                )
                                .await;
                                node_handles[change.idx] = Some(tokio::spawn(run_node(coord, tx)));
                                output_channels[change.idx] = Some(rx);
                                currently_down.remove(&change.idx);
                                node_commits[change.idx] = BTreeMap::new();
                            },
                            // NodeAction::Shutdown => {
                            //     if let Some(handle) = node_handles[change.idx].take() {
                            //         handle.abort();
                            //     }
                            //     network_state.shutdown_node(change.idx).await;
                            //     output_channels[change.idx] = None;
                            //     currently_down.insert(change.idx);
                            // },
                        }
                    }
                }
            }

            // Broadcast a transaction over the network.
            tx_nonce = tx_nonce.wrapping_add(1);
            broadcast_transaction(&mut client_network, tx_nonce, self.transaction_size).await;

            // Collect events from each live node.
            for (idx, rx_opt) in output_channels.iter_mut().enumerate() {
                let Some(rx) = rx_opt else { continue };
                if node_commits[idx].len() >= self.target_decisions {
                    continue;
                }
                if let Ok(Some(event)) = timeout(remaining, rx.recv()).await {
                    match event {
                        NodeEvent::Decided(commits) => {
                            if let Some(&max_v) = commits.keys().last() {
                                let v: u64 = *max_v;
                                if v > max_decided_view {
                                    max_decided_view = v;
                                }
                            }
                            node_commits[idx] = commits;
                        },
                        NodeEvent::TimedOut(view) => {
                            node_timeouts[idx].insert(view);
                        },
                    }
                } else {
                    return Err(TestError::Timeout);
                }
            }
        }

        Self::verify_correctness(
            &node_commits,
            &node_timeouts,
            &all_expected_failures,
            self.target_decisions,
            &currently_down,
        )
    }

    /// Verify that the collected per-node commits and timeouts are consistent:
    ///  - Views expected to fail were not decided by any node, and a quorum timed out.
    ///  - All other views were decided by a quorum, and all nodes that decided
    ///    agree on the same leaf commitment.
    fn verify_correctness(
        node_commits: &[BTreeMap<ViewNumber, [u8; 32]>],
        node_timeouts: &[BTreeSet<ViewNumber>],
        expected_failed_views: &BTreeSet<ViewNumber>,
        target_decisions: usize,
        down_nodes: &BTreeSet<usize>,
    ) -> Result<(), TestError> {
        let num_nodes = node_commits.len();
        let live_nodes = num_nodes - down_nodes.len();
        let threshold = live_nodes * 2 / 3;

        let last_view = expected_failed_views.len() + target_decisions;
        for v in 1..=last_view {
            let view = ViewNumber::new(v.try_into().unwrap());
            if expected_failed_views.contains(&view) {
                let mut timeout_count = 0;
                for (i, commits) in node_commits.iter().enumerate() {
                    if down_nodes.contains(&i) {
                        continue;
                    }
                    if commits.contains_key(&view) {
                        return Err(TestError::UnexpectedDecide { node: i, view });
                    }
                    if node_timeouts[i].contains(&view) {
                        timeout_count += 1;
                    }
                }
                if timeout_count < threshold {
                    return Err(TestError::NotEnoughTimedOut {
                        view,
                        timeout_count,
                        threshold,
                    });
                }
            } else {
                let mut reference = None;
                let mut decided_count = 0;
                for (i, commits) in node_commits.iter().enumerate() {
                    if down_nodes.contains(&i) {
                        continue;
                    }
                    let commit = commits.get(&view);
                    if commit.is_some() {
                        decided_count += 1;
                    }
                    match reference {
                        None => reference = commit,
                        Some(ref ref_commit) => {
                            if commit.is_some_and(|c| &c != ref_commit) {
                                return Err(TestError::ChainDivergence { node: i });
                            }
                        },
                    }
                }
                if decided_count < threshold {
                    return Err(TestError::NotEnoughDecided {
                        view,
                        decided_count,
                        threshold,
                    });
                }
            }
        }

        Ok(())
    }
}

/// Event loop for a single node.  Processes coordinator inputs, collects
/// decided leaf commits, and forwards them to the test runner.
async fn run_node<I: NodeImplementation<TestTypes>>(
    mut coord: Coordinator<TestTypes, I>,
    output_tx: UnboundedSender<NodeEvent>,
) {
    let mut commits: BTreeMap<ViewNumber, [u8; 32]> = BTreeMap::new();
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
                    let view = leaf.view_number();
                    if let std::collections::btree_map::Entry::Vacant(e) = commits.entry(view) {
                        e.insert(commit);
                        info!(
                            node = %coord.node_id(),
                            view = %view,
                            height = %leaf.height(),
                            "decided leaf"
                        );
                    }
                }
                let _ = output_tx.send(NodeEvent::Decided(commits.clone()));
            } else if let ConsensusOutput::SendTimeoutVote(vote, _) = &output {
                let view = vote.view_number();
                debug!(node = %coord.node_id(), %view, "timeout vote");
                let _ = output_tx.send(NodeEvent::TimedOut(view));
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

            if let Err(err) = coord.process_consensus_output(output).await
                && err.severity == Severity::Critical
            {
                tracing::error!(%err, node = %coord.node_id(), "critical error processing output");
                return;
            }
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
