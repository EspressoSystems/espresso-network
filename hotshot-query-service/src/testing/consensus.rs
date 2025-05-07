// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use std::{fmt::Display, num::NonZeroUsize, str::FromStr, sync::Arc, time::Duration};

use alloy::primitives::U256;
use async_lock::RwLock;
use async_trait::async_trait;
use futures::{
    future::{join_all, Future},
    stream::StreamExt,
};
use hotshot::{
    traits::implementations::{MasterMap, MemoryNetwork},
    types::{Event, SystemContextHandle},
    HotShotInitializer, MarketplaceConfig, SystemContext,
};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider, state_types::TestInstanceState,
    storage_types::TestStorage,
};
use hotshot_testing::block_builder::{SimpleBuilderImplementation, TestBuilderImplementation};
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    data::ViewNumber,
    drb::INITIAL_DRB_RESULT,
    epoch_membership::EpochMembershipCoordinator,
    light_client::StateKeyPair,
    signature_key::BLSPubKey,
    traits::{
        election::Membership,
        network::Topic,
        node_implementation::{ConsensusTime, Versions},
        signature_key::SignatureKey as _,
        storage::storage_add_drb_result,
    },
    HotShotConfig, PeerConfig,
};
use tokio::{
    runtime::Handle,
    task::{block_in_place, yield_now},
};
use tracing::{info_span, Instrument};
use url::Url;

use super::mocks::{MockMembership, MockNodeImpl, MockTransaction, MockTypes};
use crate::{
    availability::{AvailabilityDataSource, UpdateAvailabilityData},
    data_source::{FileSystemDataSource, SqlDataSource, VersionedDataSource},
    fetching::provider::NoFetching,
    node::NodeDataSource,
    status::{StatusDataSource, UpdateStatusData},
    task::BackgroundTask,
    SignatureKey,
};

struct MockNode<D: DataSourceLifeCycle, V: Versions> {
    hotshot: SystemContextHandle<MockTypes, MockNodeImpl, V>,
    data_source: D,
    storage: D::Storage,
}

pub struct MockNetwork<D: DataSourceLifeCycle, V: Versions> {
    tasks: Vec<BackgroundTask>,
    nodes: Vec<MockNode<D, V>>,
    pub_keys: Vec<BLSPubKey>,
}

// MockNetwork can be used with any DataSourceLifeCycle, but it's nice to have a default with a
// convenient type alias.
pub type MockDataSource = FileSystemDataSource<MockTypes, NoFetching>;
pub type MockSqlDataSource = SqlDataSource<MockTypes, NoFetching>;

pub const NUM_NODES: usize = 2;
const EPOCH_HEIGHT: u64 = 10;

impl<D: DataSourceLifeCycle + UpdateStatusData, V: Versions> MockNetwork<D, V> {
    pub async fn init() -> Self {
        Self::init_with_config(|_| {}, false).await
    }

    pub async fn init_with_leaf_ds() -> Self {
        Self::init_with_config(|_| {}, true).await
    }

