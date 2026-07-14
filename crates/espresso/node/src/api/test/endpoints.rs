use super::*;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_healthcheck() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);
    let options = Options::with_port(port);
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::<5, _, NullStateCatchup>::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let _network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    client.connect(None).await;
    let health = client.get::<AppHealth>("healthcheck").send().await.unwrap();
    assert_eq!(health.status, HealthStatus::Available);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn status_test_without_query_module() {
    status_test_helper(|opt| opt).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn submit_test_without_query_module() {
    submit_test_helper(|opt| opt).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn state_signature_test_without_query_module() {
    state_signature_test_helper(|opt| opt).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn catchup_test_without_query_module() {
    catchup_test_helper(|opt| opt).await
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_leaf_only_data_source() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let storage = SqlDataSource::create_storage().await;
    let options = SqlDataSource::leaf_only_ds_options(&storage, Options::with_port(port)).unwrap();

    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let _network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, SequencerApiVersion> = Client::new(url);

    tracing::info!("waiting for blocks");
    client.connect(Some(Duration::from_secs(15))).await;
    // Wait until some blocks have been decided.

    let account = TestConfig::<5>::builder_key().fee_account();

    let _headers = client
        .socket("availability/stream/headers/0")
        .subscribe::<Header>()
        .await
        .unwrap()
        .take(10)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    for i in 1..5 {
        let leaf = client
            .get::<LeafQueryData<SeqTypes>>(&format!("availability/leaf/{i}"))
            .send()
            .await
            .unwrap();

        assert_eq!(leaf.height(), i);

        let header = client
            .get::<Header>(&format!("availability/header/{i}"))
            .send()
            .await
            .unwrap();

        assert_eq!(header.height(), i);

        let vid = client
            .get::<VidCommonQueryData<SeqTypes>>(&format!("availability/vid/common/{i}"))
            .send()
            .await
            .unwrap();

        assert_eq!(vid.height(), i);

        client
            .get::<MerkleProof<Commitment<Header>, u64, Sha3Node, 3>>(&format!(
                "block-state/{i}/{}",
                i - 1
            ))
            .send()
            .await
            .unwrap();

        client
            .get::<MerkleProof<FeeAmount, FeeAccount, Sha3Node, 256>>(&format!(
                "fee-state/{}/{}",
                i + 1,
                account
            ))
            .send()
            .await
            .unwrap();
    }

    // This would fail even though we have processed atleast 10 leaves
    // this is because light weight nodes only support leaves, headers and VID
    client
        .get::<BlockQueryData<SeqTypes>>("availability/block/1")
        .send()
        .await
        .unwrap_err();
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_fetch_config() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url: surf_disco::Url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url.clone());

    let options = Options::with_port(port).config(Default::default());
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    client.connect(None).await;

    // Fetch a network config from the API server. The first peer URL is bogus, to test the
    // failure/retry case.
    let peers = StatePeers::<StaticVersion<0, 1>>::from_urls(
        vec!["https://notarealnode.network".parse().unwrap(), url],
        Default::default(),
        Duration::from_secs(2),
        &NoMetrics,
    );

    // Fetch the config from node 1, a different node than the one running the service.
    let validator = ValidatorConfig::generated_from_seed_indexed([0; 32], 1, U256::from(1), false);
    let config = peers.fetch_config(validator.clone()).await.unwrap();

    // Check the node-specific information in the recovered config is correct.
    assert_eq!(config.node_index, 1);

    // Check the public information is also correct (with respect to the node that actually
    // served the config, for public keys).
    pretty_assertions::assert_eq!(
        serde_json::to_value(PublicHotShotConfig::from(config.config)).unwrap(),
        serde_json::to_value(PublicHotShotConfig::from(
            network.cfg.hotshot_config().clone()
        ))
        .unwrap()
    );
}

async fn run_hotshot_event_streaming_test(url_suffix: &str) {
    let query_service_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let url = format!("http://localhost:{query_service_port}{url_suffix}")
        .parse()
        .unwrap();

    let client: Client<ServerError, SequencerApiVersion> = Client::new(url);

    let options = Options::with_port(query_service_port).hotshot_events(HotshotEvents);

    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let _network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;

    let mut subscribed_events = client
        .socket("hotshot-events/events")
        .subscribe::<Event<SeqTypes>>()
        .await
        .unwrap();

    let total_count = 5;
    // wait for these events to receive on client 1
    let mut receive_count = 0;
    loop {
        let event = subscribed_events.next().await.unwrap();
        tracing::info!("Received event in hotshot event streaming Client 1: {event:?}");
        receive_count += 1;
        if receive_count > total_count {
            tracing::info!("Client Received at least desired events, exiting loop");
            break;
        }
    }
    assert_eq!(receive_count, total_count + 1);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_hotshot_event_streaming_v0() {
    run_hotshot_event_streaming_test("/v0").await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_hotshot_event_streaming_v1() {
    run_hotshot_event_streaming_test("/v1").await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_hotshot_event_streaming() {
    run_hotshot_event_streaming_test("").await;
}

// TODO when `EPOCH_VERSION` becomes base version we can merge this
// w/ above test.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_hotshot_event_streaming_epoch_progression() {
    let epoch_height = 35;
    let wanted_epochs = 4;

    let network_config = TestConfigBuilder::default()
        .epoch_height(epoch_height)
        .build();

    let query_service_port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let hotshot_url = format!("http://localhost:{query_service_port}")
        .parse()
        .unwrap();

    let client: Client<ServerError, SequencerApiVersion> = Client::new(hotshot_url);
    let options = Options::with_port(query_service_port).hotshot_events(HotshotEvents);

    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config.clone())
        .pos_hook(
            DelegationConfig::VariableAmounts,
            Default::default(),
            POS_V3,
        )
        .await
        .expect("Pos Deployment")
        .build();

    let _network = TestNetwork::new(config, POS_V3).await;

    let mut subscribed_events = client
        .socket("hotshot-events/events")
        .subscribe::<Event<SeqTypes>>()
        .await
        .unwrap();

    let wanted_views = epoch_height * wanted_epochs;

    let mut views = HashSet::new();
    let mut epochs = HashSet::new();
    for _ in 0..=600 {
        let event = subscribed_events.next().await.unwrap();
        let event = event.unwrap();
        let view_number = event.view_number;
        views.insert(view_number.u64());

        if let hotshot::types::EventType::Decide { committing_qc, .. } = event.event {
            assert!(committing_qc.epoch().is_some(), "epochs are live");
            assert!(committing_qc.block_number().is_some());

            let epoch = committing_qc.epoch().unwrap().u64();
            epochs.insert(epoch);

            tracing::debug!(
                "Got decide: epoch: {:?}, block: {:?} ",
                epoch,
                committing_qc.block_number()
            );

            let expected_epoch =
                epoch_from_block_number(committing_qc.block_number().unwrap(), epoch_height);
            tracing::debug!("expected epoch: {expected_epoch}, qc epoch: {epoch}");

            assert_eq!(expected_epoch, epoch);
        }
        if views.contains(&wanted_views) {
            tracing::info!("Client Received at least desired views, exiting loop");
            break;
        }
    }

    // prevent false positive when we overflow the range
    assert!(views.contains(&wanted_views), "Views are not progressing");
    assert!(
        epochs.contains(&wanted_epochs),
        "Epochs are not progressing"
    );
}
