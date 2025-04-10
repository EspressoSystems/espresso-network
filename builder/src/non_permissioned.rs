use std::{collections::VecDeque, num::NonZeroUsize, sync::Arc, time::Duration};

use anyhow::Context;
use async_broadcast::broadcast;
use async_lock::RwLock;
use espresso_types::{
    eth_signature_key::EthKeyPair, v0_1::NoStorage, v0_99::ChainConfig, EpochCommittees, FeeAmount,
    NodeState, Payload, SeqTypes, ValidatedState,
};
use hotshot::traits::BlockPayload;
use hotshot_builder_core::{
    builder_state::{BuilderState, MessageType},
    service::{
        run_non_permissioned_standalone_builder_service, GlobalState, ProxyGlobalState,
        ReceivedTransaction,
    },
};
use hotshot_types::{
    data::{fake_commitment, vid_commitment, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::GENESIS_VID_NUM_STORAGE_NODES, metrics::NoMetrics,
        node_implementation::Versions, EncodeBytes,
    },
};
use marketplace_builder_shared::{block::ParentBlockReferences, utils::EventServiceStream};
use sequencer::{catchup::StatePeers, L1Params, SequencerApiVersion};
use tide_disco::Url;
use tokio::spawn;
use vbs::version::StaticVersionType;

use crate::run_builder_api_service;

#[derive(Clone, Debug)]
pub struct BuilderConfig {
    pub global_state: Arc<RwLock<GlobalState<SeqTypes>>>,
    pub hotshot_events_api_url: Url,
    pub hotshot_builder_apis_url: Url,
}

pub fn build_instance_state<V: Versions>(
    chain_config: ChainConfig,
    l1_params: L1Params,
    state_peers: Vec<Url>,
) -> NodeState {
    let l1_client = l1_params
        .options
        .connect(l1_params.urls)
        .expect("failed to create L1 client");

    let peers = Arc::new(StatePeers::<SequencerApiVersion>::from_urls(
        state_peers,
        Default::default(),
        &NoMetrics,
    ));

    let coordinator = EpochMembershipCoordinator::new(
        Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            vec![],
            l1_client.clone(),
            chain_config,
            peers.clone(),
            NoStorage,
        ))),
        100,
    );

    NodeState::new(
        u64::MAX, // dummy node ID, only used for debugging
        chain_config,
        l1_client,
        peers,
        V::Base::version(),
        coordinator,
    )
}

impl BuilderConfig {
    #[allow(clippy::too_many_arguments)]
    pub async fn init<V: Versions>(
        builder_key_pair: EthKeyPair,
        bootstrapped_view: ViewNumber,
        tx_channel_capacity: NonZeroUsize,
        event_channel_capacity: NonZeroUsize,
        node_count: NonZeroUsize,
        instance_state: NodeState,
        validated_state: ValidatedState,
        hotshot_events_api_url: Url,
        hotshot_builder_apis_url: Url,
        max_api_timeout_duration: Duration,
        max_block_size_increment_period: Duration,
        maximize_txns_count_timeout_duration: Duration,
        base_fee: FeeAmount,
        tx_status_cache_size: usize,
    ) -> anyhow::Result<Self> {
        tracing::info!(
            address = %builder_key_pair.fee_account(),
            ?bootstrapped_view,
            %tx_channel_capacity,
            %event_channel_capacity,
            ?max_api_timeout_duration,
            ?instance_state.chain_config.max_block_size,
            ?maximize_txns_count_timeout_duration,
            "initializing builder",
        );

        // tx channel
        let (mut tx_sender, tx_receiver) =
            broadcast::<Arc<ReceivedTransaction<SeqTypes>>>(tx_channel_capacity.get());
        tx_sender.set_overflow(true);

        // da channel
        let (da_sender, da_receiver) =
            broadcast::<MessageType<SeqTypes>>(event_channel_capacity.get());

        // qc channel
        let (qc_sender, qc_receiver) =
            broadcast::<MessageType<SeqTypes>>(event_channel_capacity.get());

        // decide channel
        let (decide_sender, decide_receiver) =
            broadcast::<MessageType<SeqTypes>>(event_channel_capacity.get());

        // builder api request channel
        let (req_sender, req_receiver) =
            broadcast::<MessageType<SeqTypes>>(event_channel_capacity.get());

        let (genesis_payload, genesis_ns_table) =
            Payload::from_transactions([], &validated_state, &instance_state)
                .await
                .expect("genesis payload construction failed");

        let builder_commitment = genesis_payload.builder_commitment(&genesis_ns_table);

        let vid_commitment = {
            let payload_bytes = genesis_payload.encode();
            vid_commitment::<V>(
                &payload_bytes,
                &genesis_ns_table.encode(),
                GENESIS_VID_NUM_STORAGE_NODES,
                V::Base::VERSION,
            )
        };

        // create the global state
        let global_state: GlobalState<SeqTypes> = GlobalState::<SeqTypes>::new(
            req_sender,
            tx_sender.clone(),
            vid_commitment,
            bootstrapped_view,
            bootstrapped_view,
            max_block_size_increment_period,
            instance_state.chain_config.max_block_size.into(),
            node_count.into(),
            tx_status_cache_size,
        );

        let global_state = Arc::new(RwLock::new(global_state));
        let global_state_clone = global_state.clone();

        let builder_state = BuilderState::<SeqTypes, V>::new(
            ParentBlockReferences {
                view_number: bootstrapped_view,
                vid_commitment,
                leaf_commit: fake_commitment(),
                builder_commitment,
                tx_count: 0,
                last_nonempty_view: None,
            },
            decide_receiver,
            da_receiver,
            qc_receiver,
            req_receiver,
            tx_receiver,
            VecDeque::new() /* tx_queue */,
            global_state_clone,
            maximize_txns_count_timeout_duration,
            base_fee
                .as_u64()
                .context("the base fee exceeds the maximum amount that a builder can pay (defined by u64::MAX)")?,
            Arc::new(instance_state),
            Duration::from_secs(60),
            Arc::new(validated_state),
        );

        // spawn the builder event loop
        spawn(async move {
            builder_state.event_loop();
        });

        // create the proxy global state it will server the builder apis
        let proxy_global_state = ProxyGlobalState::new(
            global_state.clone(),
            (builder_key_pair.fee_account(), builder_key_pair),
            max_api_timeout_duration,
        );

        // start the hotshot api service
        run_builder_api_service(hotshot_builder_apis_url.clone(), proxy_global_state);

        // spawn the builder service
        let events_url = hotshot_events_api_url.clone();
        let global_state_clone = global_state.clone();
        tracing::info!("Running permissionless builder against hotshot events API at {events_url}",);

        let event_stream =
            EventServiceStream::<SeqTypes, SequencerApiVersion>::connect(events_url).await?;

        spawn(async move {
            let res = run_non_permissioned_standalone_builder_service::<_, SequencerApiVersion, _>(
                da_sender,
                qc_sender,
                decide_sender,
                event_stream,
                global_state_clone,
            )
            .await;
            tracing::error!(?res, "builder service exited");
            if res.is_err() {
                panic!("Builder should restart.");
            }
        });

        tracing::info!("Builder init finished");
        Ok(Self {
            global_state,
            hotshot_events_api_url,
            hotshot_builder_apis_url,
        })
    }
}

