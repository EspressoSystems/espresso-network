use std::{arch::global_asm, collections::HashSet, num::NonZeroUsize, sync::Arc, time::Duration};

use anyhow::Context;
use async_broadcast::{
    broadcast, Receiver as BroadcastReceiver, RecvError, Sender as BroadcastSender, TryRecvError,
};
use async_lock::RwLock;
use espresso_types::{
    eth_signature_key::EthKeyPair,
    v0_1::NoStorage,
    v0_3::StakeTableFetcher,
    v0_99::{ChainConfig, RollupRegistration},
    EpochCommittees, FeeAmount, L1Client, MarketplaceVersion, MockSequencerVersions, NamespaceId,
    NodeState, Payload, SeqTypes, SequencerVersions, ValidatedState, V0_1,
};
use futures::FutureExt;
use hotshot::traits::BlockPayload;
use hotshot_builder_api::v0_99::builder::{
    BuildError, Error as BuilderApiError, Options as HotshotBuilderApiOptions,
};
use hotshot_events_service::{
    events::{Error as EventStreamApiError, Options as EventStreamingApiOptions},
    events_source::{EventConsumer, EventsStreamer},
};
use hotshot_types::{
    data::{fake_commitment, Leaf, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        block_contents::{Transaction as _, GENESIS_VID_NUM_STORAGE_NODES},
        metrics::NoMetrics,
        node_implementation::{ConsensusTime, NodeType, Versions},
        EncodeBytes,
    },
    utils::BuilderCommitment,
};
use marketplace_builder_core::{
    hooks::BuilderHooks,
    service::{EventServiceStream, GlobalState, ProxyGlobalState},
};
use marketplace_builder_shared::block::ParentBlockReferences;
use marketplace_solver::SolverError;
use sequencer::{catchup::StatePeers, L1Params, NetworkParams, SequencerApiVersion};
use surf::http::headers::ACCEPT;
use surf_disco::Client;
use tide_disco::{app, method::ReadState, App, Url};
use tokio::{spawn, time::sleep};
use vbs::version::{StaticVersion, StaticVersionType};

use crate::hooks::{
    self, fetch_namespaces_to_skip, BidConfig, EspressoFallbackHooks, EspressoReserveHooks,
};

type DynamicHooks = Box<dyn BuilderHooks<SeqTypes>>;

#[derive(Clone)]
pub struct BuilderConfig {
    pub global_state: Arc<GlobalState<SeqTypes, DynamicHooks>>,
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

    let fetcher = StakeTableFetcher::new(
        peers.clone(),
        Arc::new(async_lock::Mutex::new(NoStorage)),
        l1_client.clone(),
        chain_config,
    );
    let coordinator = EpochMembershipCoordinator::new(
        Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            vec![],
            fetcher,
        ))),
        None,
        100,
    );

    NodeState::new(
        u64::MAX, // dummy node ID, only used for debugging
        chain_config,
        l1_client.clone(),
        peers.clone(),
        V::Base::version(),
        coordinator,
    )
}

