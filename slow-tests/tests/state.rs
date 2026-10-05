use std::time::Duration;

use alloy::primitives::U256;
use committable::Commitment;
use espresso_node::{
    SequencerApiVersion,
    api::{
        Options,
        data_source::testing::TestableSequencerDataSource,
        sql::DataSource as SqlDataSource,
        test_helpers::{TestNetwork, TestNetworkConfigBuilder},
    },
    catchup::StatePeers,
    testing::{TestConfig, TestConfigBuilder},
};
use espresso_types::{FeeAccount, FeeAmount, Header, SeqTypes};
use futures::{StreamExt, TryStreamExt, future::join_all};
use hotshot_query_service::{availability::BlockQueryData, types::HeightIndexed};
use hotshot_types::traits::metrics::NoMetrics;
use http_client::{Client, error::ClientErr};
use jf_merkle_tree_compat::prelude::{MerkleProof, Sha3Node};
use test_utils::reserve_tcp_port;
use tokio::time::{sleep, timeout};

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_test_merklized_state_api() {
    const NUM_NODES: usize = 5;
    const EPOCH_HEIGHT: u64 = 20;
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    // SQL persistence on every node and catchup from node 0's query API: at
    // 0.6 the query node fills decided blocks' payloads from its persisted DA
    // proposals, and epoch boundaries need state catchup.
    let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
    let persistence: [_; NUM_NODES] = storage
        .iter()
        .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();

    let network_config = TestConfigBuilder::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(0)
        .build();
    let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
        .api_config(SqlDataSource::options(
            &storage[0],
            Options::with_port(port),
        ))
        .network_config(network_config)
        .persistences(persistence)
        .catchups(std::array::from_fn(|_| {
            StatePeers::<SequencerApiVersion>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                Duration::from_secs(2),
                &NoMetrics,
            )
        }))
        .build()
        .await;
    let mut network = TestNetwork::new(config).await;
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ClientErr, SequencerApiVersion> = Client::new(url);

    client.connect(Some(Duration::from_secs(15))).await;

    // Wait until some blocks have been decided.
    tracing::info!("waiting for blocks");
    let blocks = timeout(
        Duration::from_secs(120),
        client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(4)
            .try_collect::<Vec<_>>(),
    )
    .await
    .expect("the query service did not serve the first blocks in time")
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
        let account = TestConfig::<5>::builder_key().fee_account();
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

    // testing fee_balance api
    let account = TestConfig::<5>::builder_key().fee_account();
    let amount = client
        .get::<Option<FeeAmount>>(&format!("fee-state/fee-balance/latest/{account}"))
        .send()
        .await
        .unwrap()
        .unwrap();
    let expected = U256::MAX;
    assert_eq!(expected, amount.0);
}
