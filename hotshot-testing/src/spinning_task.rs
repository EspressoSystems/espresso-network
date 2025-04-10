// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

use alloy::primitives::U256;
use async_broadcast::broadcast;
use async_lock::RwLock;
use async_trait::async_trait;
use futures::future::join_all;
use hotshot::{
    traits::TestableNodeImplementation, types::EventType, HotShotInitializer, InitializerEpochInfo,
    SystemContext,
};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider,
    block_types::TestBlockHeader,
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
    testable_delay::DelayConfig,
};
use hotshot_types::{
    constants::EVENT_CHANNEL_SIZE,
    data::Leaf2,
    event::Event,
    message::convert_proposal,
    simple_certificate::{
        LightClientStateUpdateCertificate, NextEpochQuorumCertificate2, QuorumCertificate2,
    },
    traits::{
        network::{AsyncGenerator, ConnectedNetwork},
        node_implementation::{ConsensusTime, NodeImplementation, NodeType, Versions},
    },
    utils::genesis_epoch_from_version,
    vote::HasViewNumber,
    ValidatorConfig,
};
use hotshot_utils::anytrace::*;

use crate::{
    test_launcher::Network,
    test_runner::{LateNodeContext, LateNodeContextParameters, LateStartNode, Node, TestRunner},
    test_task::{TestResult, TestTaskState},
};

/// convenience type for state and block
pub type StateAndBlock<S, B> = (Vec<S>, Vec<B>);

/// Spinning task state
pub struct SpinningTask<
    TYPES: NodeType,
    N: ConnectedNetwork<TYPES::SignatureKey>,
    I: TestableNodeImplementation<TYPES>,
    V: Versions,
> {
    /// epoch height
    pub epoch_height: u64,
    /// Epoch start block
    pub epoch_start_block: u64,
    /// Saved epoch information. This must be sorted ascending by epoch.
    pub start_epoch_info: Vec<InitializerEpochInfo<TYPES>>,
    /// handle to the nodes
    pub(crate) handles: Arc<RwLock<Vec<Node<TYPES, I, V>>>>,
    /// late start nodes
    pub(crate) late_start: HashMap<u64, LateStartNode<TYPES, I, V>>,
    /// time based changes
    pub(crate) changes: BTreeMap<TYPES::View, Vec<ChangeNode>>,
    /// most recent view seen by spinning task
    pub(crate) latest_view: Option<TYPES::View>,
    /// Last decided leaf that can be used as the anchor leaf to initialize the node.
    pub(crate) last_decided_leaf: Leaf2<TYPES>,
    /// Highest qc seen in the test for restarting nodes
    pub(crate) high_qc: QuorumCertificate2<TYPES>,
    /// Next epoch highest qc seen in the test for restarting nodes
    pub(crate) next_epoch_high_qc: Option<NextEpochQuorumCertificate2<TYPES>>,
    /// Add specified delay to async calls
    pub(crate) async_delay_config: DelayConfig,
    /// Context stored for nodes to be restarted with
    pub(crate) restart_contexts: HashMap<usize, RestartContext<TYPES, N, I, V>>,
    /// Generate network channel for restart nodes
    pub(crate) channel_generator: AsyncGenerator<Network<TYPES, I>>,
    /// The light client state update certificate
    pub(crate) state_cert: Option<LightClientStateUpdateCertificate<TYPES>>,
}

#[async_trait]
impl<
        TYPES: NodeType<
            InstanceState = TestInstanceState,
            ValidatedState = TestValidatedState,
            BlockHeader = TestBlockHeader,
        >,
        I: TestableNodeImplementation<TYPES>,
        N: ConnectedNetwork<TYPES::SignatureKey>,
        V: Versions,
    > TestTaskState for SpinningTask<TYPES, N, I, V>