#[cfg(test)]
mod test {
    use alloy::node_bindings::Anvil;
    use espresso_types::MockSequencerVersions;
    use futures::StreamExt;
    use portpicker::pick_unused_port;
    use sequencer::{
        api::{
            options::HotshotEvents,
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
            Options,
        },
        persistence::{self},
        testing::TestConfigBuilder,
    };
    use sequencer_utils::test_utils::setup_test;
    use surf_disco::Client;
    use tempfile::TempDir;

    use super::*;
    use crate::testing::{test_builder_impl, NonPermissionedBuilderTestConfig};

    /// Test the non-permissioned builder core
    /// It creates a memory hotshot network and launches the hotshot event streaming api
    /// Builder subscrived to this api, and server the hotshot client request and the private mempool tx submission
    #[tokio::test(flavor = "multi_thread")]
    async fn test_non_permissioned_builder() {
        setup_test();

        let query_port = pick_unused_port().expect("No ports free");

        let event_port = pick_unused_port().expect("No ports free");
        let event_service_url: Url = format!("http://localhost:{event_port}").parse().unwrap();

        let builder_port = pick_unused_port().expect("No ports free");
        let builder_api_url: Url = format!("http://localhost:{builder_port}").parse().unwrap();

        // Set up and start the network
        let anvil = Anvil::new().spawn();
        let l1 = anvil.endpoint_url();
        let network_config = TestConfigBuilder::default().l1_url(l1).build();

        let tmpdir = TempDir::new().unwrap();

        let config = TestNetworkConfigBuilder::default()
            .api_config(
                Options::with_port(query_port)
                    .submit(Default::default())
                    .query_fs(
                        Default::default(),
                        persistence::fs::Options::new(tmpdir.path().to_owned()),
                    )
                    .hotshot_events(HotshotEvents {
                        events_service_port: event_port,
                    }),
            )
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        let builder_config = NonPermissionedBuilderTestConfig::init_non_permissioned_builder::<
            MockSequencerVersions,
        >(
            event_service_url.clone(),
            builder_api_url.clone(),
            network.cfg.num_nodes(),
        )
        .await;

        let events_service_client = Client::<
            hotshot_events_service::events::Error,
            SequencerApiVersion,
        >::new(event_service_url.clone());
        events_service_client.connect(None).await;

        let subscribed_events = events_service_client
            .socket("hotshot-events/events")
            .subscribe::<hotshot_types::event::Event<SeqTypes>>()
            .await
            .unwrap()
            .map(|item| item.expect("Failed to get event from event service"));

        test_builder_impl(
            builder_api_url,
            subscribed_events,
            builder_config.fee_account,
        )
        .await;
    }
}
