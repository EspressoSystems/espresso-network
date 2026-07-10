use rand::thread_rng;

use super::*;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_tx_metadata() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

    let storage = SqlDataSource::create_storage().await;
    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(
            SqlDataSource::options(&storage, Options::with_port(port))
                .submit(Default::default())
                .explorer(Default::default()),
        )
        .network_config(network_config)
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let mut events = network.server.event_stream();

    client.connect(None).await;

    // Submit a few transactions in different namespaces.
    let namespace_counts = [(101, 1), (102, 2), (103, 3)];
    for (ns, count) in &namespace_counts {
        for i in 0..*count {
            let ns_id = NamespaceId::from(*ns as u64);
            let txn = Transaction::new(ns_id, vec![*ns, i]);
            client
                .post::<()>("submit/submit")
                .body_json(&txn)
                .unwrap()
                .send()
                .await
                .unwrap();
            let (block, _) = wait_for_decide_on_handle(&mut events, &txn).await;

            // Block summary should contain information about the namespace.
            let summary: BlockSummaryQueryData<SeqTypes> = client
                .get(&format!("availability/block/summary/{block}"))
                .send()
                .await
                .unwrap();
            let ns_info = summary.namespaces();
            assert_eq!(ns_info.len(), 1);
            assert_eq!(ns_info.keys().copied().collect::<Vec<_>>(), vec![ns_id]);
            assert_eq!(ns_info[&ns_id].num_transactions, 1);
            assert_eq!(ns_info[&ns_id].size, txn.size_in_block(true));
        }
    }

    // List transactions in each namespace.
    for (ns, count) in &namespace_counts {
        tracing::info!(ns, "list transactions in namespace");

        let ns_id = NamespaceId::from(*ns as u64);
        let summaries: TransactionSummariesResponse<SeqTypes> = client
            .get(&format!(
                "explorer/transactions/latest/{count}/namespace/{ns_id}"
            ))
            .send()
            .await
            .unwrap();
        let txs = summaries.transaction_summaries;
        assert_eq!(txs.len(), *count as usize);

        // Check that transactions are listed in descending order.
        for i in 0..*count {
            let summary = &txs[i as usize];
            let expected = Transaction::new(ns_id, vec![*ns, count - i - 1]);
            assert_eq!(summary.rollups, vec![ns_id]);
            assert_eq!(summary.hash, expected.commit());
        }
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_aggregator_namespace_endpoints() {
    let mut rng = thread_rng();

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let url = format!("http://localhost:{port}").parse().unwrap();
    tracing::info!("Sequencer URL = {url}");
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

    let options = Options::with_port(port).submit(Default::default());
    const NUM_NODES: usize = 2;
    // Initialize storage for each node
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;

    let persistence_options: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let network_config = TestConfigBuilder::default().build();

    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(&storage[0], options))
        .network_config(network_config)
        .persistences(persistence_options.clone())
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let mut events = network.server.event_stream();
    let start = Instant::now();
    let mut total_transactions = 0;
    let mut tx_heights = Vec::new();
    let mut sizes = HashMap::new();
    // inserting transactions for some namespaces
    // the number of transactions inserted is equal to namespace number.
    for namespace in 1..=4 {
        for _count in 0..namespace {
            // Generate a random payload length between 4 and 10 bytes
            let payload_len = rng.gen_range(4..=10);
            let payload: Vec<u8> = (0..payload_len).map(|_| rng.r#gen()).collect();

            let txn = Transaction::new(NamespaceId::from(namespace as u32), payload);

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
            let (height, size) = wait_for_decide_on_handle(&mut events, &txn).await;
            tx_heights.push(height);
            total_transactions += 1;
            *sizes.entry(namespace).or_insert(0) += size;
        }
    }

    let duration = start.elapsed();

    println!("Time elapsed to submit transactions: {duration:?}");

    let last_tx_height = tx_heights.last().unwrap();

    // Decide events fire when consensus decides a block, but the aggregator that backs these
    // endpoints runs as a separate background task. Wait for it to have written rows up to
    // last_tx_height before asserting; otherwise queries can hit a not-yet-aggregated height
    // and 404.
    let aggregator_deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let count = client
            .get::<u64>(&format!("node/transactions/count/{last_tx_height}"))
            .send()
            .await
            .ok();
        if count == Some(total_transactions) {
            break;
        }
        assert!(
            Instant::now() < aggregator_deadline,
            "aggregator did not catch up to height {last_tx_height} (got {count:?}, expected \
             {total_transactions})"
        );
        sleep(Duration::from_secs(1)).await;
    }

    for namespace in 1..=4 {
        let count = client
            .get::<u64>(&format!("node/transactions/count/namespace/{namespace}"))
            .send()
            .await
            .unwrap();
        assert_eq!(
            count, namespace as u64,
            "Incorrect transaction count for namespace {namespace}: expected {namespace}, got \
             {count}"
        );

        // check the range endpoint
        let to_endpoint_count = client
            .get::<u64>(&format!(
                "node/transactions/count/namespace/{namespace}/{last_tx_height}"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(
            to_endpoint_count, namespace as u64,
            "Incorrect transaction count for range endpoint (to only) for namespace {namespace}: \
             expected {namespace}, got {to_endpoint_count}"
        );

        // check the range endpoint
        let from_to_endpoint_count = client
            .get::<u64>(&format!(
                "node/transactions/count/namespace/{namespace}/0/{last_tx_height}"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(
            from_to_endpoint_count, namespace as u64,
            "Incorrect transaction count for range endpoint (from-to) for namespace {namespace}: \
             expected {namespace}, got {from_to_endpoint_count}"
        );

        let ns_size = client
            .get::<usize>(&format!("node/payloads/size/namespace/{namespace}"))
            .send()
            .await
            .unwrap();

        let expected_ns_size = *sizes.get(&namespace).unwrap();
        assert_eq!(
            ns_size, expected_ns_size,
            "Incorrect payload size for namespace {namespace}: expected {expected_ns_size}, got \
             {ns_size}"
        );

        let ns_size_to = client
            .get::<usize>(&format!(
                "node/payloads/size/namespace/{namespace}/{last_tx_height}"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(
            ns_size_to, expected_ns_size,
            "Incorrect payload size for namespace {namespace} up to height {last_tx_height}: \
             expected {expected_ns_size}, got {ns_size_to}"
        );

        let ns_size_from_to = client
            .get::<usize>(&format!(
                "node/payloads/size/namespace/{namespace}/0/{last_tx_height}"
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(
            ns_size_from_to, expected_ns_size,
            "Incorrect payload size for namespace {namespace} from 0 to height {last_tx_height}: \
             expected {expected_ns_size}, got {ns_size_from_to}"
        );
    }

    let total_tx_count = client
        .get::<u64>("node/transactions/count")
        .send()
        .await
        .unwrap();
    assert_eq!(
        total_tx_count, total_transactions,
        "Incorrect total transaction count: expected {total_transactions}, got {total_tx_count}"
    );

    let total_payload_size = client
        .get::<usize>("node/payloads/size")
        .send()
        .await
        .unwrap();

    let expected_total_size: usize = sizes.values().copied().sum();
    assert_eq!(
        total_payload_size, expected_total_size,
        "Incorrect total payload size: expected {expected_total_size}, got {total_payload_size}"
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_stream_transactions_endpoint() {
    // This test submits transactions to a sequencer for multiple namespaces,
    // waits for them to be decided, and then verifies that:
    // 1. All transactions appear in the transaction stream.
    // 2. Each namespace-specific transaction stream only includes the transactions of that namespace.

    let mut rng = thread_rng();

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let url = format!("http://localhost:{port}").parse().unwrap();
    tracing::info!("Sequencer URL = {url}");
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

    let options = Options::with_port(port).submit(Default::default());
    const NUM_NODES: usize = 2;
    // Initialize storage for each node
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;

    let persistence_options: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let network_config = TestConfigBuilder::default().build();

    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(&storage[0], options))
        .network_config(network_config)
        .persistences(persistence_options.clone())
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let mut events = network.server.event_stream();
    let mut all_transactions = HashMap::new();
    let mut namespace_tx: HashMap<_, HashSet<_>> = HashMap::new();

    // Submit transactions to namespaces 1 through 4

    for namespace in 1..=4 {
        for _count in 0..namespace {
            let payload_len = rng.gen_range(4..=10);
            let payload: Vec<u8> = (0..payload_len).map(|_| rng.r#gen()).collect();

            let txn = Transaction::new(NamespaceId::from(namespace as u32), payload);

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
            // Store transaction for later validation

            all_transactions.insert(txn.commit(), txn.clone());
            namespace_tx.entry(namespace).or_default().insert(txn);
        }
    }

    let mut transactions = client
        .socket("availability/stream/transactions/0")
        .subscribe::<TransactionQueryData<SeqTypes>>()
        .await
        .expect("failed to subscribe to transactions endpoint");

    let mut count = 0;
    while let Some(tx) = transactions.next().await {
        let tx = tx.unwrap();
        let expected = all_transactions
            .get(&tx.transaction().commit())
            .expect("txn not found ");
        assert_eq!(tx.transaction(), expected, "invalid transaction");
        count += 1;

        if count == all_transactions.len() {
            break;
        }
    }

    // Validate namespace-specific stream endpoint

    for (namespace, expected_ns_txns) in &namespace_tx {
        let mut api_namespace_txns = client
            .socket(&format!(
                "availability/stream/transactions/0/namespace/{namespace}",
            ))
            .subscribe::<TransactionQueryData<SeqTypes>>()
            .await
            .unwrap_or_else(|_| {
                panic!("failed to subscribe to transactions namespace {namespace}")
            });

        let mut received = HashSet::new();

        while let Some(res) = api_namespace_txns.next().await {
            let tx = res.expect("stream error");
            received.insert(tx.transaction().clone());

            if received.len() == expected_ns_txns.len() {
                break;
            }
        }

        assert_eq!(
            received, *expected_ns_txns,
            "Mismatched transactions for namespace {namespace}"
        );
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_namespace_query_compat_v0_2() {
    test_namespace_query_compat_helper(Upgrade::trivial(FEE_VERSION)).await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_namespace_query_compat_v0_3() {
    test_namespace_query_compat_helper(Upgrade::trivial(EPOCH_VERSION)).await;
}

async fn test_namespace_query_compat_helper(upgrade: Upgrade) {
    // Number of nodes running in the test network.
    const NUM_NODES: usize = 5;

    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");
    let url: Url = format!("http://localhost:{port}").parse().unwrap();

    let test_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(Options::from(options::Http {
            port,
            max_connections: None,
            axum_port: None,
            tonic_port: None,
        }))
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![url.clone()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .network_config(test_config)
        .build();

    let mut network = TestNetwork::new(config, upgrade).await;
    let mut events = network.server.event_stream();

    // Submit a transaction.
    let ns = NamespaceId::from(10_000u64);
    let tx = Transaction::new(ns, vec![1, 2, 3]);
    network.server.submit_transaction(tx.clone()).await.unwrap();
    let block = wait_for_decide_on_handle(&mut events, &tx).await.0;

    // Check namespace proof queries.
    let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);
    client.connect(None).await;

    let (header, common): (Header, VidCommonQueryData<SeqTypes>) = try_join!(
        client.get(&format!("availability/header/{block}")).send(),
        client
            .get(&format!("availability/vid/common/{block}"))
            .send()
    )
    .unwrap();
    let version = header.version();

    // The latest version of the API (whether we specifically ask for v1 or let the redirect
    // occur) will give us a namespace proof no matter which VID version is in use.
    for api_ver in ["/v1", ""] {
        tracing::info!("test namespace API version: {api_ver}");

        let ns_proof: NamespaceProofQueryData = client
            .get(&format!(
                "{api_ver}/availability/block/{block}/namespace/{ns}"
            ))
            .send()
            .await
            .unwrap();
        let proof = ns_proof.proof.as_ref().unwrap();
        if version < EPOCH_VERSION {
            assert!(matches!(proof, NsProof::V0(..)));
        } else {
            assert!(matches!(proof, NsProof::V1(..)));
        }
        let (txs, ns_from_proof) = proof
            .verify(
                header.ns_table(),
                &header.payload_commitment(),
                common.common(),
            )
            .unwrap();
        assert_eq!(ns_from_proof, ns);
        assert_eq!(txs, ns_proof.transactions);
        assert_eq!(txs, std::slice::from_ref(&tx));

        // Test range endpoint.
        let ns_proofs: Vec<NamespaceProofQueryData> = client
            .get(&format!(
                "{api_ver}/availability/block/{}/{}/namespace/{ns}",
                block,
                block + 1
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(&ns_proofs, std::slice::from_ref(&ns_proof));

        // Any API version can correctly tell us that the namespace does not exist.
        let ns_proof: NamespaceProofQueryData = client
            .get(&format!(
                "{api_ver}/availability/block/{}/namespace/{ns}",
                block - 1
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(ns_proof.proof, None);
        assert_eq!(ns_proof.transactions, vec![]);

        // Test streaming.
        let mut proofs = client
            .socket(&format!(
                "{api_ver}/availability/stream/blocks/0/namespace/{ns}"
            ))
            .subscribe()
            .await
            .unwrap();
        for i in 0.. {
            tracing::info!(i, "stream proof");
            let proof: NamespaceProofQueryData = proofs.next().await.unwrap().unwrap();
            if proof.proof.is_none() {
                tracing::info!("waiting for non-trivial proof from stream");
                continue;
            }
            assert_eq!(&proof.transactions, std::slice::from_ref(&tx));
            break;
        }
    }

    // The legacy version of the API only works for old VID.
    tracing::info!("test namespace API version: v0");
    if version < EPOCH_VERSION {
        let ns_proof: ADVZNamespaceProofQueryData = client
            .get(&format!("v0/availability/block/{block}/namespace/{ns}"))
            .send()
            .await
            .unwrap();
        let proof = ns_proof.proof.as_ref().unwrap();
        let VidCommon::V0(common) = common.common() else {
            panic!("wrong VID common version");
        };
        let (txs, ns_from_proof) = proof
            .verify(header.ns_table(), &header.payload_commitment(), common)
            .unwrap();
        assert_eq!(ns_from_proof, ns);
        assert_eq!(txs, ns_proof.transactions);
        assert_eq!(&txs, std::slice::from_ref(&tx));

        // Test range endpoint.
        let ns_proofs: Vec<ADVZNamespaceProofQueryData> = client
            .get(&format!(
                "v0/availability/block/{}/{}/namespace/{ns}",
                block,
                block + 1
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(&ns_proofs, std::slice::from_ref(&ns_proof));
    } else {
        // It will fail if we ask for a proof for a block using new VID.
        client
            .get::<ADVZNamespaceProofQueryData>(&format!(
                "v0/availability/block/{block}/namespace/{ns}"
            ))
            .send()
            .await
            .unwrap_err();
    }

    // Any API version can correctly tell us that the namespace does not exist.
    let ns_proof: ADVZNamespaceProofQueryData = client
        .get(&format!(
            "v0/availability/block/{}/namespace/{ns}",
            block - 1
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(ns_proof.proof, None);
    assert_eq!(ns_proof.transactions, vec![]);

    // Use the legacy API to stream namespace proofs until we get to a non-trivial proof or a
    // VID version we can't deal with.
    let mut proofs = client
        .socket(&format!("v0/availability/stream/blocks/0/namespace/{ns}"))
        .subscribe()
        .await
        .unwrap();
    for i in 0.. {
        tracing::info!(i, "stream proof");
        let proof: ADVZNamespaceProofQueryData = match proofs.next().await {
            Some(proof) => proof.unwrap(),
            None => {
                // Steam not expected to end on legacy consensus version.
                assert!(
                    version >= EPOCH_VERSION,
                    "legacy steam ended while still on legacy consensus"
                );
                break;
            },
        };
        if proof.proof.is_none() {
            tracing::info!("waiting for non-trivial proof from stream");
            continue;
        }
        assert_eq!(&proof.transactions, std::slice::from_ref(&tx));
        break;
    }

    network.server.shut_down().await;
}
