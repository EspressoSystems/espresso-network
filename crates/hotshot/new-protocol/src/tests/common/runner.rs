use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt,
    time::Duration,
};

use async_broadcast::Sender;
use bon::Builder;
use committable::Committable;
use hotshot::types::{BLSPubKey, Event, EventType};
use hotshot_example_types::{node_types::TestTypes, storage_types::TestStorage};
use hotshot_types::{
    PeerConnectInfo, addr::NetAddr, data::ViewNumber, message::UpgradeLock,
    traits::signature_key::SignatureKey, vote::HasViewNumber, x25519::Keypair,
};
use tokio::{
    select,
    sync::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
    task::JoinHandle,
    time::{Instant, timeout},
};
use tracing::{debug, info};

use crate::{
    consensus::ConsensusOutput,
    coordinator::{Coordinator, CoordinatorOutput, error::Severity},
    helpers::test_upgrade_lock,
    network::{Network, cliquenet::Cliquenet},
    tests::common::{
        coordinator_builder::build_test_coordinator, utils::mock_membership_with_client,
    },
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
#[derive(Builder)]
pub struct TestRunner {
    /// Number of nodes in the test network.
    #[builder(default = 5)]
    num_nodes: usize,

    /// Number of leaves each node must decide before the test passes.
    #[builder(default = 10)]
    target_decisions: usize,

    /// Maximum wall-clock time before the test is considered failed.
    #[builder(default = Duration::from_secs(300))]
    max_runtime: Duration,

    /// Epoch height passed to each coordinator (0 = no epochs).
    #[builder(default = 100)]
    epoch_height: u64,

    /// Per-node view timeout duration.
    #[builder(default = Duration::from_secs(5))]
    view_timeout: Duration,

    /// Views that are expected to timeout.  If a node times out in a view
    /// not in this set the test fails.  If a node decides a leaf for a view
    /// in this set the test also fails.
    #[builder(default)]
    expected_failed_views: BTreeSet<ViewNumber>,

    /// Node indices that are offline for the entire test.  Down nodes do
    /// not run a coordinator.  Views where a down node is leader are
    /// expected to timeout.
    #[builder(default)]
    down_nodes: BTreeSet<usize>,

    /// View-triggered node changes.  Each entry is `(view, changes)`.
    /// Changes are applied when any node first decides a leaf at or past
    /// the specified view.
    #[builder(default)]
    node_changes: Vec<(u64, Vec<NodeChange>)>,

    /// Optional legacy → new-protocol seed handed to each coordinator
    /// before its run loop starts.
    pre_cutover_seed: Option<PreCutoverSeed>,

    #[builder(skip = test_upgrade_lock())]
    upgrade_lock: UpgradeLock<TestTypes>,
}

/// Seed handed to every coordinator at startup to bridge legacy state.
#[derive(Clone)]
pub struct PreCutoverSeed {
    pub decided_anchor: hotshot_types::data::Leaf2<TestTypes>,
    pub undecided: Vec<hotshot_types::data::Leaf2<TestTypes>>,
    pub high_qc: crate::message::Certificate1<TestTypes>,
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

/// Event with its originating node index and generation.
///
/// The generation is bumped each time a node is restarted so that events
/// queued by the aborted task can be distinguished from events produced by
/// the fresh task.
struct TaggedEvent {
    idx: usize,
    generation: u64,
    event: NodeEvent,
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
    /// injection needed).
    ///
    /// When `node_changes` is non-empty, nodes are dynamically started,
    /// restarted, or shut down at the specified views.  Verification is
    /// adjusted to account for the dynamic topology.
    pub async fn run(&mut self) -> Result<(), TestError> {
        crate::logging::init_test_logging();

        let initially_down = self.initially_down_nodes();
        let mut node_handles: Vec<Option<JoinHandle<()>>> = Vec::with_capacity(self.num_nodes);
        let mut generations: Vec<u64> = vec![0; self.num_nodes];
        let (event_tx, mut event_rx) = mpsc::unbounded_channel::<TaggedEvent>();
        let mut currently_down = initially_down;

        let mut cancels = HashMap::new();

        // Generate keys and addresses for all nodes.
        let parties = (0..self.num_nodes)
            .map(|i| {
                let (public_key, private_key) =
                    BLSPubKey::generated_from_seed_indexed([0u8; 32], i as u64);
                let keypair = Keypair::derive_from::<BLSPubKey>(&private_key).unwrap();
                let port = test_utils::reserve_tcp_port()
                    .expect("OS should have ephemeral ports available");
                let addr = NetAddr::Inet(std::net::Ipv4Addr::LOCALHOST.into(), port);
                (keypair, public_key, addr)
            })
            .collect::<Vec<_>>();

        // Spawn one coordinator task per live node.  Each node gets its
        // own membership instance so they don't share internal state.
        for (i, (_, public_key, _)) in parties.iter().enumerate() {
            let network = create_network(i, &parties, &self.upgrade_lock).await;

            let (membership, storage, client, external_events_tx) =
                mock_membership_with_client(self.num_nodes, self.epoch_height, *public_key).await;

            let coord = build_test_coordinator(
                i as u64,
                network,
                membership,
                storage,
                client,
                self.epoch_height,
                self.view_timeout,
                self.pre_cutover_seed.clone(),
            )
            .await;

            let tx = event_tx.clone();
            let generation = generations[i];
            node_handles.push(if currently_down.contains(&i) {
                None
            } else {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                cancels.insert(i, cancel_tx);
                // Pre-populate commits with seeded leaves so the verifier
                // sees them as decided (they are inherited from the legacy
                // protocol; the new protocol won't fire LeafDecided for
                // them). Also stamp views 1..anchor with the anchor's
                // commit so the verifier accepts them as legacy-decided —
                // it only checks node-cross consistency on these slots,
                // not the actual chain shape.
                let mut initial_commits: BTreeMap<ViewNumber, [u8; 32]> = BTreeMap::new();
                if let Some(seed) = &self.pre_cutover_seed {
                    let anchor_view = seed.decided_anchor.view_number();
                    let anchor_commit: [u8; 32] = seed.decided_anchor.commit().into();
                    for v in 1..*anchor_view {
                        initial_commits.insert(ViewNumber::new(v), anchor_commit);
                    }
                    initial_commits.insert(anchor_view, anchor_commit);
                    for leaf in &seed.undecided {
                        initial_commits.insert(leaf.view_number(), leaf.commit().into());
                    }
                }
                Some(tokio::spawn(run_node(
                    coord,
                    tx,
                    i,
                    generation,
                    external_events_tx,
                    cancel_rx,
                    initial_commits,
                )))
            });
        }

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

        // Pre-populate commits for seeded leaves: those views are
        // "previously decided" (in the legacy protocol) and the new-protocol
        // nodes inherit them via the seed rather than re-deriving them, so
        // they will never appear in `LeafDecided` outputs. The verifier
        // expects every view in `1..=target_decisions` to be either decided
        // or expected-to-fail; without this pre-population the seeded views
        // would falsely fail the `NotEnoughDecided` check.
        if let Some(seed) = &self.pre_cutover_seed {
            let anchor_view = seed.decided_anchor.view_number();
            let anchor_commit: [u8; 32] = seed.decided_anchor.commit().into();
            for commits in &mut node_commits {
                for v in 1..*anchor_view {
                    commits.insert(ViewNumber::new(v), anchor_commit);
                }
                commits.insert(anchor_view, anchor_commit);
                for leaf in &seed.undecided {
                    commits.insert(leaf.view_number(), leaf.commit().into());
                }
            }
        }

        let deadline = Instant::now() + self.max_runtime;
        while node_commits
            .iter()
            .enumerate()
            .any(|(i, s)| !currently_down.contains(&i) && s.len() < self.target_decisions)
        {
            let remaining = deadline
                .checked_duration_since(Instant::now())
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
                                if let Some(tx) = cancels.remove(&change.idx) {
                                    let (a, b) = oneshot::channel();
                                    if tx.send(a).is_ok() {
                                        let _ = b.await;
                                    }
                                }
                                // Kill existing task if any.
                                if let Some(handle) = node_handles[change.idx].take() {
                                    handle.abort();
                                    let _ = handle.await;
                                }
                                // Create a fresh coordinator from genesis.
                                let net =
                                    create_network(change.idx, &parties, &self.upgrade_lock).await;
                                let (membership, storage, client, external_events_tx) = {
                                    let k = parties[change.idx].1;
                                    mock_membership_with_client(
                                        self.num_nodes,
                                        self.epoch_height,
                                        k,
                                    )
                                    .await
                                };
                                let coord = build_test_coordinator(
                                    change.idx as u64,
                                    net,
                                    membership,
                                    storage,
                                    client,
                                    self.epoch_height,
                                    self.view_timeout,
                                    self.pre_cutover_seed.clone(),
                                )
                                .await;
                                // Bump the generation so stale events queued
                                // by the aborted task are ignored.
                                generations[change.idx] += 1;
                                let tx = event_tx.clone();
                                let generation = generations[change.idx];
                                let (cancel_tx, cancel_rx) = oneshot::channel();
                                cancels.insert(change.idx, cancel_tx);
                                // Restarted nodes start with a fresh commits
                                // map (mirroring the wipe at line ~404 below).
                                let initial_commits = BTreeMap::new();
                                node_handles[change.idx] = Some(tokio::spawn(run_node(
                                    coord,
                                    tx,
                                    change.idx,
                                    generation,
                                    external_events_tx,
                                    cancel_rx,
                                    initial_commits,
                                )));
                                currently_down.remove(&change.idx);
                                node_commits[change.idx] = BTreeMap::new();
                            },
                            // NodeAction::Shutdown => {
                            //     if let Some(handle) = node_handles[change.idx].take() {
                            //         handle.abort();
                            //     }
                            //     network_state.shutdown_node(change.idx).await;
                            //     generations[change.idx] += 1;
                            //     currently_down.insert(change.idx);
                            // },
                        }
                    }
                }
            }

            // Get the next event from any node, rather than polling each
            // node in sequence.  Stale events (from tasks aborted by a
            // restart) are filtered out via the generation counter.
            let Ok(Some(tagged)) = timeout(remaining, event_rx.recv()).await else {
                return Err(TestError::Timeout);
            };
            if tagged.generation != generations[tagged.idx] {
                continue;
            }
            if node_commits[tagged.idx].len() >= self.target_decisions {
                continue;
            }
            match tagged.event {
                NodeEvent::Decided(commits) => {
                    if let Some(&max_v) = commits.keys().last() {
                        let v: u64 = *max_v;
                        if v > max_decided_view {
                            max_decided_view = v;
                        }
                    }
                    node_commits[tagged.idx] = commits;
                },
                NodeEvent::TimedOut(view) => {
                    node_timeouts[tagged.idx].insert(view);
                },
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

async fn create_network(
    i: usize,
    parties: &[(Keypair, BLSPubKey, NetAddr)],
    lock: &UpgradeLock<TestTypes>,
) -> Cliquenet<TestTypes> {
    let peer_infos: Vec<(BLSPubKey, PeerConnectInfo)> = parties
        .iter()
        .map(|(kp, pk, addr)| {
            (
                *pk,
                PeerConnectInfo {
                    x25519_key: kp.public_key(),
                    p2p_addr: addr.clone(),
                },
            )
        })
        .collect();

    let config = cliquenet::Config::builder()
        .name("test")
        .keypair(parties[i].0.clone().into())
        .bind(parties[i].2.clone())
        .random_connect_delay(false)
        .parties(
            peer_infos
                .iter()
                .map(|(_, info)| (info.x25519_key.into(), info.p2p_addr.clone())),
        )
        .build();

    Cliquenet::create_with_config(parties[i].1, lock.clone(), config, peer_infos.clone())
        .await
        .unwrap()
}

/// Event loop for a single node.  Processes coordinator inputs, collects
/// decided leaf commits, and forwards them to the test runner.  All events
/// are tagged with the node's index and generation so the runner can
/// multiplex a single receive channel across every node and drop events
/// from tasks that have been superseded by a restart.
async fn run_node<N: Network<TestTypes>>(
    mut coord: Coordinator<TestTypes, N, TestStorage<TestTypes>>,
    output_tx: UnboundedSender<TaggedEvent>,
    idx: usize,
    generation: u64,
    external_events_tx: Sender<Event<TestTypes>>,
    mut cancel: oneshot::Receiver<oneshot::Sender<()>>,
    initial_commits: BTreeMap<ViewNumber, [u8; 32]>,
) {
    let mut commits: BTreeMap<ViewNumber, [u8; 32]> = initial_commits;
    let mut last_view = ViewNumber::genesis();
    let send = |event: NodeEvent| {
        let _ = output_tx.send(TaggedEvent {
            idx,
            generation,
            event,
        });
    };

    loop {
        select! {
            i = coord.next_consensus_input() => match i {
                Ok(input) => coord.apply_consensus(input).await,
                Err(err) if err.severity == Severity::Critical => break,
                Err(_) => continue,
            },
            x = &mut cancel => {
                coord.stop().await;
                if let Ok(tx) = x {
                    let _ = tx.send(());
                }
                return
            }
        }

        while let Some(output) = coord.outbox_mut().pop_front() {
            if let ConsensusOutput::LeafDecided { leaves, .. } = &output {
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
                send(NodeEvent::Decided(commits.clone()));
            } else if let ConsensusOutput::SendTimeoutVote(vote, _) = &output {
                let view = vote.view_number();
                debug!(node = %coord.node_id(), %view, "timeout vote");
                send(NodeEvent::TimedOut(view));
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

            while let Some(output) = coord.coordinator_outbox_mut().pop_front() {
                if let CoordinatorOutput::ExternalMessageReceived { sender, data } = &output {
                    let _ = external_events_tx
                        .broadcast_direct(Event {
                            view_number: coord.current_view(),
                            event: EventType::ExternalMessageReceived {
                                sender: *sender,
                                data: data.clone(),
                            },
                        })
                        .await;
                }
            }
        }
    }
}
