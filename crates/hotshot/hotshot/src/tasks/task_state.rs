// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

use async_trait::async_trait;
use chrono::Utc;
use hotshot_task_impls::{
    builder::BuilderClient, consensus::ConsensusTaskState, da::DaTaskState,
    quorum_proposal::QuorumProposalTaskState, quorum_proposal_recv::QuorumProposalRecvTaskState,
    quorum_vote::QuorumVoteTaskState, request::NetworkRequestState, rewind::RewindTaskState,
    transactions::TransactionTaskState, upgrade::UpgradeTaskState, vid::VidTaskState,
    view_sync::ViewSyncTaskState,
};
use hotshot_types::{
    consensus::OuterConsensus,
    traits::{
        consensus_api::ConsensusApi,
        node_implementation::{ConsensusTime, NodeImplementation, NodeType},
    },
};
use tokio::spawn;

use crate::{types::SystemContextHandle, Versions};

/// Trait for creating task states.
#[async_trait]
pub trait CreateTaskState<TYPES, I, V>
where
    TYPES: NodeType,
    I: NodeImplementation<TYPES>,
    V: Versions,
{
    /// Function to create the task state from a given `SystemContextHandle`.
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self;
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for NetworkRequestState<TYPES, I>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        Self {
            network: Arc::clone(&handle.hotshot.network),
            consensus: OuterConsensus::new(handle.hotshot.consensus()),
            view: handle.cur_view().await,
            delay: handle.hotshot.config.data_request_delay,
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            id: handle.hotshot.id,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            spawned_tasks: BTreeMap::new(),
            epoch_height: handle.epoch_height,
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for UpgradeTaskState<TYPES, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        #[cfg(not(feature = "example-upgrade"))]
        return Self {
            output_event_stream: handle.hotshot.external_event_stream.0.clone(),
            cur_view: handle.cur_view().await,
            cur_epoch: handle.cur_epoch().await,
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            vote_collectors: BTreeMap::default(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            id: handle.hotshot.id,
            start_proposing_view: handle.hotshot.config.start_proposing_view,
            stop_proposing_view: handle.hotshot.config.stop_proposing_view,
            start_voting_view: handle.hotshot.config.start_voting_view,
            stop_voting_view: handle.hotshot.config.stop_voting_view,
            start_proposing_time: handle.hotshot.config.start_proposing_time,
            stop_proposing_time: handle.hotshot.config.stop_proposing_time,
            start_voting_time: handle.hotshot.config.start_voting_time,
            stop_voting_time: handle.hotshot.config.stop_voting_time,
            epoch_start_block: handle.hotshot.config.epoch_start_block,
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            epoch_height: handle.epoch_height,
            consensus: OuterConsensus::new(handle.hotshot.consensus()),
        };

        #[cfg(feature = "example-upgrade")]
        return Self {
            output_event_stream: handle.hotshot.external_event_stream.0.clone(),
            cur_view: handle.cur_view().await,
            cur_epoch: handle.cur_epoch().await,
            membership: Arc::clone(&handle.hotshot.memberships),
            network: Arc::clone(&handle.hotshot.network),
            vote_collector: None.into(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            id: handle.hotshot.id,
            start_proposing_view: 5,
            stop_proposing_view: 10,
            start_voting_view: 0,
            stop_voting_view: 20,
            start_proposing_time: 0,
            stop_proposing_time: u64::MAX,
            start_voting_time: 0,
            stop_voting_time: u64::MAX,
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
        };
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for VidTaskState<TYPES, I, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        Self {
            consensus: OuterConsensus::new(handle.hotshot.consensus()),
            cur_view: handle.cur_view().await,
            cur_epoch: handle.cur_epoch().await,
            network: Arc::clone(&handle.hotshot.network),
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            id: handle.hotshot.id,
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            epoch_height: handle.epoch_height,
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for DaTaskState<TYPES, I, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        Self {
            consensus: OuterConsensus::new(handle.hotshot.consensus()),
            output_event_stream: handle.hotshot.external_event_stream.0.clone(),
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            network: Arc::clone(&handle.hotshot.network),
            cur_view: handle.cur_view().await,
            cur_epoch: handle.cur_epoch().await,
            vote_collectors: BTreeMap::default(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            id: handle.hotshot.id,
            storage: handle.storage.clone(),
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for ViewSyncTaskState<TYPES, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        let cur_view = handle.cur_view().await;

        Self {
            cur_view,
            next_view: cur_view,
            cur_epoch: handle.cur_epoch().await,
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            num_timeouts_tracked: 0,
            replica_task_map: HashMap::default().into(),
            pre_commit_relay_map: HashMap::default().into(),
            commit_relay_map: HashMap::default().into(),
            finalize_relay_map: HashMap::default().into(),
            view_sync_timeout: handle.hotshot.config.view_sync_timeout,
            id: handle.hotshot.id,
            last_garbage_collected_view: TYPES::View::new(0),
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for TransactionTaskState<TYPES, I, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        Self {
            builder_timeout: handle.builder_timeout(),
            output_event_stream: handle.hotshot.external_event_stream.0.clone(),
            consensus: OuterConsensus::new(handle.hotshot.consensus()),
            cur_view: handle.cur_view().await,
            cur_epoch: handle.cur_epoch().await,
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            instance_state: handle.hotshot.instance_state(),
            id: handle.hotshot.id,
            builder_clients: handle
                .hotshot
                .config
                .builder_urls
                .iter()
                .cloned()
                .map(BuilderClient::new)
                .collect(),
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            auction_results_provider: Arc::clone(
                &handle.hotshot.marketplace_config.auction_results_provider,
            ),
            fallback_builder_url: handle
                .hotshot
                .marketplace_config
                .fallback_builder_url
                .clone(),
            epoch_height: handle.epoch_height,
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for QuorumVoteTaskState<TYPES, I, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        let consensus = handle.hotshot.consensus();

        // Clone the consensus metrics
        let consensus_metrics = Arc::clone(&consensus.read().await.metrics);

        Self {
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            state_private_key: handle.state_private_key().clone(),
            consensus: OuterConsensus::new(consensus),
            instance_state: handle.hotshot.instance_state(),
            latest_voted_view: handle.cur_view().await,
            vote_dependencies: BTreeMap::new(),
            network: Arc::clone(&handle.hotshot.network),
            membership: handle.hotshot.membership_coordinator.clone(),
            output_event_stream: handle.hotshot.external_event_stream.0.clone(),
            id: handle.hotshot.id,
            storage: handle.storage.clone(),
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            epoch_height: handle.hotshot.config.epoch_height,
            consensus_metrics,
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for QuorumProposalTaskState<TYPES, I, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        let consensus = handle.hotshot.consensus();

        Self {
            latest_proposed_view: handle.cur_view().await,
            cur_epoch: handle.cur_epoch().await,
            proposal_dependencies: BTreeMap::new(),
            formed_state_cert: BTreeMap::new(),
            formed_quorum_certificates: BTreeMap::new(),
            formed_next_epoch_quorum_certificates: BTreeMap::new(),
            consensus: OuterConsensus::new(consensus),
            instance_state: handle.hotshot.instance_state(),
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            storage: handle.storage.clone(),
            timeout: handle.hotshot.config.next_view_timeout,
            id: handle.hotshot.id,
            formed_upgrade_certificate: None,
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            epoch_height: handle.hotshot.config.epoch_height,
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for QuorumProposalRecvTaskState<TYPES, I, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        let consensus = handle.hotshot.consensus();

        Self {
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            consensus: OuterConsensus::new(consensus),
            cur_view: handle.cur_view().await,
            cur_epoch: handle.cur_epoch().await,
            membership: handle.hotshot.membership_coordinator.clone(),
            timeout: handle.hotshot.config.next_view_timeout,
            output_event_stream: handle.hotshot.external_event_stream.0.clone(),
            storage: handle.storage.clone(),
            spawned_tasks: BTreeMap::new(),
            id: handle.hotshot.id,
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            epoch_height: handle.hotshot.config.epoch_height,
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for ConsensusTaskState<TYPES, I, V>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        let consensus = handle.hotshot.consensus();

        Self {
            public_key: handle.public_key().clone(),
            private_key: handle.private_key().clone(),
            instance_state: handle.hotshot.instance_state(),
            network: Arc::clone(&handle.hotshot.network),
            membership_coordinator: handle.hotshot.membership_coordinator.clone(),
            vote_collectors: BTreeMap::default(),
            epoch_root_vote_collectors: BTreeMap::default(),
            next_epoch_vote_collectors: BTreeMap::default(),
            timeout_vote_collectors: BTreeMap::default(),
            cur_view: handle.cur_view().await,
            cur_view_time: Utc::now().timestamp(),
            cur_epoch: handle.cur_epoch().await,
            output_event_stream: handle.hotshot.external_event_stream.0.clone(),
            timeout_task: spawn(async {}),
            timeout: handle.hotshot.config.next_view_timeout,
            consensus: OuterConsensus::new(consensus),
            storage: handle.storage.clone(),
            id: handle.hotshot.id,
            upgrade_lock: handle.hotshot.upgrade_lock.clone(),
            epoch_height: handle.hotshot.config.epoch_height,
            view_start_time: Instant::now(),
            first_epoch: None,
        }
    }
}

#[async_trait]
impl<TYPES: NodeType, I: NodeImplementation<TYPES>, V: Versions> CreateTaskState<TYPES, I, V>
    for RewindTaskState<TYPES>
{
    async fn create_from(handle: &SystemContextHandle<TYPES, I, V>) -> Self {
        Self {
            events: Vec::new(),
            id: handle.hotshot.id,
        }
    }
}