    pub async fn init_with_config(
        update_config: impl FnOnce(&mut HotShotConfig<MockTypes>),
        leaf_only: bool,
    ) -> Self {
        let (pub_keys, priv_keys): (Vec<_>, Vec<_>) = (0..NUM_NODES)
            .map(|i| BLSPubKey::generated_from_seed_indexed([0; 32], i as u64))
            .unzip();
        let num_staked_nodes = NonZeroUsize::new(pub_keys.len()).unwrap();
        let state_key_pairs = (0..num_staked_nodes.into())
            .map(|i| StateKeyPair::generate_from_seed_indexed([0; 32], i as u64))
            .collect::<Vec<_>>();
        let master_map = MasterMap::new();
        let stake = 1u64;
        let known_nodes_with_stake = (0..num_staked_nodes.into())
            .map(|id| PeerConfig {
                stake_table_entry: pub_keys[id].stake_table_entry(U256::from(stake)),
                state_ver_key: state_key_pairs[id].ver_key(),
            })
            .collect::<Vec<_>>();

        let membership = Arc::new(RwLock::new(MockMembership::new(
            known_nodes_with_stake.clone(),
            known_nodes_with_stake.clone(),
        )));

        // Pick a random, unused port for the builder server
        let builder_port = portpicker::pick_unused_port().expect("No ports available");

        // Create the bind URL from the random port
        let builder_url =
            Url::parse(&format!("http://0.0.0.0:{builder_port}")).expect("Failed to parse URL");

        // Start the builder server
        let builder_task =
            <SimpleBuilderImplementation as TestBuilderImplementation<MockTypes>>::start(
                NUM_NODES,
                builder_url.clone(),
                (),
                Default::default(),
            )
            .await;

        let mut config = HotShotConfig {
            builder_urls: vec1::vec1![builder_url.clone()],
            fixed_leader_for_gpuvid: 0,
            num_nodes_with_stake: num_staked_nodes,
            known_nodes_with_stake: known_nodes_with_stake.clone(),
            next_view_timeout: 10000,
            num_bootstrap: 0,
            da_staked_committee_size: pub_keys.len(),
            known_da_nodes: known_nodes_with_stake.clone(),
            data_request_delay: Duration::from_millis(200),
            view_sync_timeout: Duration::from_millis(250),
            start_threshold: (
                known_nodes_with_stake.len() as u64,
                known_nodes_with_stake.len() as u64,
            ),
            builder_timeout: Duration::from_secs(1),
            start_proposing_view: 0,
            stop_proposing_view: 0,
            start_voting_view: 0,
            stop_voting_view: 0,
            start_proposing_time: 0,
            stop_proposing_time: 0,
            start_voting_time: 0,
            stop_voting_time: 0,
            epoch_height: EPOCH_HEIGHT,
            epoch_start_block: 0,
        };
        update_config(&mut config);

        let nodes = join_all(
            priv_keys
                .into_iter()
                .enumerate()
                .map(|(node_id, priv_key)| {
                    let membership = membership.clone();
                    let config = config.clone();

                    let pub_keys = pub_keys.clone();
                    let master_map = master_map.clone();
                    let state_priv_keys = state_key_pairs
                        .iter()
                        .map(|kp| kp.sign_key())
                        .collect::<Vec<_>>();

                    let span = info_span!("initialize node", node_id);
                    async move {
                        let storage = D::create(node_id).await;
                        let data_source = if leaf_only {
                            D::leaf_only_ds(&storage).await
                        } else {
                            D::connect(&storage).await
                        };

                        let network = Arc::new(MemoryNetwork::new(
                            &pub_keys[node_id],
                            &master_map.clone(),
                            &[Topic::Global, Topic::Da],
                            None,
                        ));

                        let hs_storage: TestStorage<MockTypes> = TestStorage::default();
                        membership
                            .write()
                            .await
                            .set_first_epoch(ViewNumber::new(0), INITIAL_DRB_RESULT);
                        let memberships = EpochMembershipCoordinator::new(
                            membership,
                            Some(storage_add_drb_result(hs_storage.clone())),
                            config.epoch_height,
                        );

                        let hotshot = SystemContext::init(
                            pub_keys[node_id],
                            priv_key,
                            state_priv_keys[node_id].clone(),
                            node_id as u64,
                            config,
                            memberships,
                            network,
                            HotShotInitializer::from_genesis::<V>(
                                TestInstanceState::default(),
                                0,
                                0,
                                vec![],
                            )
                            .await
                            .unwrap(),
                            ConsensusMetricsValue::new(&*data_source.populate_metrics()),
                            hs_storage,
                            MarketplaceConfig {
                                auction_results_provider: Arc::new(
                                    TestAuctionResultsProvider::default(),
                                ),
                                fallback_builder_url: Url::from_str("https://some.url").unwrap(),
                            },
                        )
                        .await
                        .unwrap()
                        .0;

                        MockNode {
                            hotshot,
                            data_source,
                            storage,
                        }
                    }
                    .instrument(span)
                }),
        )
        .await;

        // Hook the builder up to the event stream from the first node

        builder_task.start(Box::new(nodes[0].hotshot.event_stream()));

        let mut network = Self {
            nodes,
            pub_keys,
            tasks: Default::default(),
        };
        D::setup(&mut network).await;
        network
    }
}

