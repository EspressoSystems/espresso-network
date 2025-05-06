use espresso_types::SeqTypes;
use hotshot_builder_api::v0_1::builder::{
    Error as BuilderApiError, Options as HotshotBuilderApiOptions,
};
use hotshot_builder_core::service::ProxyGlobalState;
use sequencer::SequencerApiVersion;
use tide_disco::{App, Url};
use tokio::spawn;
use vbs::version::{StaticVersion, StaticVersionType};

pub mod non_permissioned;

// It runs the api service for the builder
pub fn run_builder_api_service(url: Url, source: ProxyGlobalState<SeqTypes>) {
    // it is to serve hotshot
    let builder_api = hotshot_builder_api::v0_1::builder::define_api::<
        ProxyGlobalState<SeqTypes>,
        SeqTypes,
    >(&HotshotBuilderApiOptions::default())
    .expect("Failed to construct the builder APIs");

    // it enables external clients to submit txn to the builder's private mempool
    let private_mempool_api = hotshot_builder_api::v0_1::builder::submit_api::<
        ProxyGlobalState<SeqTypes>,
        SeqTypes,
        StaticVersion<0, 1>,
    >(&HotshotBuilderApiOptions::default())
    .expect("Failed to construct the builder API for private mempool txns");

    let mut app: App<ProxyGlobalState<SeqTypes>, BuilderApiError> = App::with_state(source);

    app.register_module("block_info", builder_api)
        .expect("Failed to register the builder API");

    app.register_module("txn_submit", private_mempool_api)
        .expect("Failed to register the private mempool API");

    spawn(app.serve(url, SequencerApiVersion::instance()));
}

#[cfg(test)]
pub mod testing {
    use std::{
        num::NonZeroUsize,
        sync::Arc,
        time::{Duration, Instant},
    };

    use alloy::{
        node_bindings::{Anvil, AnvilInstance},
        primitives::U256,
    };
    use async_lock::RwLock;
    use committable::Committable;
    use espresso_types::{
        traits::SequencerPersistence, v0_99::ChainConfig, Event, FeeAccount, NamespaceId,
        NodeState, PrivKey, PubKey, Transaction, ValidatedState,
    };
    use futures::stream::{Stream, StreamExt};
    use hotshot::{
        traits::BlockPayload,
        types::{
            BLSPrivKey, BLSPubKey,
            EventType::{self, Decide},
            SignatureKey,
        },
    };
    use hotshot_builder_api::v0_2::block_info::{
        AvailableBlockData, AvailableBlockHeaderInputV1, AvailableBlockInfo,
    };
    use hotshot_events_service::{
        events::{Error as EventStreamApiError, Options as EventStreamingApiOptions},
        events_source::{EventConsumer, EventsStreamer},
    };
    use hotshot_types::{
        data::{Leaf2, ViewNumber},
        event::LeafInfo,
        light_client::StateKeyPair,
        traits::{
            block_contents::BlockHeader,
            node_implementation::{ConsensusTime, NodeType, Versions},
            signature_key::BuilderSignatureKey as _,
        },
        HotShotConfig, PeerConfig, ValidatorConfig,
    };
    use sequencer::{context::Consensus, network, SequencerApiVersion};
    use surf_disco::Client;
    use vbs::version::StaticVersion;

    use super::*;
    use crate::non_permissioned::BuilderConfig;

    #[derive(Clone)]
    pub struct HotShotTestConfig {
        pub config: HotShotConfig<SeqTypes>,
        priv_keys_staking_nodes: Vec<BLSPrivKey>,
        priv_keys_non_staking_nodes: Vec<BLSPrivKey>,
        staking_nodes_state_key_pairs: Vec<StateKeyPair>,
        non_staking_nodes_state_key_pairs: Vec<StateKeyPair>,
        anvil: Arc<AnvilInstance>,
    }