where
    I: TestableNodeImplementation<TYPES>,
    I: NodeImplementation<
        TYPES,
        Network = N,
        Storage = TestStorage<TYPES>,
        AuctionResultsProvider = TestAuctionResultsProvider<TYPES>,
    >,
{
    type Event = Event<TYPES>;
    type Error = Error;

    async fn handle_event(&mut self, (message, _id): (Self::Event, usize)) -> Result<()> {
        let Event { view_number, event } = message;

        if let EventType::Decide {
            leaf_chain,
            qc: _,
            block_size: _,
        } = event
        {
            let leaf = leaf_chain.first().unwrap().leaf.clone();
            if leaf.view_number() > self.last_decided_leaf.view_number() {
                self.last_decided_leaf = leaf;
            }
        } else if let EventType::QuorumProposal {
            proposal,
            sender: _,
        } = event
        {
            if proposal.data.justify_qc().view_number() > self.high_qc.view_number() {
                self.high_qc = proposal.data.justify_qc().clone();
            }
        }

        let mut new_nodes = vec![];
        let mut new_networks = vec![];
        // if we have not seen this view before
        if self.latest_view.is_none() || view_number > self.latest_view.unwrap() {
            // perform operations on the nodes
            if let Some(operations) = self.changes.remove(&view_number) {
                for ChangeNode { idx, updown } in operations {
                    match updown {
                        NodeAction::Up => {
                            let node_id = idx.try_into().unwrap();
                            if let Some(node) = self.late_start.remove(&node_id) {
                                tracing::error!("Node {} spinning up late", idx);
                                let network = if let Some(network) = node.network {
                                    network
                                } else {
                                    let generated_network = (self.channel_generator)(node_id).await;
                                    generated_network.wait_for_ready().await;
                                    generated_network
                                };
                                let node_id = idx.try_into().unwrap();
                                let context = match node.context {
                                    LateNodeContext::InitializedContext(context) => context,
                                    // Node not initialized. Initialize it
                                    // based on the received leaf.
                                    LateNodeContext::UninitializedContext(late_context_params) => {
                                        // We'll deconstruct the individual terms here.
                                        let LateNodeContextParameters {
                                            storage,
                                            memberships,
                                            config,
                                            marketplace_config,
                                        } = late_context_params;

                                        let initializer = HotShotInitializer::<TYPES>::load(
                                            TestInstanceState::new(self.async_delay_config.clone()),
                                            self.epoch_height,
                                            self.epoch_start_block,
                                            self.start_epoch_info.clone(),
                                            self.last_decided_leaf.clone(),
                                            (
                                                TYPES::View::genesis(),
                                                genesis_epoch_from_version::<V, TYPES>(),
                                            ),
                                            (self.high_qc.clone(), self.next_epoch_high_qc.clone()),
                                            BTreeMap::new(),
                                            BTreeMap::new(),
                                            None,
                                            self.state_cert.clone(),
                                        );
                                        // We assign node's public key and stake value rather than read from config file since it's a test
                                        let validator_config =
                                            ValidatorConfig::generated_from_seed_indexed(
                                                [0u8; 32],
                                                node_id,
                                                U256::from(1),
                                                // For tests, make the node DA based on its index
                                                node_id < config.da_staked_committee_size as u64,
                                            );

                                        TestRunner::add_node_with_config(
                                            node_id,
                                            network.clone(),
                                            memberships,
                                            initializer,
                                            config,
                                            validator_config,
                                            storage,
                                            marketplace_config,
                                        )
                                        .await
                                    },
                                    LateNodeContext::Restart => {
                                        panic!("Cannot spin up a node with Restart context")
                                    },
                                };

                                let handle = context.run_tasks().await;

                                // Create the node and add it to the state, so we can shut them
                                // down properly later to avoid the overflow error in the overall
                                // safety task.
                                let node = Node {
                                    node_id,
                                    network,
                                    handle,
                                };
                                node.handle.hotshot.start_consensus().await;

                                self.handles.write().await.push(node);
                            }
                        },
                        NodeAction::Down => {
                            if let Some(node) = self.handles.write().await.get_mut(idx) {
                                tracing::error!("Node {} shutting down", idx);
                                node.handle.shut_down().await;
                            }
                        },
                        NodeAction::RestartDown(delay_views) => {
                            let node_id = idx.try_into().unwrap();
                            if let Some(node) = self.handles.write().await.get_mut(idx) {
                                tracing::error!("Node {} shutting down", idx);
                                node.handle.shut_down().await;
                                // For restarted nodes generate the network on correct view
                                let generated_network = (self.channel_generator)(node_id).await;

                                let Some(LateStartNode {
                                    network: _,
                                    context: LateNodeContext::Restart,
                                }) = self.late_start.get(&node_id)
                                else {
                                    panic!("Restarted Nodes must have an uninitialized context");
                                };

                                let storage = node.handle.storage().clone();
                                let memberships = node.handle.membership_coordinator.clone();
                                let config = node.handle.hotshot.config.clone();
                                let marketplace_config =
                                    node.handle.hotshot.marketplace_config.clone();

                                let read_storage = storage.read().await;
                                let next_epoch_high_qc =
                                    read_storage.next_epoch_high_qc_cloned().await;
                                let start_view = read_storage.last_actioned_view().await;
                                let start_epoch = read_storage.last_actioned_epoch().await;
                                let high_qc = read_storage.high_qc_cloned().await.unwrap_or(
                                    QuorumCertificate2::genesis::<V>(
                                        &TestValidatedState::default(),
                                        &TestInstanceState::default(),
                                    )
                                    .await,
                                );
                                let state_cert = read_storage.state_cert_cloned().await;
                                let saved_proposals = read_storage.proposals_cloned().await;
                                let mut vid_shares = BTreeMap::new();
                                for (view, hash_map) in read_storage.vids_cloned().await {
                                    let mut converted_hash_map = HashMap::new();
                                    for (key, proposal) in hash_map {
                                        converted_hash_map
                                            .entry(key)
                                            .or_insert_with(BTreeMap::new)
                                            .insert(
                                                proposal.data.target_epoch,
                                                convert_proposal(proposal),
                                            );
                                    }
                                    vid_shares.insert(view, converted_hash_map);
                                }
                                let decided_upgrade_certificate =
                                    read_storage.decided_upgrade_certificate().await;

                                let initializer = HotShotInitializer::<TYPES>::load(
                                    TestInstanceState::new(self.async_delay_config.clone()),
                                    self.epoch_height,
                                    self.epoch_start_block,
                                    self.start_epoch_info.clone(),
                                    self.last_decided_leaf.clone(),
                                    (start_view, start_epoch),
                                    (high_qc, next_epoch_high_qc),
                                    saved_proposals,
                                    vid_shares,
                                    decided_upgrade_certificate,
                                    state_cert,
                                );
                                // We assign node's public key and stake value rather than read from config file since it's a test
                                let validator_config = ValidatorConfig::generated_from_seed_indexed(
                                    [0u8; 32],
                                    node_id,
                                    U256::from(1),
                                    // For tests, make the node DA based on its index
                                    node_id < config.da_staked_committee_size as u64,
                                );
                                let internal_chan = broadcast(EVENT_CHANNEL_SIZE);
                                let context =
                                    TestRunner::<TYPES, I, V, N>::add_node_with_config_and_channels(
                                        node_id,
                                        generated_network.clone(),
                                        Arc::clone(memberships.membership()),
                                        initializer,
                                        config,
                                        validator_config,
                                        (*read_storage).clone(),
                                        marketplace_config.clone(),
                                        internal_chan,
                                        (
                                            node.handle.external_channel_sender(),
                                            node.handle.event_stream_known_impl().new_receiver(),
                                        ),
                                    )
                                    .await;
                                if delay_views == 0 {
                                    new_nodes.push((context, idx));
                                    new_networks.push(generated_network.clone());
                                } else {
                                    let up_view = view_number + delay_views;
                                    let change = ChangeNode {
                                        idx,
                                        updown: NodeAction::RestartUp,
                                    };
                                    self.changes.entry(up_view).or_default().push(change);
                                    let new_ctx = RestartContext {
                                        context,
                                        network: generated_network.clone(),
                                    };
                                    self.restart_contexts.insert(idx, new_ctx);
                                }
                            }
                        },
                        NodeAction::RestartUp => {
                            if let Some(ctx) = self.restart_contexts.remove(&idx) {
                                new_nodes.push((ctx.context, idx));
                                new_networks.push(ctx.network.clone());
                            }
                        },
                        NodeAction::NetworkUp => {
                            if let Some(handle) = self.handles.write().await.get(idx) {
                                tracing::error!("Node {} networks resuming", idx);
                                handle.network.resume();
                            }
                        },
                        NodeAction::NetworkDown => {
                            if let Some(handle) = self.handles.write().await.get(idx) {
                                tracing::error!("Node {} networks pausing", idx);
                                handle.network.pause();
                            }
                        },
                    }
                }
            }
            let mut ready_futs = vec![];
            while let Some(net) = new_networks.pop() {
                ready_futs.push(async move {
                    net.wait_for_ready().await;
                });
            }
            join_all(ready_futs).await;

            let mut start_futs = vec![];

            while let Some((node, id)) = new_nodes.pop() {
                let handles = self.handles.clone();
                let fut = async move {
                    tracing::info!("Starting node {} back up", id);
                    let handle = node.run_tasks().await;

                    // Create the node and add it to the state, so we can shut them
                    // down properly later to avoid the overflow error in the overall
                    // safety task.
                    let node = Node {
                        node_id: id.try_into().unwrap(),
                        network: node.network.clone(),
                        handle,
                    };
                    node.handle.hotshot.start_consensus().await;

                    handles.write().await[id] = node;
                };
                start_futs.push(fut);
            }
            if !start_futs.is_empty() {
                join_all(start_futs).await;
                tracing::info!("Nodes all started");
            }

            // update our latest view
            self.latest_view = Some(view_number);
        }

        Ok(())
    }

    async fn check(&self) -> TestResult {
        TestResult::Pass
    }
}

#[derive(Clone)]
pub(crate) struct RestartContext<
    TYPES: NodeType,
    N: ConnectedNetwork<TYPES::SignatureKey>,
    I: TestableNodeImplementation<TYPES>,
    V: Versions,
> {
    context: Arc<SystemContext<TYPES, I, V>>,
    network: Arc<N>,
}

/// Spin the node up or down
#[derive(Clone, Debug)]
pub enum NodeAction {
    /// spin the node up
    Up,
    /// spin the node down
    Down,
    /// spin the node's network up
    NetworkUp,
    /// spin the node's network down
    NetworkDown,
    /// Take a node down to be restarted after a number of views
    RestartDown(u64),
    /// Start a node up again after it's been shutdown for restart.  This
    /// should only be created following a `RestartDown`
    RestartUp,
}

/// denotes a change in node state
#[derive(Clone, Debug)]
pub struct ChangeNode {
    /// the index of the node
    pub idx: usize,
    /// spin the node or node's network up or down
    pub updown: NodeAction,
}

/// description of the spinning task
/// (used to build a spinning task)
#[derive(Clone, Debug)]
pub struct SpinningTaskDescription {
    /// the changes in node status, time -> changes
    pub node_changes: Vec<(u64, Vec<ChangeNode>)>,
}
