use self::data_source::StateSignatureDataSource;
use crate::{
    network, persistence::SequencerPersistence, state::ValidatedState,
    state_signature::StateSigner, Node, NodeState, SeqTypes, SequencerContext, Transaction,
};
use async_once_cell::Lazy;
use async_std::sync::{Arc, RwLock};
use async_trait::async_trait;
use data_source::{StateDataSource, SubmitDataSource};
use derivative::Derivative;
use futures::{
    future::{BoxFuture, Future, FutureExt},
    stream::{BoxStream, Stream},
};
use hotshot::types::{Event, SystemContextHandle};
use hotshot_events_service::events_source::{BuilderEvent, EventsSource, EventsStreamer};
use hotshot_query_service::data_source::ExtensibleDataSource;
use hotshot_types::{data::ViewNumber, light_client::StateSignatureRequestBody};
use std::pin::Pin;
use vbs::version::StaticVersionType;

pub mod data_source;
pub mod endpoints;
pub mod fs;
pub mod options;
pub mod sql;
mod update;

pub use options::Options;

type BoxLazy<T> = Pin<Arc<Lazy<T, BoxFuture<'static, T>>>>;

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct ConsensusState<N: network::Type, P: SequencerPersistence, Ver: StaticVersionType> {
    state_signer: Arc<StateSigner<Ver>>,
    event_streamer: Arc<RwLock<EventsStreamer<SeqTypes>>>,
    node_state: NodeState,

    #[derivative(Debug = "ignore")]
    handle: SystemContextHandle<SeqTypes, Node<N, P>>,
}

