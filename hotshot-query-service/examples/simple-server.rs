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

//! Simple HotShot query server
//!
//! This example demonstrates the most basic usage of hotshot-query-service. It starts a small
//! consensus network with two nodes and connects a query service to each node. It runs each query
//! server on local host. The program continues until it is manually killed.

use std::{num::NonZeroUsize, str::FromStr, sync::Arc, time::Duration};

use alloy::primitives::U256;
use async_lock::RwLock;
use clap::Parser;
use futures::future::{join_all, try_join_all};
use hotshot::{
    traits::implementations::{MasterMap, MemoryNetwork},
    types::{SignatureKey, SystemContextHandle},
    HotShotInitializer, MarketplaceConfig, SystemContext,
};
use hotshot_example_types::{
    auction_results_provider_types::TestAuctionResultsProvider, state_types::TestInstanceState,
    storage_types::TestStorage,
};
use hotshot_query_service::{
    data_source,
    fetching::provider::NoFetching,
    run_standalone_service,
    status::UpdateStatusData,
    testing::{
        consensus::DataSourceLifeCycle,
        mocks::{MockBase, MockMembership, MockNodeImpl, MockTypes, MockVersions},
    },
    Error,
};
use hotshot_testing::block_builder::{SimpleBuilderImplementation, TestBuilderImplementation};
use hotshot_types::{
    consensus::ConsensusMetricsValue,
    epoch_membership::EpochMembershipCoordinator,
    light_client::StateKeyPair,
    signature_key::BLSPubKey,
    traits::{election::Membership, network::Topic},
    HotShotConfig, PeerConfig,
};
use tracing_subscriber::EnvFilter;
use url::Url;
use vbs::version::StaticVersionType;

const NUM_NODES: usize = 2;

#[derive(Parser)]
struct Options {
    /// Port on which to host the query service for the first consensus node.
    #[clap(long, default_value = "18080")]
    port1: u16,

    /// Port on which to host the query service for the second consensus node.
    #[clap(long, default_value = "28080")]
    port2: u16,
}

#[cfg(not(target_os = "windows"))]
type DataSource = data_source::SqlDataSource<MockTypes, NoFetching>;

// To use SqlDataSource, we need to run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(target_os = "windows")]
type DataSource = data_source::FileSystemDataSource<MockTypes, NoFetching>;

type Db = <DataSource as DataSourceLifeCycle>::Storage;

#[cfg(not(target_os = "windows"))]
async fn init_db() -> Db {
    Db::init().await
}

#[cfg(target_os = "windows")]
async fn init_db() -> Db {
    Db::with_prefix("simple-server-db").unwrap()
}

#[cfg(not(target_os = "windows"))]
async fn init_data_source(#[allow(unused_variables)] db: &Db) -> DataSource {
    let mut cfg = data_source::sql::Config::default();

    #[cfg(not(feature = "embedded-db"))]
    {
        cfg = cfg.host(db.host()).port(db.port());
    }

    #[cfg(feature = "embedded-db")]
    {
        cfg = cfg.db_path(db.path());
    }

    cfg.connect(Default::default()).await.unwrap()
}

#[cfg(target_os = "windows")]
async fn init_data_source(db: &Db) -> DataSource {
    DataSource::create(db.path(), Default::default())
        .await
        .unwrap()
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize logging
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    let opt = Options::parse();

    // Start databases for the query services.
    let dbs = join_all((0..NUM_NODES).map(|_| init_db())).await;

    // Create the data sources for the query services.
    let data_sources = join_all(dbs.iter().map(init_data_source)).await;

    // Start consensus.
    let nodes = init_consensus(&data_sources).await;

    // Use version 0.1, for no particular reason
    let bind_version = MockBase::instance();

    // Start the servers.
    try_join_all(
        data_sources
            .into_iter()
            .zip(nodes)
            .zip([opt.port1, opt.port2])
            .map(|((data_source, node), port)| async move {
                let opt = hotshot_query_service::Options {
                    port,
                    ..Default::default()
                };
                run_standalone_service(opt, data_source, node, bind_version).await
            }),
    )
    .await?;

    Ok(())
}

