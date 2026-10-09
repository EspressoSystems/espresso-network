//! Payload write and read path through the blob store.

use committable::Committable;
use hotshot_example_types::node_types::TEST_VERSIONS;
use hotshot_types::traits::EncodeBytes;

use super::{testing::TmpDb, *};
use crate::{
    availability::{BlockQueryData, LeafQueryData, PayloadQueryData},
    data_source::storage::{
        AvailabilityStorage, ExplorerStorage, UpdateAvailabilityStorage, blob::BlobLoc,
    },
    explorer::{BlockDetail, BlockIdentifier},
    node::BlockId,
    testing::mocks::{MockPayload, MockTypes, mock_transaction},
};

type Leaf = LeafQueryData<MockTypes>;
type Block = BlockQueryData<MockTypes>;

/// A block at `height` with the given transactions. Blocks with the same `ns_table` and
/// transactions share one `payload` row.
async fn block_at(height: u64, ns_table: u64, txs: &[&[u8]]) -> (Leaf, Block) {
    let mut leaf =
        Leaf::genesis(&Default::default(), &Default::default(), TEST_VERSIONS.test).await;
    let header = leaf.leaf.block_header_mut();
    header.block_number = height;
    header.metadata.num_transactions = ns_table;
    let payload = MockPayload {
        transactions: txs.iter().map(|tx| mock_transaction(tx.to_vec())).collect(),
    };
    let block = Block::new(leaf.header().clone(), payload);
    (leaf, block)
}

async fn connect(db: &TmpDb) -> SqlStorage {
    SqlStorage::connect(db.config(), StorageConnectionType::Query)
        .await
        .unwrap()
}

async fn store(storage: &SqlStorage, leaf: &Leaf, block: &Block) {
    let mut tx = storage.write().await.unwrap();
    tx.insert_leaf(leaf).await.unwrap();
    tx.insert_block(block).await.unwrap();
    tx.commit().await.unwrap();
}

async fn count(storage: &SqlStorage, sql: &str) -> i64 {
    let mut tx = storage.read().await.unwrap();
    let (count,) = query_as::<(i64,)>(sql)
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
    count
}