impl BuilderConfig {
    async fn start_service(
        global_state: Arc<GlobalState<SeqTypes, DynamicHooks>>,
        events_api_url: Url,
        builder_api_url: Url,
    ) -> anyhow::Result<()> {
        // create the proxy global state it will server the builder apis
        let app = Arc::clone(&global_state)
            .into_app()
            .context("Failed to construct builder API app")?;

        spawn(async move {
            tracing::info!("Starting builder API app at {builder_api_url}");
            let res = app
                .serve(builder_api_url, MarketplaceVersion::instance())
                .boxed() // https://github.com/rust-lang/rust/issues/102211
                .await;
            tracing::error!(?res, "Builder API app exited");
        });

        // spawn the builder service
        tracing::info!("Running builder against hotshot events API at {events_api_url}",);

        let stream =
            EventServiceStream::<SeqTypes, SequencerApiVersion>::connect(events_api_url).await?;

        spawn(async move {
            let res = global_state.start_event_loop(stream).await;
            tracing::error!(?res, "Builder service exited");
            if res.is_err() {
                panic!("Builder should restart.");
            }
        });

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn init(
        is_reserve: bool,
        builder_key_pair: EthKeyPair,
        bootstrapped_view: ViewNumber,
        tx_channel_capacity: NonZeroUsize,
        event_channel_capacity: NonZeroUsize,
        instance_state: NodeState,
        events_api_url: Url,
        builder_api_url: Url,
        api_timeout: Duration,
        maximize_txns_count_timeout_duration: Duration,
        base_fee: FeeAmount,
        bid_config: Option<BidConfig>,
        solver_base_url: Url,
    ) -> anyhow::Result<Self> {
        tracing::info!(
            address = %builder_key_pair.fee_account(),
            ?bootstrapped_view,
            %tx_channel_capacity,
            %event_channel_capacity,
            ?api_timeout,
            ?instance_state.chain_config.max_block_size,
            ?maximize_txns_count_timeout_duration,
            "initializing builder",
        );

        let hooks: DynamicHooks = if is_reserve {
            let bid_config = bid_config.expect("Missing bid config for the reserve builder.");
            Box::new(hooks::EspressoReserveHooks {
                namespaces: bid_config.namespaces.into_iter().collect(),
                solver_base_url,
                builder_api_base_url: builder_api_url.clone(),
                bid_key_pair: builder_key_pair.clone(),
                bid_amount: bid_config.amount,
            })
        } else {
            // Fetch the namespaces upon initialization. It will be fetched every 20 views when
            // handling events.
            let namespaces_to_skip = fetch_namespaces_to_skip(solver_base_url.clone()).await;
            Box::new(hooks::EspressoFallbackHooks {
                solver_base_url,
                namespaces_to_skip: RwLock::new(namespaces_to_skip).into(),
            })
        };

        // create the global state
        let global_state = GlobalState::new(
            marketplace_builder_core::service::BuilderConfig {
                builder_keys: (builder_key_pair.fee_account(), builder_key_pair),
                api_timeout,
                tx_capture_timeout: maximize_txns_count_timeout_duration,
                txn_garbage_collect_duration: Duration::from_secs(60),
                txn_channel_capacity: tx_channel_capacity.get(),
                tx_status_cache_capacity: 81920,
                base_fee: base_fee.as_u64().expect("Base fee too high"),
            },
            hooks,
        );

        Self::start_service(
            Arc::clone(&global_state),
            events_api_url.clone(),
            builder_api_url.clone(),
        )
        .await?;

        tracing::info!("Builder init finished");

        Ok(Self {
            global_state,
            hotshot_events_api_url: events_api_url,
            hotshot_builder_apis_url: builder_api_url,
        })
    }
}

#[cfg(all(test, not(feature = "embedded-db")))]
mod test {
    use std::{
        str::FromStr,
        time::{Duration, Instant},
    };

    use alloy::{node_bindings::Anvil, primitives::U256};
    use anyhow::Error;
    use async_lock::RwLock;
    use committable::{Commitment, Committable};
    use espresso_types::{
        mock::MockStateCatchup,
        v0_99::{RollupRegistration, RollupRegistrationBody},
        Event, FeeAccount, Leaf2, MarketplaceVersion, NamespaceId, PubKey, SeqTypes,
        SequencerVersions, Transaction,
    };
    use futures::{Stream, StreamExt};
    use hooks::connect_to_solver;
    use hotshot::{
        helpers::initialize_logging,
        rand,
        types::{
            BLSPrivKey, EventType,
            EventType::{Decide, *},
        },
    };
    use hotshot_builder_api::v0_99::builder::BuildError;
    use hotshot_events_service::{
        events::{Error as EventStreamApiError, Options as EventStreamingApiOptions},
        events_source::{EventConsumer, EventsStreamer},
    };
    use hotshot_query_service::availability::LeafQueryData;
    use hotshot_types::{
        bundle::Bundle,
        data::VidCommitment,
        event::LeafInfo,
        light_client::StateKeyPair,
        signature_key::BLSPubKey,
        traits::{
            block_contents::{BlockPayload, BuilderFee, GENESIS_VID_NUM_STORAGE_NODES},
            node_implementation::{NodeType, Versions},
            signature_key::{BuilderSignatureKey, SignatureKey},
        },
    };
    use marketplace_builder_shared::block::BuilderStateId;
    use marketplace_solver::{testing::MockSolver, SolverError};
    use portpicker::pick_unused_port;
    use sequencer::{
        api::{
            fs::DataSource,
            options::HotshotEvents,
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
            Options,
        },
        persistence,
        persistence::no_storage::{self, NoStorage},
        testing::TestConfigBuilder,
        SequencerApiVersion,
    };
    use sequencer_utils::test_utils::setup_test;
    use surf_disco::{
        socket::{Connection, Unsupported},
        Client,
    };
    use tempfile::TempDir;
    use tide_disco::error::ServerError;
    use tokio::{task::spawn, time::sleep};
    use vbs::version::StaticVersion;