impl<N: network::Type, P: SequencerPersistence, Ver: StaticVersionType + 'static>
    From<&SequencerContext<N, P, Ver>> for ConsensusState<N, P, Ver>
{
    fn from(ctx: &SequencerContext<N, P, Ver>) -> Self {
        Self {
            state_signer: ctx.state_signer(),
            event_streamer: ctx.get_event_streamer(),
            node_state: ctx.node_state(),
            handle: ctx.consensus().clone(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
struct ApiState<N: network::Type, P: SequencerPersistence, Ver: StaticVersionType> {
    // The consensus state is initialized lazily so we can start the API (and healthcheck endpoints)
    // before consensus has started. Any endpoint that uses consensus state will wait for
    // initialization to finish, but endpoints that do not require a consensus handle can proceed
    // without waiting.
    #[derivative(Debug = "ignore")]
    consensus: BoxLazy<ConsensusState<N, P, Ver>>,
}

impl<N: network::Type, P: SequencerPersistence, Ver: StaticVersionType + 'static>
    ApiState<N, P, Ver>
{
    fn new(init: impl Future<Output = ConsensusState<N, P, Ver>> + Send + 'static) -> Self {
        Self {
            consensus: Arc::pin(Lazy::from_future(init.boxed())),
        }
    }

    fn event_stream(&self) -> impl Stream<Item = Event<SeqTypes>> + Unpin {
        let state = self.clone();
        async move { state.consensus().await.get_event_stream() }
            .boxed()
            .flatten_stream()
    }

    async fn state_signer(&self) -> &StateSigner<Ver> {
        &self.consensus.as_ref().get().await.get_ref().state_signer
    }

    async fn event_streamer(&self) -> &RwLock<EventsStreamer<SeqTypes>> {
        &self.consensus.as_ref().get().await.get_ref().event_streamer
    }

    async fn consensus(&self) -> &SystemContextHandle<SeqTypes, Node<N, P>> {
        &self.consensus.as_ref().get().await.get_ref().handle
    }

    async fn node_state(&self) -> &NodeState {
        &self.consensus.as_ref().get().await.get_ref().node_state
    }
}

type StorageState<N, P, D, Ver> = ExtensibleDataSource<D, ApiState<N, P, Ver>>;

#[async_trait]
impl<N: network::Type, Ver: StaticVersionType + 'static, P: SequencerPersistence>
    EventsSource<SeqTypes> for ApiState<N, P, Ver>
{
    type EventStream = BoxStream<'static, Arc<BuilderEvent<SeqTypes>>>;

    async fn get_event_stream(&self) -> Self::EventStream {
        self.event_streamer()
            .await
            .read()
            .await
            .get_event_stream()
            .await
    }
}

impl<
        N: network::Type,
        D: Send + Sync,
        Ver: StaticVersionType + 'static,
        P: SequencerPersistence,
    > SubmitDataSource<N, P> for StorageState<N, P, D, Ver>
{
    async fn submit(&self, tx: Transaction) -> anyhow::Result<()> {
        self.as_ref().submit(tx).await
    }
}

impl<N: network::Type, Ver: StaticVersionType + 'static, P: SequencerPersistence>
    SubmitDataSource<N, P> for ApiState<N, P, Ver>
{
    async fn submit(&self, tx: Transaction) -> anyhow::Result<()> {
        self.consensus().await.submit_transaction(tx).await?;
        Ok(())
    }
}

impl<
        N: network::Type,
        D: Send + Sync,
        Ver: StaticVersionType + 'static,
        P: SequencerPersistence,
    > StateDataSource for StorageState<N, P, D, Ver>
{
    async fn get_decided_state(&self) -> Arc<ValidatedState> {
        self.as_ref().get_decided_state().await
    }

    async fn get_undecided_state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>> {
        self.as_ref().get_undecided_state(view).await
    }
}

impl<N: network::Type, Ver: StaticVersionType + 'static, P: SequencerPersistence> StateDataSource
    for ApiState<N, P, Ver>
{
    async fn get_decided_state(&self) -> Arc<ValidatedState> {
        self.consensus().await.get_decided_state().await
    }

    async fn get_undecided_state(&self, view: ViewNumber) -> Option<Arc<ValidatedState>> {
        self.consensus().await.get_state(view).await
    }
}

#[async_trait]
impl<N: network::Type, D: Sync, Ver: StaticVersionType + 'static, P: SequencerPersistence>
    StateSignatureDataSource<N> for StorageState<N, P, D, Ver>
{
    async fn get_state_signature(&self, height: u64) -> Option<StateSignatureRequestBody> {
        self.as_ref().get_state_signature(height).await
    }
}

#[async_trait]
impl<N: network::Type, Ver: StaticVersionType + 'static, P: SequencerPersistence>
    StateSignatureDataSource<N> for ApiState<N, P, Ver>
{
    async fn get_state_signature(&self, height: u64) -> Option<StateSignatureRequestBody> {
        self.state_signer().await.get_state_signature(height).await
    }
}

#[cfg(test)]
mod test_helpers {
    use super::*;
    use crate::{
        api::endpoints::{AccountQueryData, BlocksFrontier},
        catchup::{mock::MockStateCatchup, StateCatchup},
        persistence::{no_storage::NoStorage, SequencerPersistence},
        state::BlockMerkleTree,
        testing::{run_test_builder, wait_for_decide_on_handle, TestConfig},
    };
    use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
    use async_std::task::sleep;
    use committable::Committable;
    use es_version::{SequencerVersion, SEQUENCER_VERSION};
    use ethers::prelude::Address;
    use futures::{
        future::{join_all, FutureExt},
        stream::StreamExt,
    };
    use hotshot::types::{Event, EventType};

    use hotshot_types::{
        event::LeafInfo,
        traits::{metrics::NoMetrics, node_implementation::ConsensusTime},
    };
    use itertools::izip;
    use jf_primitives::merkle_tree::{MerkleCommitment, MerkleTreeScheme};
    use portpicker::pick_unused_port;
    use std::time::Duration;
    use surf_disco::Client;
    use tide_disco::error::ServerError;

    pub const STAKE_TABLE_CAPACITY_FOR_TEST: usize = 10;

    pub struct TestNetwork<P: SequencerPersistence> {
        pub server: SequencerContext<network::Memory, P, SequencerVersion>,
        pub peers: Vec<SequencerContext<network::Memory, P, SequencerVersion>>,
        pub cfg: TestConfig,
    }

    impl<P: SequencerPersistence> TestNetwork<P> {
        pub async fn with_state(
            opt: Options,
            state: [ValidatedState; TestConfig::NUM_NODES],
            persistence: [P; TestConfig::NUM_NODES],
            catchup: [impl StateCatchup + 'static; TestConfig::NUM_NODES],
        ) -> Self {
            let mut cfg = TestConfig::default();

            let (builder_task, builder_url) = run_test_builder().await;

            cfg.set_builder_url(builder_url);

            let mut nodes = join_all(izip!(state, persistence, catchup).enumerate().map(
                |(i, (state, persistence, catchup))| {
                    let opt = opt.clone();
                    let cfg = &cfg;
                    async move {
                        if i == 0 {
                            opt.serve(
                                |metrics| {
                                    let cfg = cfg.clone();
                                    async move {
                                        cfg.init_node(
                                            0,
                                            state,
                                            persistence,
                                            catchup,
                                            &*metrics,
                                            STAKE_TABLE_CAPACITY_FOR_TEST,
                                            SEQUENCER_VERSION,
                                        )
                                        .await
                                    }
                                    .boxed()
                                },
                                SEQUENCER_VERSION,
                            )
                            .await
                            .unwrap()
                        } else {
                            cfg.init_node(
                                i,
                                state,
                                persistence,
                                catchup,
                                &NoMetrics,
                                STAKE_TABLE_CAPACITY_FOR_TEST,
                                SEQUENCER_VERSION,
                            )
                            .await
                        }
                    }
                },
            ))
            .await;

            let handle_0 = &nodes[0];

            // Hook the builder up to the event stream from the first node
            if let Some(builder_task) = builder_task {
                builder_task.start(Box::new(handle_0.get_event_stream()));
            }

            for ctx in &nodes {
                ctx.start_consensus().await;
            }

            let server = nodes.remove(0);
            let peers = nodes;

            Self { server, peers, cfg }
        }

        pub async fn new(opt: Options, persistence: [P; TestConfig::NUM_NODES]) -> Self {
            Self::with_state(
                opt,
                Default::default(),
                persistence,
                std::array::from_fn(|_| MockStateCatchup::default()),
            )
            .await
        }

        pub async fn stop_consensus(&mut self) {
            self.server.consensus_mut().shut_down().await;
            for ctx in &mut self.peers {
                ctx.consensus_mut().shut_down().await;
            }
        }
    }

    /// Test the status API with custom options.
    ///
    /// The `opt` function can be used to modify the [`Options`] which are used to start the server.
    /// By default, the options are the minimal required to run this test (configuring a port and
    /// enabling the status API). `opt` may add additional functionality (e.g. adding a query module
    /// to test a different initialization path) but should not remove or modify the existing
    /// functionality (e.g. removing the status module or changing the port).
    pub async fn status_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_logging();
        setup_backtrace();

        let port = pick_unused_port().expect("No ports free");
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerVersion> = Client::new(url);

        let options = opt(Options::from(options::Http { port }).status(Default::default()));
        let _network = TestNetwork::new(options, [NoStorage; TestConfig::NUM_NODES]).await;
        client.connect(None).await;

        // The status API is well tested in the query service repo. Here we are just smoke testing
        // that we set it up correctly. Wait for a (non-genesis) block to be sequenced and then
        // check the success rate metrics.
        while client
            .get::<u64>("status/block-height")
            .send()
            .await
            .unwrap()
            <= 1
        {
            sleep(Duration::from_secs(1)).await;
        }
        let success_rate = client
            .get::<f64>("status/success-rate")
            .send()
            .await
            .unwrap();
        // If metrics are populating correctly, we should get a finite number. If not, we might get
        // NaN or infinity due to division by 0.
        assert!(success_rate.is_finite(), "{success_rate}");
        // We know at least some views have been successful, since we finalized a block.
        assert!(success_rate > 0.0, "{success_rate}");
    }

    /// Test the submit API with custom options.
    ///
    /// The `opt` function can be used to modify the [`Options`] which are used to start the server.
    /// By default, the options are the minimal required to run this test (configuring a port and
    /// enabling the submit API). `opt` may add additional functionality (e.g. adding a query module
    /// to test a different initialization path) but should not remove or modify the existing
    /// functionality (e.g. removing the submit module or changing the port).
    pub async fn submit_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_logging();
        setup_backtrace();

        let txn = Transaction::new(Default::default(), vec![1, 2, 3, 4]);

        let port = pick_unused_port().expect("No ports free");

        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerVersion> = Client::new(url);

        let options = opt(Options::from(options::Http { port }).submit(Default::default()));
        let network = TestNetwork::new(options, [NoStorage; TestConfig::NUM_NODES]).await;
        let mut events = network.server.get_event_stream();

        client.connect(None).await;

        let hash = client
            .post("submit/submit")
            .body_json(&txn)
            .unwrap()
            .send()
            .await
            .unwrap();
        assert_eq!(txn.commit(), hash);

        // Wait for a Decide event containing transaction matching the one we sent
        wait_for_decide_on_handle(&mut events, &txn).await;
    }

    /// Test the state signature API.
    pub async fn state_signature_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_logging();
        setup_backtrace();

        let port = pick_unused_port().expect("No ports free");

        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerVersion> = Client::new(url);

        let options = opt(Options::from(options::Http { port }));
        let network = TestNetwork::new(options, [NoStorage; TestConfig::NUM_NODES]).await;

        let mut height: u64;
        // Wait for block >=2 appears
        // It's waiting for an extra second to make sure that the signature is generated
        loop {
            height = network
                .server
                .consensus()
                .get_decided_leaf()
                .await
                .get_height();
            sleep(std::time::Duration::from_secs(1)).await;
            if height >= 2 {
                break;
            }
        }
        // we cannot verify the signature now, because we don't know the stake table
        client
            .get::<StateSignatureRequestBody>(&format!("state-signature/block/{}", height))
            .send()
            .await
            .unwrap();
    }

    /// Test the state API with custom options.
    ///
    /// The `opt` function can be used to modify the [`Options`] which are used to start the server.
    /// By default, the options are the minimal required to run this test (configuring a port and
    /// enabling the state API). `opt` may add additional functionality (e.g. adding a query module
    /// to test a different initialization path) but should not remove or modify the existing
    /// functionality (e.g. removing the state module or changing the port).
    pub async fn state_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_logging();
        setup_backtrace();

        let port = pick_unused_port().expect("No ports free");
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerVersion> = Client::new(url);

        let options = opt(Options::from(options::Http { port }).catchup(Default::default()));
        let mut network = TestNetwork::new(options, [NoStorage; TestConfig::NUM_NODES]).await;
        client.connect(None).await;

        // Wait for a few blocks to be decided.
        let mut events = network.server.get_event_stream();
        loop {
            if let Event {
                event: EventType::Decide { leaf_chain, .. },
                ..
            } = events.next().await.unwrap()
            {
                if leaf_chain
                    .iter()
                    .any(|LeafInfo { leaf, .. }| leaf.get_block_header().height > 2)
                {
                    break;
                }
            }
        }

        // Stop consensus running on the node so we freeze the decided and undecided states.
        network.server.consensus_mut().shut_down().await;

        // Decided fee state: absent account.
        let res = client
            .get::<AccountQueryData>(&format!("catchup/account/{:x}", Address::default()))
            .send()
            .await
            .unwrap();
        assert_eq!(res.balance, 0.into());
        assert_eq!(
            res.proof
                .verify(
                    &network
                        .server
                        .consensus()
                        .get_decided_state()
                        .await
                        .fee_merkle_tree
                        .commitment()
                )
                .unwrap(),
            0.into()
        );

        // Undecided fee state: absent account.
        let leaf = network.server.consensus().get_decided_leaf().await;
        let view = leaf.get_view_number() + 1;
        let res = client
            .get::<AccountQueryData>(&format!(
                "catchup/{}/account/{:x}",
                view.get_u64(),
                Address::default()
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.balance, 0.into());
        assert_eq!(
            res.proof
                .verify(
                    &network
                        .server
                        .consensus()
                        .get_state(view)
                        .await
                        .unwrap()
                        .fee_merkle_tree
                        .commitment()
                )
                .unwrap(),
            0.into()
        );

        // Decided block state.
        let res = client
            .get::<BlocksFrontier>("catchup/blocks")
            .send()
            .await
            .unwrap();
        let root = &network
            .server
            .consensus()
            .get_decided_state()
            .await
            .block_merkle_tree
            .commitment();
        BlockMerkleTree::verify(root.digest(), root.size() - 1, res)
            .unwrap()
            .unwrap();

        // Undecided block state.
        let res = client
            .get::<BlocksFrontier>(&format!("catchup/{}/blocks", view.get_u64()))
            .send()
            .await
            .unwrap();
        let root = &network
            .server
            .consensus()
            .get_state(view)
            .await
            .unwrap()
            .block_merkle_tree
            .commitment();
        BlockMerkleTree::verify(root.digest(), root.size() - 1, res)
            .unwrap()
            .unwrap();
    }
}

#[cfg(test)]
#[espresso_macros::generic_tests]
mod api_tests {
    use self::options::HotshotEvents;

    use super::*;
    use crate::{
        persistence::no_storage::NoStorage,
        testing::{wait_for_decide_on_handle, TestConfig},
        Header,
    };
    use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
    use committable::Committable;
    use data_source::testing::TestableSequencerDataSource;
    use endpoints::NamespaceProofQueryData;
    use es_version::SequencerVersion;
    use futures::stream::StreamExt;
    use hotshot_query_service::availability::LeafQueryData;
    use hotshot_types::vid::vid_scheme;
    use portpicker::pick_unused_port;
    use surf_disco::Client;
    use test_helpers::{
        state_signature_test_helper, state_test_helper, status_test_helper, submit_test_helper,
        TestNetwork,
    };
    use tide_disco::error::ServerError;

    #[async_std::test]
    pub(crate) async fn submit_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        submit_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[async_std::test]
    pub(crate) async fn status_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        status_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[async_std::test]
    pub(crate) async fn state_signature_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        state_signature_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[async_std::test]
    pub(crate) async fn test_namespace_query<D: TestableSequencerDataSource>() {
        setup_logging();
        setup_backtrace();

        let vid = vid_scheme(5);
        let txn = Transaction::new(Default::default(), vec![1, 2, 3, 4]);

        // Start query service.
        let port = pick_unused_port().expect("No ports free");
        let storage = D::create_storage().await;
        let network = TestNetwork::new(
            D::options(&storage, options::Http { port }.into()).submit(Default::default()),
            [NoStorage; TestConfig::NUM_NODES],
        )
        .await;
        let mut events = network.server.get_event_stream();

        // Connect client.
        let client: Client<ServerError, SequencerVersion> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        client.connect(None).await;

        // Wait for at least one empty block to be sequenced (after consensus starts VID).
        client
            .socket("availability/stream/leaves/0")
            .subscribe::<LeafQueryData<SeqTypes>>()
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();

        let hash = client
            .post("submit/submit")
            .body_json(&txn)
            .unwrap()
            .send()
            .await
            .unwrap();
        assert_eq!(txn.commit(), hash);

        // Wait for a Decide event containing transaction matching the one we sent
        let block_height = wait_for_decide_on_handle(&mut events, &txn).await as usize;
        tracing::info!(block_height, "transaction sequenced");
        let mut found_txn = false;
        let mut found_empty_block = false;
        for block_num in 0..=block_height {
            let header: Header = client
                .get(&format!("availability/header/{block_num}"))
                .send()
                .await
                .unwrap();
            let ns_query_res: NamespaceProofQueryData = client
                .get(&format!("availability/block/{block_num}/namespace/0"))
                .send()
                .await
                .unwrap();
            ns_query_res
                .proof
                .verify(&vid, &header.payload_commitment, &header.ns_table)
                .unwrap();

            found_empty_block = found_empty_block || ns_query_res.transactions.is_empty();

            for txn in ns_query_res.transactions {
                if txn.commit() == hash {
                    // Ensure that we validate an inclusion proof
                    found_txn = true;
                }
            }
        }
        assert!(found_txn);
        assert!(found_empty_block);
    }

    #[async_std::test]
    pub(crate) async fn state_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        state_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[async_std::test]
    pub(crate) async fn test_hotshot_event_streaming<D: TestableSequencerDataSource>() {
        use hotshot_events_service::events_source::BuilderEvent;
        use HotshotEvents;

        setup_logging();

        setup_backtrace();

        let hotshot_event_streaming_port =
            pick_unused_port().expect("No ports free for hotshot event streaming");
        let query_service_port = pick_unused_port().expect("No ports free for query service");

        let url = format!("http://localhost:{hotshot_event_streaming_port}")
            .parse()
            .unwrap();

        let hotshot_events = HotshotEvents {
            events_service_port: hotshot_event_streaming_port,
        };

        let client: Client<ServerError, SequencerVersion> = Client::new(url);

        let options = Options::from(options::Http {
            port: query_service_port,
        })
        .hotshot_events(hotshot_events);

        let _network = TestNetwork::new(options, [NoStorage; TestConfig::NUM_NODES]).await;

        let mut subscribed_events = client
            .socket("hotshot-events/events")
            .subscribe::<BuilderEvent<SeqTypes>>()
            .await
            .unwrap();

        let total_count = 5;
        // wait for these events to receive on client 1
        let mut receive_count = 0;
        loop {
            let event = subscribed_events.next().await.unwrap();
            tracing::info!(
                "Received event in hotshot event streaming Client 1: {:?}",
                event
            );
            receive_count += 1;
            if receive_count > total_count {
                tracing::info!("Client Received atleast desired events, exiting loop");
                break;
            }
        }
        // Offset 1 is due to the startup event info
        assert_eq!(receive_count, total_count + 1);
    }
}

#[cfg(test)]
mod test {
    use self::{
        data_source::testing::TestableSequencerDataSource, sql::DataSource as SqlDataSource,
    };
    use super::*;
    use crate::{
        catchup::{mock::MockStateCatchup, StatePeers},
        persistence::no_storage::NoStorage,
        state::{FeeAccount, FeeAmount},
        testing::TestConfig,
        Header,
    };
    use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
    use async_std::task::sleep;
    use committable::Commitment;
    use es_version::{SequencerVersion, SEQUENCER_VERSION};
    use futures::{
        future::{self, join_all},
        stream::{StreamExt, TryStreamExt},
    };
    use hotshot::types::EventType;
    use hotshot_query_service::{
        availability::{BlockQueryData, LeafQueryData},
        types::HeightIndexed,
    };
    use hotshot_types::{
        event::LeafInfo,
        traits::{metrics::NoMetrics, node_implementation::ConsensusTime},
    };
    use jf_primitives::merkle_tree::prelude::{MerkleProof, Sha3Node};
    use portpicker::pick_unused_port;
    use std::time::Duration;
    use surf_disco::Client;
    use test_helpers::{
        state_signature_test_helper, state_test_helper, status_test_helper, submit_test_helper,
        TestNetwork,
    };
    use tide_disco::{app::AppHealth, error::ServerError, healthcheck::HealthStatus};

    #[async_std::test]
    async fn test_healthcheck() {
        setup_logging();
        setup_backtrace();

        let port = pick_unused_port().expect("No ports free");
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerVersion> = Client::new(url);
        let options = Options::from(options::Http { port });
        let _network = TestNetwork::new(options, [NoStorage; TestConfig::NUM_NODES]).await;

        client.connect(None).await;
        let health = client.get::<AppHealth>("healthcheck").send().await.unwrap();
        assert_eq!(health.status, HealthStatus::Available);
    }

    #[async_std::test]
    async fn status_test_without_query_module() {
        status_test_helper(|opt| opt).await
    }

    #[async_std::test]
    async fn submit_test_without_query_module() {
        submit_test_helper(|opt| opt).await
    }

    #[async_std::test]
    async fn state_signature_test_without_query_module() {
        state_signature_test_helper(|opt| opt).await
    }

    #[async_std::test]
    async fn state_test_without_query_module() {
        state_test_helper(|opt| opt).await
    }

    #[async_std::test]
    async fn test_merklized_state_api() {
        setup_logging();
        setup_backtrace();

        let port = pick_unused_port().expect("No ports free");

        let storage = SqlDataSource::create_storage().await;
        let options = SqlDataSource::options(
            &storage,
            Options::from(options::Http { port })
                .state(Default::default())
                .status(Default::default()),
        );

        let mut network = TestNetwork::new(options, [NoStorage; TestConfig::NUM_NODES]).await;
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerVersion> = Client::new(url);

        client.connect(None).await;

        // Wait until some blocks have been decided.
        tracing::info!("waiting for blocks");
        let blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(4)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        // sleep for few seconds so that state data is upserted
        tracing::info!("waiting for state to be inserted");
        sleep(Duration::from_secs(5)).await;
        network.stop_consensus().await;

        for block in blocks {
            let i = block.height();
            tracing::info!(i, "get block state");
            let path = client
                .get::<MerkleProof<Commitment<Header>, u64, Sha3Node, 3>>(&format!(
                    "block-state/{}/{i}",
                    i + 1
                ))
                .send()
                .await
                .unwrap();
            assert_eq!(*path.elem().unwrap(), block.hash());

            tracing::info!(i, "get fee state");
            let account = TestConfig::builder_key().fee_account();
            let path = client
                .get::<MerkleProof<FeeAmount, FeeAccount, Sha3Node, 256>>(&format!(
                    "fee-state/{}/{}",
                    i + 1,
                    account
                ))
                .send()
                .await
                .unwrap();
            assert_eq!(*path.index(), account);
            assert!(*path.elem().unwrap() > 0.into(), "{:?}", path.elem());
        }
    }

    #[async_std::test]
    async fn test_catchup() {
        setup_logging();
        setup_backtrace();

        // Start a sequencer network, using the query service for catchup.
        let port = pick_unused_port().expect("No ports free");
        let mut network = TestNetwork::with_state(
            Options::from(options::Http { port }).catchup(Default::default()),
            Default::default(),
            [NoStorage; TestConfig::NUM_NODES],
            std::array::from_fn(|_| {
                StatePeers::<SequencerVersion>::from_urls(vec![format!("http://localhost:{port}")
                    .parse()
                    .unwrap()])
            }),
        )
        .await;

        // Wait for replica 0 to reach a (non-genesis) decide, before disconnecting it.
        let mut events = network.peers[0].get_event_stream();
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            if leaf_chain[0].leaf.get_height() > 0 {
                break;
            }
        }

        // Shut down and restart replica 0. We don't just stop consensus and restart it; we fully
        // drop the node and recreate it so it loses all of its temporary state and starts off from
        // genesis. It should be able to catch up by listening to proposals and then rebuild its
        // state from its peers.
        tracing::info!("shutting down node");
        network.peers.remove(0);

        // Wait for a few blocks to pass while the node is down, so it falls behind.
        network
            .server
            .get_event_stream()
            .filter(|event| future::ready(matches!(event.event, EventType::Decide { .. })))
            .take(3)
            .collect::<Vec<_>>()
            .await;

        tracing::info!("restarting node");
        let node = network
            .cfg
            .init_node(
                1,
                ValidatedState::default(),
                NoStorage,
                StatePeers::<SequencerVersion>::from_urls(vec![format!("http://localhost:{port}")
                    .parse()
                    .unwrap()]),
                &NoMetrics,
                test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                SEQUENCER_VERSION,
            )
            .await;
        let mut events = node.get_event_stream();

        // Wait for a (non-genesis) block proposed by each node, to prove that the lagging node has
        // caught up and all nodes are in sync.
        let mut proposers = [false; TestConfig::NUM_NODES];
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
                let height = leaf.get_height();
                let leaf_builder =
                    (leaf.get_view_number().get_u64() as usize) % TestConfig::NUM_NODES;
                if height == 0 {
                    continue;
                }

                tracing::info!(
                    "waiting for blocks from {proposers:?}, block {height} is from {leaf_builder}",
                );
                proposers[leaf_builder] = true;
            }

            if proposers.iter().all(|has_proposed| *has_proposed) {
                break;
            }
        }
    }

    #[async_std::test]
    pub(crate) async fn test_restart() {
        setup_logging();
        setup_backtrace();

        // Initialize nodes.
        let storage =
            join_all((0..TestConfig::NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence = join_all(
            storage
                .iter()
                .map(<SqlDataSource as TestableSequencerDataSource>::connect),
        )
        .await
        .try_into()
        .unwrap();
        let port = pick_unused_port().unwrap();
        let mut network = TestNetwork::with_state(
            SqlDataSource::options(&storage[0], options::Http { port }.into())
                .state(Default::default())
                .status(Default::default()),
            Default::default(),
            persistence,
            std::array::from_fn(|_| MockStateCatchup::default()),
        )
        .await;

        // Connect client.
        let client: Client<ServerError, SequencerVersion> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        client.connect(None).await;
        tracing::info!(port, "server running");

        // Wait until some blocks have been decided.
        client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(3)
            .collect::<Vec<_>>()
            .await;

        // Shut down the consensus nodes.
        tracing::info!("shutting down nodes");
        network.stop_consensus().await;

        // Get the block height we reached.
        let height = client
            .get::<usize>("status/block-height")
            .send()
            .await
            .unwrap();
        tracing::info!("decided {height} blocks before shutting down");

        // Get the decided chain, so we can check consistency after the restart.
        let chain: Vec<LeafQueryData<SeqTypes>> = client
            .socket("availability/stream/leaves/0")
            .subscribe()
            .await
            .unwrap()
            .take(height)
            .try_collect()
            .await
            .unwrap();
        let decided_view = chain.last().unwrap().leaf().get_view_number();

        // Get the most recent state, for catchup.
        let state = network.server.consensus().get_decided_state().await;
        tracing::info!(?decided_view, ?state, "consensus state");

        // Wait for merklized state storage to update.
        while let Err(err) = client
            .get::<()>(&format!("block-state/{}/{}", height - 1, height - 2))
            .send()
            .await
        {
            tracing::info!(
                height,
                "waiting for merklized state to become available ({err:#})"
            );
            sleep(Duration::from_secs(1)).await;
        }

        // Fully shut down the API servers.
        drop(network);

        // Start up again, resuming from the last decided leaf.
        let port = pick_unused_port().expect("No ports free");
        let persistence = join_all(
            storage
                .iter()
                .map(<SqlDataSource as TestableSequencerDataSource>::connect),
        )
        .await
        .try_into()
        .unwrap();
        let _network = TestNetwork::with_state(
            SqlDataSource::options(&storage[0], options::Http { port }.into())
                .catchup(Default::default()),
            Default::default(),
            persistence,
            std::array::from_fn(|_| {
                // Catchup using node 0 as a peer. Node 0 was running the archival state service
                // before the restart, so it should be able to resume without catching up by loading
                // state from storage.
                StatePeers::<SequencerVersion>::from_urls(vec![format!("http://localhost:{port}")
                    .parse()
                    .unwrap()])
            }),
        )
        .await;
        let client: Client<ServerError, SequencerVersion> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        client.connect(None).await;
        tracing::info!(port, "server running");

        // Make sure we can decide new blocks after the restart.
        tracing::info!("waiting for decide, height {height}");
        let new_leaf: LeafQueryData<SeqTypes> = client
            .socket(&format!("availability/stream/leaves/{height}"))
            .subscribe()
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(new_leaf.height(), height as u64);
        assert_eq!(
            new_leaf.leaf().get_parent_commitment(),
            chain[height - 1].hash()
        );

        // Ensure the new chain is consistent with the old chain.
        let new_chain: Vec<LeafQueryData<SeqTypes>> = client
            .socket("availability/stream/leaves/0")
            .subscribe()
            .await
            .unwrap()
            .take(height)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(chain, new_chain);
    }
}