    impl Default for HotShotTestConfig {
        fn default() -> Self {
            let num_nodes_with_stake = Self::NUM_STAKED_NODES;
            let num_nodes_without_stake = Self::NUM_NON_STAKED_NODES;

            // first generate stake table entries for the staking nodes
            let (priv_keys_staking_nodes, staking_nodes_state_key_pairs, known_nodes_with_stake) =
                generate_stake_table_entries(num_nodes_with_stake as u64, 1);
            // Now generate the stake table entries for the non-staking nodes
            let (
                priv_keys_non_staking_nodes,
                non_staking_nodes_state_key_pairs,
                _known_nodes_without_stake,
            ) = generate_stake_table_entries(num_nodes_without_stake as u64, 0);

            let builder_url = hotshot_builder_url();

            let config: HotShotConfig<SeqTypes> = HotShotConfig {
                num_nodes_with_stake: NonZeroUsize::new(num_nodes_with_stake).unwrap(),
                known_da_nodes: known_nodes_with_stake.clone(),
                known_nodes_with_stake: known_nodes_with_stake.clone(),
                next_view_timeout: Duration::from_secs(5).as_millis() as u64,
                num_bootstrap: 1usize,
                da_staked_committee_size: num_nodes_with_stake,
                data_request_delay: Duration::from_millis(200),
                view_sync_timeout: Duration::from_secs(5),
                fixed_leader_for_gpuvid: 0,
                builder_urls: vec1::vec1![builder_url],
                builder_timeout: Duration::from_secs(1),
                start_threshold: (
                    known_nodes_with_stake.clone().len() as u64,
                    known_nodes_with_stake.clone().len() as u64,
                ),
                start_proposing_view: 0,
                stop_proposing_view: 0,
                start_voting_view: 0,
                stop_voting_view: 0,
                start_proposing_time: 0,
                start_voting_time: 0,
                stop_proposing_time: 0,
                stop_voting_time: 0,
                epoch_height: 0,
                epoch_start_block: 0,
            };

            Self {
                config,
                priv_keys_staking_nodes,
                priv_keys_non_staking_nodes,
                staking_nodes_state_key_pairs,
                non_staking_nodes_state_key_pairs,
                anvil: Arc::new(Anvil::new().spawn()),
            }
        }
    }

    pub fn generate_stake_table_entries(
        num_nodes: u64,
        stake_value: u64,
    ) -> (
        Vec<BLSPrivKey>,
        Vec<StateKeyPair>,
        Vec<PeerConfig<SeqTypes>>,
    ) {
        // Generate keys for the nodes.
        let priv_keys = (0..num_nodes)
            .map(|_| PrivKey::generate(&mut rand::thread_rng()))
            .collect::<Vec<_>>();
        let pub_keys = priv_keys
            .iter()
            .map(PubKey::from_private)
            .collect::<Vec<_>>();
        let state_key_pairs = (0..num_nodes)
            .map(|_| StateKeyPair::generate())
            .collect::<Vec<_>>();

        let nodes_with_stake = pub_keys
            .iter()
            .zip(&state_key_pairs)
            .map(|(pub_key, state_key_pair)| PeerConfig::<SeqTypes> {
                stake_table_entry: pub_key.stake_table_entry(U256::from(stake_value)),
                state_ver_key: state_key_pair.ver_key(),
            })
            .collect::<Vec<_>>();

        (priv_keys, state_key_pairs, nodes_with_stake)
    }

    impl HotShotTestConfig {
        pub const NUM_STAKED_NODES: usize = 4;
        pub const NUM_NON_STAKED_NODES: usize = 2;

        pub fn num_staked_nodes(&self) -> usize {
            self.priv_keys_staking_nodes.len()
        }
        pub fn num_non_staked_nodes(&self) -> usize {
            self.priv_keys_non_staking_nodes.len()
        }
        pub fn num_staking_non_staking_nodes(&self) -> usize {
            self.num_staked_nodes() + self.num_non_staked_nodes()
        }
        pub fn total_nodes() -> usize {
            Self::NUM_STAKED_NODES + Self::NUM_NON_STAKED_NODES
        }
        pub fn get_anvil(&self) -> Arc<AnvilInstance> {
            self.anvil.clone()
        }
        pub fn get_validator_config(&self, i: usize, is_staked: bool) -> ValidatorConfig<SeqTypes> {
            if is_staked {
                ValidatorConfig {
                    public_key: self.config.known_nodes_with_stake[i]
                        .stake_table_entry
                        .stake_key,
                    private_key: self.priv_keys_staking_nodes[i].clone(),
                    stake_value: self.config.known_nodes_with_stake[i]
                        .stake_table_entry
                        .stake_amount,
                    state_public_key: self.staking_nodes_state_key_pairs[i].ver_key(),
                    state_private_key: self.staking_nodes_state_key_pairs[i].sign_key(),
                    is_da: true,
                }
            } else {
                ValidatorConfig {
                    public_key: self.config.known_nodes_with_stake[i]
                        .stake_table_entry
                        .stake_key,
                    private_key: self.priv_keys_non_staking_nodes[i].clone(),
                    stake_value: U256::from(0),
                    state_public_key: self.non_staking_nodes_state_key_pairs[i].ver_key(),
                    state_private_key: self.non_staking_nodes_state_key_pairs[i].sign_key(),
                    is_da: true,
                }
            }
        }