    use super::*;

    const REGISTERED_NAMESPACE: u64 = 10;
    const UNREGISTERED_NAMESPACE: u64 = 20;

    /// Ports for the query and event APIs, respectively.
    struct Ports {
        query: u16,
        event: u16,
    }

    /// URLs for the query, event, and builder APIs, respectively.
    #[derive(Clone)]
    struct Urls {
        query: Url,
        event: Url,
        builder: Url,
    }

    enum Mempool {
        Public,
        Private,
    }

    /// Pick unused ports for URLs, then set up and start the network.
    ///
    /// Returns ports and URLs that will be used later.
    async fn pick_urls_and_start_network() -> (Ports, Urls) {
        let query_port = pick_unused_port().expect("No ports free");
        let query_api_url: Url = format!("http://localhost:{query_port}").parse().unwrap();

        let event_port = pick_unused_port().expect("No ports free");
        let event_service_url: Url = format!("http://localhost:{event_port}").parse().unwrap();

        let builder_port = pick_unused_port().expect("No ports free");
        let builder_api_url: Url = format!("http://localhost:{builder_port}").parse().unwrap();

        (
            Ports {
                query: query_port,
                event: event_port,
            },
            Urls {
                query: query_api_url,
                event: event_service_url,
                builder: builder_api_url,
            },
        )
    }

    /// Initiate a mock solver and register a rollup.
    ///
    /// Returns the solver and its base URL.
    async fn init_mock_solver_and_register_rollup() -> (MockSolver, Url) {
        let mock_solver = MockSolver::init().await;
        let solver_base_url = mock_solver.solver_url.clone();
        let client = connect_to_solver(solver_base_url.clone());

        // Create a list of signature keys for rollup registration data
        let mut signature_keys = Vec::new();
        for _ in 0..10 {
            let private_key =
                <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
            signature_keys.push(BLSPubKey::from_private(&private_key))
        }

        // Add an extra key which will be used to sign the registration.
        let private_key =
            <BLSPubKey as SignatureKey>::PrivateKey::generate(&mut rand::thread_rng());
        let signature_key = BLSPubKey::from_private(&private_key);
        signature_keys.push(signature_key);

        // Initialize a rollup registration with namespace id = `REGISTERED_NAMESPACE`.
        let reg_ns_1_body = RollupRegistrationBody {
            namespace_id: REGISTERED_NAMESPACE.into(),
            reserve_url: Some(Url::from_str("http://localhost").unwrap()),
            reserve_price: 200.into(),
            active: true,
            signature_keys: signature_keys.clone(),
            text: "test".to_string(),
            signature_key,
        };

        // Sign the registration body
        let reg_signature = <SeqTypes as NodeType>::SignatureKey::sign(
            &private_key,
            reg_ns_1_body.commit().as_ref(),
        )
        .expect("failed to sign");

        let reg_ns_1 = RollupRegistration {
            body: reg_ns_1_body,
            signature: reg_signature,
        };

        // registering a rollup
        let _: RollupRegistration = client
            .post("register_rollup")
            .body_json(&reg_ns_1)
            .unwrap()
            .send()
            .await
            .unwrap();
        let result: Vec<RollupRegistration> =
            client.get("rollup_registrations").send().await.unwrap();
        assert_eq!(result, vec![reg_ns_1]);

        (mock_solver, solver_base_url)
    }

