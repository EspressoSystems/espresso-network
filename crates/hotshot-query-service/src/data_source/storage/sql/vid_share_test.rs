//! VID share write and read path through the share lane.

use committable::Committable;
use hotshot_example_types::node_types::TEST_VERSIONS;
use hotshot_types::{
    data::{VidCommon, VidShare},
    vid::advz::advz_scheme,
};
use jf_advz::VidScheme;

use super::{testing::TmpDb, *};
use crate::{
    availability::{LeafQueryData, VidCommonQueryData},
    data_source::storage::{NodeStorage, UpdateAvailabilityStorage},
    node::BlockId,
    testing::mocks::MockTypes,
};

type Leaf = LeafQueryData<MockTypes>;

async fn connect(db: &TmpDb) -> SqlStorage {
    SqlStorage::connect(db.config(), StorageConnectionType::Query)
        .await
        .unwrap()
}

/// A leaf at `height` with VID common and the first share of a dispersal of `payload`.
async fn vid_at(height: u64, payload: &[u8]) -> (Leaf, VidCommonQueryData<MockTypes>, VidShare) {
    let mut leaf =
        Leaf::genesis(&Default::default(), &Default::default(), TEST_VERSIONS.test).await;
    leaf.leaf.block_header_mut().block_number = height;
    let disperse = advz_scheme(2).disperse(payload).unwrap();
    let common = VidCommonQueryData::new(leaf.header().clone(), VidCommon::V0(disperse.common));
    let share = VidShare::V0(disperse.shares[0].clone());
    (leaf, common, share)
}

async fn store(
    storage: &SqlStorage,
    (leaf, common, share): &(Leaf, VidCommonQueryData<MockTypes>, VidShare),
) {
    let mut tx = storage.write().await.unwrap();
    tx.insert_leaf(leaf).await.unwrap();
    tx.insert_vid(common, Some(share)).await.unwrap();
    tx.commit().await.unwrap();
}

async fn share(storage: &SqlStorage, id: BlockId<MockTypes>) -> QueryResult<VidShare> {
    let mut tx = storage.read().await.unwrap();
    NodeStorage::<MockTypes>::vid_share(&mut tx, id).await
}

async fn inline_shares(storage: &SqlStorage) -> i64 {
    let mut tx = storage.read().await.unwrap();
    let (count,) = query_as::<(i64,)>("SELECT count(*) FROM header WHERE vid_share IS NOT NULL")
        .fetch_one(tx.as_mut())
        .await
        .unwrap();
    count
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_share_reads_by_height_hash_and_payload_hash() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let first = vid_at(1, &[1, 2, 3]).await;
    let second = vid_at(2, &[4, 5, 6]).await;
    assert_ne!(first.2, second.2);
    store(&storage, &first).await;
    store(&storage, &second).await;

    // The share lane holds the shares, not the header rows.
    assert_eq!(inline_shares(&storage).await, 0);
    assert_eq!(share(&storage, BlockId::Number(1)).await.unwrap(), first.2);
    assert_eq!(share(&storage, BlockId::Number(2)).await.unwrap(), second.2);
    for (leaf, _, expected) in [&first, &second] {
        let id = BlockId::Hash(leaf.header().commit());
        assert_eq!(share(&storage, id).await.unwrap(), *expected);
    }
    // Both blocks have the genesis payload hash. The lowest height wins.
    let id = BlockId::PayloadHash(first.0.payload_hash());
    assert_eq!(share(&storage, id).await.unwrap(), first.2);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_share_survives_restart() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let first = vid_at(1, &[1, 2, 3]).await;
    let second = vid_at(2, &[4, 5, 6]).await;
    store(&storage, &first).await;
    store(&storage, &second).await;
    drop(storage);

    let storage = connect(&db).await;
    assert_eq!(share(&storage, BlockId::Number(1)).await.unwrap(), first.2);
    let id = BlockId::Hash(second.0.header().commit());
    assert_eq!(share(&storage, id).await.unwrap(), second.2);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_lost_share_is_missing() {
    let db = TmpDb::init().await.with_blobs();
    let storage = connect(&db).await;
    let vid = vid_at(1, &[1, 2, 3]).await;
    store(&storage, &vid).await;
    drop(storage);

    // A crash before the 1 s sync loses the unsynced share records.
    for entry in std::fs::read_dir(db.blob_dir().unwrap().join("share")).unwrap() {
        std::fs::remove_file(entry.unwrap().path()).unwrap();
    }
    let storage = connect(&db).await;
    for id in [BlockId::Number(1), BlockId::Hash(vid.0.header().commit())] {
        assert!(matches!(
            share(&storage, id).await.unwrap_err(),
            QueryError::Missing
        ));
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_share_is_inline_without_blobs() {
    let db = TmpDb::init().await;
    let storage = connect(&db).await;
    let vid = vid_at(1, &[1, 2, 3]).await;
    store(&storage, &vid).await;

    assert_eq!(inline_shares(&storage).await, 1);
    assert_eq!(share(&storage, BlockId::Number(1)).await.unwrap(), vid.2);
}