        // url for the hotshot event streaming api
        pub fn hotshot_event_streaming_api_url() -> Url {
            // spawn the event streaming api
            let port = portpicker::pick_unused_port()
                .expect("Could not find an open port for hotshot event streaming api");

            let hotshot_events_streaming_api_url =
                Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

            hotshot_events_streaming_api_url
        }

        // start the server for the hotshot event streaming api
        pub fn run_hotshot_event_streaming_api(
            url: Url,
            source: Arc<RwLock<EventsStreamer<SeqTypes>>>,
        ) {
            // Start the web server.
            let hotshot_events_api = hotshot_events_service::events::define_api::<
                Arc<RwLock<EventsStreamer<SeqTypes>>>,
                SeqTypes,
                StaticVersion<0, 1>,
            >(
                &EventStreamingApiOptions::default(),
                "0.0.1".parse().unwrap(),
            )
            .expect("Failed to define hotshot eventsAPI");

            let mut app = App::<_, EventStreamApiError>::with_state(source);

            app.register_module("hotshot-events", hotshot_events_api)
                .expect("Failed to register hotshot events API");

            tokio::spawn(app.serve(url, SequencerApiVersion::instance()));
        }
        // enable hotshot event streaming
        pub fn enable_hotshot_node_event_streaming<P: SequencerPersistence, V: Versions>(
            hotshot_events_api_url: Url,
            known_nodes_with_stake: Vec<PeerConfig<SeqTypes>>,
            num_non_staking_nodes: usize,
            hotshot_context_handle: Arc<Consensus<network::Memory, P, V>>,
        ) {
            // create a event streamer
            let events_streamer = Arc::new(RwLock::new(EventsStreamer::new(
                known_nodes_with_stake,
                num_non_staking_nodes,
            )));

            // serve the hotshot event streaming api with events_streamer state
            Self::run_hotshot_event_streaming_api(hotshot_events_api_url, events_streamer.clone());

            // send the events to the event streaming state
            tokio::spawn({
                async move {
                    let mut hotshot_event_stream = hotshot_context_handle.event_stream();
                    loop {
                        let event = hotshot_event_stream.next().await.unwrap();
                        tracing::debug!("Before writing in event streamer: {event:?}");
                        events_streamer.write().await.handle_event(event).await;
                        tracing::debug!("Event written to the event streamer");
                    }
                }
            });
        }
    }

    // Wait for decide event, make sure it matches submitted transaction. Return the block number
    // containing the transaction.
    pub async fn wait_for_decide_on_handle(
        events: &mut (impl Stream<Item = Event> + Unpin),
        submitted_txn: &Transaction,
    ) -> u64 {
        let commitment = submitted_txn.commit();

        // Keep getting events until we see a Decide event
        loop {
            let event = events.next().await.unwrap();
            tracing::info!("Received event from handle: {event:?}");

            if let Decide { leaf_chain, .. } = event.event {
                if let Some(height) = leaf_chain.iter().find_map(|LeafInfo { leaf, .. }| {
                    if leaf
                        .block_payload()
                        .as_ref()?
                        .transaction_commitments(leaf.block_header().metadata())
                        .contains(&commitment)
                    {
                        Some(leaf.block_header().block_number())
                    } else {
                        None
                    }
                }) {
                    return height;
                }
            } else {
                // Keep waiting
            }
        }
    }

    pub struct NonPermissionedBuilderTestConfig {
        pub config: BuilderConfig,
        pub fee_account: FeeAccount,
    }

    impl NonPermissionedBuilderTestConfig {
        pub const SUBSCRIBED_DA_NODE_ID: usize = 5;

        pub async fn init_non_permissioned_builder<V: Versions>(
            hotshot_events_streaming_api_url: Url,
            hotshot_builder_api_url: Url,
            num_nodes: usize,
        ) -> Self {
            // generate builder keys
            let seed = [201_u8; 32];
            let (fee_account, key_pair) = FeeAccount::generated_from_seed_indexed(seed, 2011_u64);

            // channel capacity for the builder states
            let tx_channel_capacity = NonZeroUsize::new(500).unwrap();
            let event_channel_capacity = NonZeroUsize::new(20).unwrap();
            // bootstrapping view number
            // A new builder can use this view number to start building blocks from this view number
            let bootstrapped_view = ViewNumber::new(0);

            let node_count = NonZeroUsize::new(num_nodes).unwrap();

            let builder_config = BuilderConfig::init::<V>(
                key_pair,
                bootstrapped_view,
                tx_channel_capacity,
                event_channel_capacity,
                node_count,
                NodeState::default().with_current_version(V::Base::VERSION),
                ValidatedState::default(),
                hotshot_events_streaming_api_url,
                hotshot_builder_api_url,
                Duration::from_millis(2000),
                Duration::from_secs(60),
                Duration::from_millis(500),
                ChainConfig::default().base_fee,
                819200,
            )
            .await
            .unwrap();

            Self {
                config: builder_config,
                fee_account,
            }
        }
    }