    /// Connect to the builder.
    async fn connect_to_builder(urls: Urls) -> Client<ServerError, MarketplaceVersion> {
        // Wait for at least one empty block to be sequenced (after consensus starts VID).
        let sequencer_client: Client<ServerError, SequencerApiVersion> = Client::new(urls.query);
        sequencer_client.connect(None).await;
        sequencer_client
            .socket("availability/stream/leaves/0")
            .subscribe::<LeafQueryData<SeqTypes>>()
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();

        //  Connect to builder
        let builder_client: Client<ServerError, MarketplaceVersion> =
            Client::new(urls.builder.clone());
        builder_client.connect(None).await;

        builder_client
    }

    /// Get the view number and commitment if given a `QuorumProposal` event.
    async fn proposal_view_number_and_commitment(event: Event) -> Option<(u64, VidCommitment)> {
        if let EventType::QuorumProposal { proposal, .. } = event.event {
            let view_number = *proposal.data.view_number();
            let commitment = Leaf2::from_quorum_proposal(&proposal.data).payload_commitment();
            return Some((view_number, commitment));
        }
        None
    }

    /// Wait for a quorum proposal event and get its view number and commitment.
    async fn wait_for_proposal_view_number_and_commitment(
        events: &mut (impl Stream<Item = Event> + Unpin),
    ) -> (u64, VidCommitment) {
        let start = Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Didn't get a quorum proposal in 5 seconds");
            }
            let event = events.next().await.unwrap();
            if let Some((view_number, commitment)) =
                proposal_view_number_and_commitment(event).await
            {
                return (view_number, commitment);
            }
        }
    }

    /// Wait for a transaction event.
    async fn wait_for_transaction(
        events: &mut (impl Stream<Item = Event> + Unpin),
        transaction: Transaction,
    ) {
        let start = Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Didn't get the transaction in 5 seconds");
            }
            let event = events.next().await.unwrap();
            if let EventType::Transactions { transactions: txns } = event.event {
                if txns == vec![transaction.clone()] {
                    return;
                }
            }
        }
    }

    /// Fetch the bundle associated with the provided parent information.
    async fn get_bundle(
        builder_client: Client<ServerError, MarketplaceVersion>,
        parent_view_number: u64,
        parent_commitment: VidCommitment,
    ) -> Bundle<SeqTypes> {
        builder_client
            .get::<Bundle<SeqTypes>>(
                format!(
                    "bundle_info/bundle/{parent_view_number}/{parent_commitment}/{}",
                    parent_view_number + 1
                )
                .as_str(),
            )
            .send()
            .await
            .unwrap()
    }

    /// Submit transactions via the private mempool and fetch the bundle.
    async fn submit_and_get_bundle_with_private_mempool(
        builder_client: Client<ServerError, MarketplaceVersion>,
        transactions: Vec<Transaction>,
        urls: Urls,
    ) -> (Bundle<SeqTypes>, u64) {
        // Subscribe to events.
        let events_service_client = Client::<
            hotshot_events_service::events::Error,
            SequencerApiVersion,
        >::new(urls.event.clone());
        events_service_client.connect(None).await;
        let mut events = events_service_client
            .socket("hotshot-events/events")
            .subscribe::<Event>()
            .await
            .unwrap();

        // Submit transactions via the private mempool.
        let txn_submission_client: Client<ServerError, SequencerApiVersion> =
            Client::new(urls.builder.clone());
        txn_submission_client.connect(None).await;
        txn_submission_client
            .post::<Vec<Commitment<Transaction>>>("txn_submit/batch")
            .body_json(&transactions)
            .unwrap()
            .send()
            .await
            .unwrap();

        // Get the parent view number and commitment.
        let parent_view_number;
        let parent_commitment;
        let start = Instant::now();
        loop {
            if start.elapsed() > Duration::from_secs(5) {
                panic!("Didn't get a quorum proposal in 5 seconds");
            }
            let event = events.next().await.unwrap().unwrap();
            if let Some((view_number, commitment)) =
                proposal_view_number_and_commitment(event).await
            {
                parent_view_number = view_number;
                parent_commitment = commitment;
                break;
            }
        }

        // Fetch the bundle.
        (
            get_bundle(builder_client, parent_view_number, parent_commitment).await,
            parent_view_number,
        )
    }

    async fn test_marketplace_reserve_builder(mempool: Mempool) {
        setup_test();

        let (ports, urls) = pick_urls_and_start_network().await;

        // Run the network.
        let anvil = Anvil::new().spawn();
        let l1 = anvil.endpoint_url();
        let network_config = TestConfigBuilder::default().l1_url(l1).build();
        let tmpdir = TempDir::new().unwrap();
        let config = TestNetworkConfigBuilder::default()
            .api_config(
                Options::with_port(ports.query)
                    .submit(Default::default())
                    .query_fs(
                        Default::default(),
                        persistence::fs::Options::new(tmpdir.path().to_owned()),
                    )
                    .hotshot_events(HotshotEvents {
                        events_service_port: ports.event,
                    }),
            )
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        // Register a rollup using the mock solver.
        // Use `_mock_solver` here to avoid it being dropped.
        let (_mock_solver, solver_base_url) = init_mock_solver_and_register_rollup().await;

        let keypair = FeeAccount::test_key_pair();
        let address = keypair.address();
        let base_fee = FeeAmount::from(10);
        // Start and connect to a reserve builder.
        let init = BuilderConfig::init(
            true,
            keypair.clone(),
            ViewNumber::genesis(),
            NonZeroUsize::new(1024).unwrap(),
            NonZeroUsize::new(1024).unwrap(),
            NodeState::default(),
            urls.event.clone(),
            urls.builder.clone(),
            Duration::from_secs(2),
            Duration::from_secs(2),
            base_fee,
            Some(BidConfig {
                namespaces: vec![NamespaceId::from(REGISTERED_NAMESPACE)],
                amount: FeeAmount::from(10),
            }),
            solver_base_url,
        );
        let _ = init.await.unwrap();
        let builder_client = connect_to_builder(urls.clone()).await;

        // Construct transactions.
        let registered_transaction =
            Transaction::new(REGISTERED_NAMESPACE.into(), vec![1, 1, 1, 1]);
        let unregistered_transaction =
            Transaction::new(UNREGISTERED_NAMESPACE.into(), vec![1, 1, 1, 2]);

        let (bundle, parent_view_number) = match mempool {
            Mempool::Public => {
                let server = &network.server;
                let mut events = server.event_stream().await;

                // Get the parent information before submitting transactions.
                let (parent_view_number, parent_commitment) =
                    wait_for_proposal_view_number_and_commitment(&mut events).await;

                // Submit transactions and wait until they are received.
                server
                    .submit_transaction(registered_transaction.clone())
                    .await
                    .unwrap();
                wait_for_transaction(&mut events, registered_transaction.clone()).await;
                server
                    .submit_transaction(unregistered_transaction.clone())
                    .await
                    .unwrap();
                wait_for_transaction(&mut events, unregistered_transaction).await;

                // Return the retrieved bundle and parent view number
                (
                    get_bundle(builder_client, parent_view_number, parent_commitment).await,
                    parent_view_number,
                )
            },
            Mempool::Private => {
                submit_and_get_bundle_with_private_mempool(
                    builder_client,
                    vec![registered_transaction.clone(), unregistered_transaction],
                    urls,
                )
                .await
            },
        };

        assert_eq!(bundle.transactions, vec![registered_transaction.clone()]);

        let txn_commit = <[u8; 32]>::from(registered_transaction.commit()).to_vec();
        let signature = bundle.signature;
        assert!(signature.recover_address_from_msg(txn_commit).unwrap() == address);

        let fee = base_fee * U256::from(registered_transaction.minimum_block_size());

        let fee_signature = <<SeqTypes  as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::sign_sequencing_fee_marketplace(
            &keypair,
            fee.as_u64().unwrap(),
            parent_view_number + 1,
        )
        .unwrap();

        let sequencing_fee = BuilderFee {
            fee_amount: fee.as_u64().unwrap(),
            fee_account: FeeAccount::from(address),
            fee_signature,
        };

        assert_eq!(bundle.sequencing_fee, sequencing_fee);
    }

    async fn test_marketplace_fallback_builder(mempool: Mempool) {
        setup_test();

        let (ports, urls) = pick_urls_and_start_network().await;

        // Run the network.
        let anvil = Anvil::new().spawn();
        let l1 = anvil.endpoint_url();
        let network_config = TestConfigBuilder::default().l1_url(l1).build();
        let tmpdir = TempDir::new().unwrap();
        let config = TestNetworkConfigBuilder::default()
            .api_config(
                Options::with_port(ports.query)
                    .submit(Default::default())
                    .query_fs(
                        Default::default(),
                        persistence::fs::Options::new(tmpdir.path().to_owned()),
                    )
                    .hotshot_events(HotshotEvents {
                        events_service_port: ports.event,
                    }),
            )
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        // Register a rollup using the mock solver.
        // Use `_mock_solver` here to avoid it being dropped.
        let (_mock_solver, solver_base_url) = init_mock_solver_and_register_rollup().await;

        let keypair = FeeAccount::test_key_pair();
        let address = keypair.address();
        let base_fee = FeeAmount::from(10);

        // Start and connect to a fallback builder.
        let init = BuilderConfig::init(
            false,
            FeeAccount::test_key_pair(),
            ViewNumber::genesis(),
            NonZeroUsize::new(1024).unwrap(),
            NonZeroUsize::new(1024).unwrap(),
            NodeState::default(),
            urls.event.clone(),
            urls.builder.clone(),
            Duration::from_secs(2),
            Duration::from_secs(2),
            base_fee,
            None,
            solver_base_url,
        );
        let _ = init.await.unwrap();
        let builder_client = connect_to_builder(urls.clone()).await;

        // Construct transactions.
        let registered_transaction =
            Transaction::new(REGISTERED_NAMESPACE.into(), vec![1, 1, 1, 1]);
        let unregistered_transaction =
            Transaction::new(UNREGISTERED_NAMESPACE.into(), vec![1, 1, 1, 2]);

        let (bundle, parent_view_number) = match mempool {
            Mempool::Public => {
                let server = &network.server;
                let mut events = server.event_stream().await;

                // Get the parent information before submitting transactions.
                let (parent_view_number, parent_commitment) =
                    wait_for_proposal_view_number_and_commitment(&mut events).await;

                // Submit transactions and wait until they are received.
                server
                    .submit_transaction(registered_transaction.clone())
                    .await
                    .unwrap();
                wait_for_transaction(&mut events, registered_transaction).await;
                server
                    .submit_transaction(unregistered_transaction.clone())
                    .await
                    .unwrap();
                wait_for_transaction(&mut events, unregistered_transaction.clone()).await;

                // Get the bundle.
                (
                    get_bundle(builder_client, parent_view_number, parent_commitment).await,
                    parent_view_number,
                )
            },
            Mempool::Private => {
                submit_and_get_bundle_with_private_mempool(
                    builder_client,
                    vec![registered_transaction, unregistered_transaction.clone()],
                    urls,
                )
                .await
            },
        };

        assert_eq!(bundle.transactions, vec![unregistered_transaction.clone()]);

        let txn_commit = <[u8; 32]>::from(unregistered_transaction.clone().commit()).to_vec();
        let signature = bundle.signature;
        assert!(signature.recover_address_from_msg(txn_commit).unwrap() == address);

        let fee = base_fee * U256::from(unregistered_transaction.minimum_block_size());

        let fee_signature = <<SeqTypes  as NodeType>::BuilderSignatureKey as BuilderSignatureKey>::sign_sequencing_fee_marketplace(
                    &keypair,
                    fee.as_u64().unwrap(),
                    parent_view_number + 1,
                )
                .unwrap();

        let sequencing_fee = BuilderFee {
            fee_amount: fee.as_u64().unwrap(),
            fee_account: FeeAccount::from(address),
            fee_signature,
        };

        assert_eq!(bundle.sequencing_fee, sequencing_fee);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_marketplace_reserve_builder_with_public_mempool() {
        test_marketplace_reserve_builder(Mempool::Public).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_marketplace_reserve_builder_with_private_mempool() {
        test_marketplace_reserve_builder(Mempool::Private).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_marketplace_fallback_builder_with_public_mempool() {
        test_marketplace_fallback_builder(Mempool::Public).await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_marketplace_fallback_builder_with_private_mempool() {
        test_marketplace_fallback_builder(Mempool::Private).await;
    }
}
