use super::*;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_healthcheck() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ClientErr, StaticVersion<0, 1>> = Client::new(url);
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
    let client: Client<ClientErr, SequencerApiVersion> = Client::new(url);

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
async fn test_database_metadata_endpoints() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let storage = SqlDataSource::create_storage().await;
    let options = SqlDataSource::options(&storage, Options::with_port(port));

    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let _network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ClientErr, SequencerApiVersion> = Client::new(url);
    client.connect(Some(Duration::from_secs(15))).await;

    let table_sizes = client
        .get::<Vec<data_source::TableSize>>("database/table-sizes")
        .send()
        .await
        .unwrap();
    assert!(!table_sizes.is_empty());

    // Deferred backfill migrations register tracking rows at node startup, so the list may
    // be non-empty; just check the entries are well-formed.
    let migration_status = client
        .get::<Vec<data_source::MigrationStatus>>("database/migration-status")
        .send()
        .await
        .unwrap();
    assert!(migration_status.iter().all(|m| !m.name.is_empty()));
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_fetch_config() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url: Url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ClientErr, StaticVersion<0, 1>> = Client::new(url.clone());

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