    pub fn hotshot_builder_url() -> Url {
        // spawn the builder api
        let port =
            portpicker::pick_unused_port().expect("Could not find an open port for builder api");

        let hotshot_builder_api_url =
            Url::parse(format!("http://localhost:{port}").as_str()).unwrap();

        hotshot_builder_api_url
    }

    pub async fn test_builder_impl(
        hotshot_builder_api_url: Url,
        mut subscribed_events: impl Stream<Item = Event> + Unpin,
        builder_pub_key: FeeAccount,
    ) {
        // Start a builder api client
        let builder_client =
            Client::<hotshot_builder_api::v0_1::builder::Error, StaticVersion<0, 1>>::new(
                hotshot_builder_api_url.clone(),
            );
        assert!(builder_client.connect(Some(Duration::from_secs(60))).await);

        // Test submitting transactions
        let txn = Transaction::new(NamespaceId::from(1_u32), vec![1, 2, 3]);
        match builder_client
            .post::<()>("txn_submit/submit")
            .body_json(&txn)
            .unwrap()
            .send()
            .await
        {
            Ok(response) => {
                tracing::info!("Received txn submitted response : {:?}", response);
            },
            Err(e) => {
                panic!("Error submitting private transaction {:?}", e);
            },
        }

        let seed = [207_u8; 32];

        // Hotshot client Public, Private key
        let (hotshot_client_pub_key, hotshot_client_private_key) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed(seed, 2011_u64);

        let start = Instant::now();
        let (available_block_info, view_num) = loop {
            if start.elapsed() > Duration::from_secs(10) {
                panic!("Didn't get a quorum proposal in 10 seconds");
            }

            let event = subscribed_events.next().await.unwrap();
            tracing::warn!("Event: {:?}", event.event);
            if let EventType::QuorumProposal { proposal, .. } = event.event {
                let parent_view_number = *proposal.data.view_number();
                let parent_commitment =
                    Leaf2::from_quorum_proposal(&proposal.data).payload_commitment();
                let encoded_signature = <SeqTypes as NodeType>::SignatureKey::sign(
                    &hotshot_client_private_key,
                    parent_commitment.as_ref(),
                )
                .expect("Claim block signing failed");
                let available_blocks = builder_client
                        .get::<Vec<AvailableBlockInfo<SeqTypes>>>(&format!(
                            "block_info/availableblocks/{parent_commitment}/{parent_view_number}/{hotshot_client_pub_key}/{encoded_signature}"
                        ))
                        .send()
                        .await.expect("Error getting available blocks");
                assert!(!available_blocks.is_empty());
                break (available_blocks, parent_view_number);
            }
        };

        let builder_commitment = available_block_info[0].block_hash.clone();

        // sign the builder_commitment using the client_private_key
        let encoded_signature = <SeqTypes as NodeType>::SignatureKey::sign(
            &hotshot_client_private_key,
            builder_commitment.as_ref(),
        )
        .expect("Claim block signing failed");

        // Test claiming blocks
        let available_block_data = match builder_client
                .get::<AvailableBlockData<SeqTypes>>(&format!(
                    "block_info/claimblock/{builder_commitment}/{view_num}/{hotshot_client_pub_key}/{encoded_signature}"
                ))
                .send()
                .await
            {
                Ok(response) => {
                    tracing::info!("Received Block Data: {:?}", response);
                    response
                }
                Err(e) => {
                    panic!("Error while claiming block {:?}", e);
                }
            };

        assert_eq!(
            available_block_data
                .block_payload
                .transactions(&available_block_data.metadata)
                .collect::<Vec<_>>(),
            vec![txn]
        );

        // Test claiming block header input
        let _available_block_header = match builder_client
                .get::<AvailableBlockHeaderInputV1<SeqTypes>>(&format!(
                    "block_info/claimheaderinput/{builder_commitment}/{view_num}/{hotshot_client_pub_key}/{encoded_signature}"
                ))
                .send()
                .await
            {
                Ok(response) => {
                    tracing::info!("Received Block Header : {:?}", response);
                    response
                }
                Err(e) => {
                    panic!("Error getting claiming block header {:?}", e);
                }
            };

        // test getting builder key
        match builder_client
            .get::<FeeAccount>("block_info/builderaddress")
            .send()
            .await
        {
            Ok(response) => {
                tracing::info!("Received Builder Key : {:?}", response);
                assert_eq!(response, builder_pub_key);
            },
            Err(e) => {
                panic!("Error getting builder key {:?}", e);
            },
        }
    }
}