impl<D: DataSourceLifeCycle, V: Versions> MockNetwork<D, V> {
    pub fn handle(&self) -> &SystemContextHandle<MockTypes, MockNodeImpl, V> {
        &self.nodes[0].hotshot
    }

    pub async fn submit_transaction(&self, tx: MockTransaction) {
        self.handle().submit_transaction(tx).await.unwrap();
    }

    pub fn num_nodes(&self) -> usize {
        self.pub_keys.len()
    }

    pub fn proposer(&self, i: usize) -> SignatureKey<MockTypes> {
        self.pub_keys[i]
    }

    pub fn data_source_index(&self, i: usize) -> D {
        self.nodes[i].data_source.clone()
    }

    pub fn data_source(&self) -> D {
        self.data_source_index(0)
    }

    pub fn storage(&self) -> &D::Storage {
        &self.nodes[0].storage
    }

    pub fn spawn(&mut self, name: impl Display, task: impl Future + Send + 'static) {
        self.tasks.push(BackgroundTask::spawn(name, task));
    }

    pub async fn shut_down(mut self) {
        self.shut_down_impl().await
    }

    async fn shut_down_impl(&mut self) {
        for node in &mut self.nodes {
            node.hotshot.shut_down().await;
        }
    }

    pub fn epoch_height(&self) -> u64 {
        EPOCH_HEIGHT
    }
}

impl<D: DataSourceLifeCycle, V: Versions> MockNetwork<D, V> {
    pub async fn start(&mut self) {
        // Spawn the update tasks.
        for (i, node) in self.nodes.iter_mut().enumerate() {
            let ds = node.data_source.clone();
            let mut events = node.hotshot.event_stream();
            self.tasks.push(BackgroundTask::spawn(
                format!("update node {i}"),
                async move {
                    while let Some(event) = events.next().await {
                        tracing::info!(node = i, event = ?event.event, "EVENT");
                        {
                            ds.handle_event(&event).await;
                        }
                        yield_now().await;
                    }
                },
            ));
        }

        join_all(
            self.nodes
                .iter()
                .map(|node| node.hotshot.hotshot.start_consensus()),
        )
        .await;
    }
}

impl<D: DataSourceLifeCycle, V: Versions> Drop for MockNetwork<D, V> {
    fn drop(&mut self) {
        if let Ok(handle) = Handle::try_current() {
            block_in_place(move || handle.block_on(self.shut_down_impl()));
        }
    }
}

#[async_trait]
pub trait DataSourceLifeCycle: Clone + Send + Sync + Sized + 'static {
    /// Backing storage for the data source.
    ///
    /// This can be used to connect to data sources to the same underlying data. It must be kept
    /// alive as long as the related data sources are open.
    type Storage: Send + Sync;

    async fn create(node_id: usize) -> Self::Storage;
    async fn connect(storage: &Self::Storage) -> Self;
    async fn reset(storage: &Self::Storage) -> Self;
    async fn handle_event(&self, event: &Event<MockTypes>);
    async fn leaf_only_ds(_storage: &Self::Storage) -> Self {
        panic!("not supported")
    }

    /// Setup runs after setting up the network but before starting a test.
    async fn setup<V: Versions>(_network: &mut MockNetwork<Self, V>) {}
}

pub trait TestableDataSource:
    DataSourceLifeCycle
    + AvailabilityDataSource<MockTypes>
    + UpdateAvailabilityData<MockTypes>
    + NodeDataSource<MockTypes>
    + StatusDataSource
    + VersionedDataSource
{
}

impl<T> TestableDataSource for T where
    T: DataSourceLifeCycle
        + AvailabilityDataSource<MockTypes>
        + UpdateAvailabilityData<MockTypes>
        + NodeDataSource<MockTypes>
        + StatusDataSource
        + VersionedDataSource
{
}