fn counter(storage: &SqlStorage, name: &str) -> usize {
    storage.metrics.get_counter(name).unwrap().get()
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_payload_roundtrips_on_all_read_paths() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let mut blocks = vec![];
    for (height, txs) in [
        (0, vec![]),
        (1, vec![&[1u8, 2][..], &[3]]),
        (2, vec![&[4][..]]),
    ] {
        let (leaf, block) = block_at(height, height, &txs).await;
        store(&storage, &leaf, &block).await;
        blocks.push(block);
    }

    // Only the non-empty payloads have a locator, and no payload bytes are in SQL.
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 2);
    assert_eq!(
        count(
            &storage,
            "SELECT count(*) FROM payload WHERE length(data) > 0"
        )
        .await,
        0
    );

    let mut tx = storage.read().await.unwrap();
    for (i, block) in blocks.iter().enumerate() {
        let id = BlockId::<MockTypes>::Number(i);
        assert_eq!(&tx.get_block(id).await.unwrap(), block);
        assert_eq!(
            tx.get_payload(id).await.unwrap(),
            PayloadQueryData::from(block.clone())
        );
    }
    let range = tx.get_block_range(0..3).await.unwrap();
    assert_eq!(
        range.into_iter().collect::<QueryResult<Vec<_>>>().unwrap(),
        blocks
    );
    let range = tx.get_payload_range(0..3).await.unwrap();
    assert_eq!(
        range.into_iter().collect::<QueryResult<Vec<_>>>().unwrap(),
        blocks
            .iter()
            .cloned()
            .map(PayloadQueryData::from)
            .collect::<Vec<_>>()
    );
    assert_eq!(tx.get_block_ranges(&[0..1, 1..3]).await.unwrap(), blocks);

    let (_, txn) = blocks[1].enumerate().next().unwrap();
    assert_eq!(
        tx.get_block_with_transaction(txn.commit()).await.unwrap(),
        blocks[1]
    );

    let detail = tx
        .get_block_detail(BlockIdentifier::Height(1))
        .await
        .unwrap();
    assert_eq!(detail, BlockDetail::try_from(blocks[1].clone()).unwrap());
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_empty_payload_writes_no_record() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let blobs = storage.blob_store().unwrap().clone();
    let bytes = blobs.payload_bytes();

    let (leaf, block) = block_at(1, 1, &[]).await;
    store(&storage, &leaf, &block).await;

    assert_eq!(blobs.payload_bytes(), bytes);
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 0);
    let mut tx = storage.read().await.unwrap();
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_inline_payload_is_kept_when_blobs_are_enabled() {
    let db = TmpDb::init().await;
    let inline = connect(&db).await;
    let (leaf1, block1) = block_at(1, 5, &[&[1, 2, 3]]).await;
    store(&inline, &leaf1, &block1).await;
    assert_eq!(
        count(
            &inline,
            "SELECT count(*) FROM payload WHERE length(data) > 0"
        )
        .await,
        1
    );

    // A new height with the same payload must not blank the inline bytes of the shared row.
    let dir = tempfile::tempdir().unwrap();
    let cfg = db.config().blob(BlobCfg {
        dir: dir.path().to_path_buf(),
        share_retention: Duration::from_secs(60),
    });
    let blobbed = SqlStorage::connect(cfg, StorageConnectionType::Query)
        .await
        .unwrap();
    let (leaf2, block2) = block_at(2, 5, &[&[1, 2, 3]]).await;
    store(&blobbed, &leaf2, &block2).await;

    assert_eq!(count(&blobbed, "SELECT count(*) FROM payload").await, 1);
    assert_eq!(
        count(
            &blobbed,
            "SELECT count(*) FROM payload WHERE length(data) > 0"
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &blobbed,
            "SELECT count(*) FROM payload_loc WHERE height = 2"
        )
        .await,
        1
    );
    assert_eq!(count(&blobbed, "SELECT count(*) FROM payload_loc").await, 1);

    let mut tx = blobbed.read().await.unwrap();
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block1);
    assert_eq!(tx.get_block(BlockId::Number(2)).await.unwrap(), block2);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_shared_payload_has_one_row_and_two_locators() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf1, block1) = block_at(1, 7, &[&[9, 9]]).await;
    let (leaf2, block2) = block_at(2, 7, &[&[9, 9]]).await;

    let mut tx = storage.write().await.unwrap();
    tx.insert_leaf(&leaf1).await.unwrap();
    tx.insert_leaf(&leaf2).await.unwrap();
    tx.insert_block_range([&block1, &block2]).await.unwrap();
    tx.commit().await.unwrap();

    assert_eq!(count(&storage, "SELECT count(*) FROM payload").await, 1);
    assert_eq!(count(&storage, "SELECT count(*) FROM payload_loc").await, 2);
    let mut tx = storage.read().await.unwrap();
    assert_eq!(tx.get_block(BlockId::Number(1)).await.unwrap(), block1);
    assert_eq!(tx.get_block(BlockId::Number(2)).await.unwrap(), block2);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_staged_payload_is_not_appended_in_the_transaction() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let blobs = storage.blob_store().unwrap().clone();
    let (leaf1, block1) = block_at(1, 1, &[&[1]]).await;
    let (leaf2, block2) = block_at(2, 2, &[&[2]]).await;

    let loc = blobs
        .append_payload(1, block1.payload.encode().to_vec())
        .await
        .unwrap();
    let staged_bytes = blobs.payload_bytes();
    let mut tx = storage.write().await.unwrap();
    UpdateAvailabilityStorage::<MockTypes>::attach_blobs(&mut tx, vec![(1, loc)]);
    tx.insert_leaf(&leaf1).await.unwrap();
    tx.insert_block(&block1).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(counter(&storage, "blob_staged_in_tx"), 0);
    assert_eq!(blobs.payload_bytes(), staged_bytes);

    store(&storage, &leaf2, &block2).await;
    assert_eq!(counter(&storage, "blob_staged_in_tx"), 1);
    assert!(blobs.payload_bytes() > staged_bytes);

    let stored: String = {
        let mut tx = storage.read().await.unwrap();
        let (loc,) = query_as::<(String,)>("SELECT loc FROM payload_loc WHERE height = 1")
            .fetch_one(tx.as_mut())
            .await
            .unwrap();
        loc
    };
    assert_eq!(stored.parse::<BlobLoc>().unwrap(), loc);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_connect_refuses_locators_without_blob_dir() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf, block) = block_at(1, 1, &[&[1]]).await;
    store(&storage, &leaf, &block).await;
    drop(storage);

    let mut plain = db.config();
    plain.blob = None;
    let err = SqlStorage::connect(plain, StorageConnectionType::Query)
        .await
        .expect_err("connect must fail");
    assert_eq!(
        err.to_string(),
        "payload_loc rows present but ESPRESSO_NODE_BLOB_DIR unset"
    );

    connect(&db).await;
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_unreadable_record_is_missing_until_restored() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let (leaf, block) = block_at(1, 1, &[&[1, 2, 3]]).await;
    store(&storage, &leaf, &block).await;
    drop(storage);

    for entry in std::fs::read_dir(db.blob_dir().unwrap().join("payload")).unwrap() {
        std::fs::remove_file(entry.unwrap().path()).unwrap();
    }
    let storage = connect(&db).await;

    let id = BlockId::<MockTypes>::Number(1);
    let mut tx = storage.read().await.unwrap();
    assert!(matches!(
        tx.get_block(id).await.unwrap_err(),
        QueryError::Missing
    ));
    assert!(matches!(
        tx.get_payload(id).await.unwrap_err(),
        QueryError::Missing
    ));
    drop(tx);
    assert_eq!(counter(&storage, "blob_missing"), 2);

    let mut tx = storage.write().await.unwrap();
    tx.insert_block(&block).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = storage.read().await.unwrap();
    assert_eq!(tx.get_block(id).await.unwrap(), block);
}
