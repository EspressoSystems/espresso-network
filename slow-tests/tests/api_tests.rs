use std::time::Duration;

use alloy::primitives::U256;
use committable::Commitment;
use espresso_types::{FeeAccount, FeeAmount, Header, SeqTypes};
use futures::{StreamExt, TryStreamExt};
use hotshot_query_service::{availability::BlockQueryData, types::HeightIndexed};
use jf_merkle_tree_compat::prelude::{MerkleProof, Sha3Node};
use portpicker::pick_unused_port;
use sequencer::{
    api::{
        data_source::testing::TestableSequencerDataSource,
        sql::DataSource as SqlDataSource,
        test_helpers::{TestNetwork, TestNetworkConfigBuilder},
        Options,
    },
    testing::{TestConfig, TestConfigBuilder},
    SequencerApiVersion,
};
use surf_disco::Client;
use tide_disco::error::ServerError;
use tokio::time::sleep;

type MockSequencerVersions =
    espresso_types::SequencerVersions<espresso_types::EpochVersion, espresso_types::V0_0>;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_test_merklized_state_api() {
    let port = pick_unused_port().expect("No ports free");

    let storage = SqlDataSource::create_storage().await;

    let options = SqlDataSource::options(&storage, Options::with_port(port));

    let network_config = TestConfigBuilder::default().build();
    let config = TestNetworkConfigBuilder::default()
        .api_config(options)
        .network_config(network_config)
        .build();
    let mut network = TestNetwork::new(config, MockSequencerVersions::new()).await;
    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ServerError, SequencerApiVersion> = Client::new(url);

    client.connect(Some(Duration::from_secs(15))).await;

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