async fn init_consensus(
    data_sources: &[DataSource],
) -> Vec<SystemContextHandle<MockTypes, MockNodeImpl, MockVersions>> {
    let (pub_keys, priv_keys): (Vec<_>, Vec<_>) = (0..data_sources.len())
        .map(|i| BLSPubKey::generated_from_seed_indexed([0; 32], i as u64))
        .unzip();
    let state_key_pairs = (0..data_sources.len())
        .map(|i| StateKeyPair::generate_from_seed_indexed([0; 32], i as u64))
        .collect::<Vec<_>>();
    let master_map = MasterMap::new();
    let known_nodes_with_stake = pub_keys
        .iter()
        .zip(&state_key_pairs)
        .map(|(pub_key, state_key_pair)| PeerConfig::<MockTypes> {
            stake_table_entry: pub_key.stake_table_entry(U256::from(1)),
            state_ver_key: state_key_pair.ver_key(),
        })
        .collect::<Vec<_>>();

    // Get the number of nodes with stake
    let num_nodes_with_stake = NonZeroUsize::new(pub_keys.len()).unwrap();

    let membership = MockMembership::new(
        known_nodes_with_stake.clone(),
        known_nodes_with_stake.clone(),
    );

    // Pick a random, unused port for the builder server
    let builder_port = portpicker::pick_unused_port().expect("No ports available");

    let builder_url =
        Url::parse(&format!("http://0.0.0.0:{builder_port}")).expect("Failed to parse URL");

    // Start the builder server
    let builder_task =
        <SimpleBuilderImplementation as TestBuilderImplementation<MockTypes>>::start(
            1,
            builder_url.clone(),
            (),
            Default::default(),
        )
        .await;

    // Create the configuration
    let config = HotShotConfig {
        builder_urls: vec1::vec1![builder_url],
        fixed_leader_for_gpuvid: 0,
        num_nodes_with_stake,
        known_nodes_with_stake: known_nodes_with_stake.clone(),
        next_view_timeout: 10000,
        num_bootstrap: 0,
        known_da_nodes: known_nodes_with_stake.clone(),
        da_staked_committee_size: pub_keys.len(),
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
        epoch_height: 0,
        epoch_start_block: 0,
    };

    let nodes = join_all(priv_keys.into_iter().zip(data_sources).enumerate().map(
        |(node_id, (priv_key, data_source))| {
            let pub_keys = pub_keys.clone();
            let config = config.clone();
            let master_map = master_map.clone();
            let state_private_keys = state_key_pairs
                .iter()
                .map(|kp| kp.sign_key())
                .collect::<Vec<_>>();

            let membership = membership.clone();
            async move {
                let network = Arc::new(MemoryNetwork::new(
                    &pub_keys[node_id],
                    &master_map.clone(),
                    &[Topic::Global, Topic::Da],
                    None,
                ));

                let storage: TestStorage<MockTypes> = TestStorage::default();
                let coordinator = EpochMembershipCoordinator::new(
                    Arc::new(RwLock::new(membership)),
                    config.epoch_height,
                );

                SystemContext::init(
                    pub_keys[node_id],
                    priv_key,
                    state_private_keys[node_id].clone(),
                    node_id as u64,
                    config,
                    coordinator,
                    network,
                    HotShotInitializer::from_genesis::<MockVersions>(
                        TestInstanceState::default(),
                        0,
                        0,
                        vec![],
                    )
                    .await
                    .unwrap(),
                    ConsensusMetricsValue::new(&*data_source.populate_metrics()),
                    storage,
                    MarketplaceConfig {
                        auction_results_provider: Arc::new(TestAuctionResultsProvider::default()),
                        fallback_builder_url: Url::from_str("https://some.url").unwrap(),
                    },
                )
                .await
                .unwrap()
                .0
            }
        },
    ))
    .await;

    // Hook the builder up to the event stream from the first node
    let event_stream = nodes[0].event_stream();
    builder_task.start(Box::new(event_stream));

    nodes
}
