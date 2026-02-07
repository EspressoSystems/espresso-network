use std::time::{Duration, Instant};

use alloy::primitives::{Address, U256};
use committable::Commitment;
use espresso_types::{
    v0_3::RewardAmount,
    v0_4::{RewardAccountV2, RewardMerkleTreeV2, REWARD_MERKLE_TREE_V2_HEIGHT},
    FeeAccount, FeeAmount, Header, SeqTypes,
};
use futures::{StreamExt, TryStreamExt};
use hotshot_query_service::{
    availability::BlockQueryData,
    data_source::{
        sql::Config,
        storage::sql::{
            testing::TmpDb, SqlStorage, StorageConnectionType, Transaction as SqlTransaction, Write,
        },
        Transaction, VersionedDataSource,
    },
    merklized_state::{MerklizedState, UpdateStateData},
    types::HeightIndexed,
};
use jf_merkle_tree_compat::{
    prelude::{MerkleProof, Sha3Node},
    LookupResult, MerkleTreeScheme, ToTraversalPath, UniversalMerkleTreeScheme,
};
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
use test_utils::reserve_tcp_port;
use tide_disco::error::ServerError;
use tokio::time::sleep;

type MockSequencerVersions =
    espresso_types::SequencerVersions<espresso_types::EpochVersion, espresso_types::V0_0>;

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_test_merklized_state_api() {
    let port = reserve_tcp_port().expect("Failed to reserve TCP port");

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

fn make_reward_account(i: usize) -> RewardAccountV2 {
    let mut addr_bytes = [0u8; 20];
    addr_bytes[16..20].copy_from_slice(&(i as u32).to_be_bytes());
    RewardAccountV2(Address::from(addr_bytes))
}

async fn insert_test_header(
    tx: &mut SqlTransaction<Write>,
    block_height: u64,
    reward_tree: &RewardMerkleTreeV2,
) {
    let reward_commitment = serde_json::to_value(reward_tree.commitment()).unwrap();
    let test_data = serde_json::json!({
        "block_merkle_tree_root": format!("block_root_{}", block_height),
        "fee_merkle_tree_root": format!("fee_root_{}", block_height),
        "fields": {
            RewardMerkleTreeV2::header_state_commitment_field(): reward_commitment
        }
    });
    tx.upsert(
        "header",
        ["height", "hash", "payload_hash", "timestamp", "data"],
        ["height"],
        [(
            block_height as i64,
            format!("hash_{}", block_height),
            format!("payload_{}", block_height),
            block_height as i64,
            test_data,
        )],
    )
    .await
    .unwrap();
}

async fn batch_insert_proofs(
    tx: &mut SqlTransaction<Write>,
    reward_tree: &RewardMerkleTreeV2,
    accounts: &[RewardAccountV2],
    block_height: u64,
) {
    let proofs_and_paths: Vec<_> = accounts
        .iter()
        .map(|account| {
            let proof = match reward_tree.universal_lookup(*account) {
                LookupResult::Ok(_, proof) => proof,
                LookupResult::NotInMemory => panic!("account not in memory"),
                LookupResult::NotFound(proof) => proof,
            };
            let traversal_path = <RewardAccountV2 as ToTraversalPath<
                { RewardMerkleTreeV2::ARITY },
            >>::to_traversal_path(account, reward_tree.height());
            (proof, traversal_path)
        })
        .collect();

    UpdateStateData::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::insert_merkle_nodes_batch(
        tx,
        proofs_and_paths,
        block_height,
    )
    .await
    .expect("failed to batch insert proofs");
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn slow_test_batch_insertion_20k_accounts() {
    let db = TmpDb::init().await;
    let opt = SqlDataSource::persistence_options(&db);
    let cfg = Config::try_from(&opt).expect("failed to create config from options");
    let storage = SqlStorage::connect(cfg, StorageConnectionType::Query)
        .await
        .expect("failed to connect to storage");

    let num_accounts = 20_000usize;

    let accounts: Vec<RewardAccountV2> = (0..num_accounts).map(make_reward_account).collect();

    tracing::info!("Starting tree update for {} accounts", num_accounts);
    let tree_update_start = Instant::now();
    let mut reward_tree = RewardMerkleTreeV2::new(REWARD_MERKLE_TREE_V2_HEIGHT);
    for (i, account) in accounts.iter().enumerate() {
        let reward_amount = RewardAmount::from(((i + 1) * 100) as u64);
        reward_tree.update(*account, reward_amount).unwrap();
        if (i + 1) % 5_000 == 0 {
            tracing::info!("tree_update: {}/{}", i + 1, num_accounts);
        }
    }
    let tree_update_duration = tree_update_start.elapsed();
    tracing::info!("tree_update complete: {:?}", tree_update_duration);

    let mut tx = storage.write().await.unwrap();
    insert_test_header(&mut tx, 1, &reward_tree).await;

    tracing::info!("Starting batch insert for {} accounts", num_accounts);
    let batch_insert_start = Instant::now();
    batch_insert_proofs(&mut tx, &reward_tree, &accounts, 1).await;
    let batch_insert_duration = batch_insert_start.elapsed();
    tracing::info!("batch_insert complete: {:?}", batch_insert_duration);

    UpdateStateData::<SeqTypes, RewardMerkleTreeV2, { RewardMerkleTreeV2::ARITY }>::set_last_state_height(&mut tx, 1)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    tracing::info!(
        "20k accounts: tree_update={:?}, batch_insert={:?}",
        tree_update_duration,
        batch_insert_duration
    );
}
