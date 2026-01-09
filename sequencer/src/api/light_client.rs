use std::{fmt::Display, sync::Arc, time::Duration};

use anyhow::Result;
use espresso_types::{BlockMerkleTree, SeqTypes};
use futures::{future::FutureExt, stream::StreamExt};
use hotshot_query_service::{
    availability::{AvailabilityDataSource, LeafId},
    data_source::{storage::NodeStorage, VersionedDataSource},
    merklized_state::{MerklizedStateDataSource, Snapshot},
    node::BlockId,
    types::HeightIndexed,
    Error,
};
use hotshot_types::utils::{epoch_from_block_number, root_block_in_epoch};
use jf_merkle_tree_compat::MerkleTreeScheme;
use light_client::consensus::{header::HeaderProof, leaf::LeafProof};
use tide_disco::{method::ReadState, Api, RequestParams, StatusCode};
use vbs::version::StaticVersionType;

use crate::api::data_source::{NodeStateDataSource, StakeTableDataSource};

async fn get_leaf_proof<State>(
    state: &State,
    requested: usize,
    finalized: Option<usize>,
    fetch_timeout: Duration,
) -> Result<LeafProof, Error>
where
    State: AvailabilityDataSource<SeqTypes> + VersionedDataSource,
    for<'a> State::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let (endpoint, qc_chain) = match finalized {
        Some(finalized) => {
            // If we have a known-finalized block, we will not need a final 2-chain of QCs to prove
            // the last leaf in the result finalized, since we will either terminate with a 3-chain
            // of leaves or at the `finalized` leaf. Thus, we can use None for the final QC chain.
            if finalized <= requested {
                return Err(Error::Custom {
                    message: format!(
                        "finalized leaf height ({finalized}) must be greater than requested \
                         ({requested})"
                    ),
                    status: StatusCode::BAD_REQUEST,
                });
            }
            (finalized, None)
        },
        None => {
            async {
                // Grab the endpoint and the final QC chain in the same transaction, to ensure that
                // the QC chain actually corresponds to the endpoint block (and is not subject to
                // concurrent updates).
                let mut tx = state.read().await?;
                let height = NodeStorage::block_height(&mut tx).await?;
                let qc_chain = tx.latest_qc_chain().await?;
                Ok((height, qc_chain))
            }
            .await
            .map_err(|err: anyhow::Error| Error::Custom {
                message: err.to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })?
        },
    };
    let mut leaves = state.get_leaf_range(requested..endpoint).await;
    let mut proof = LeafProof::default();

    while let Some(leaf) = leaves.next().await {
        let leaf = leaf
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found("missing leaves"))?;

        if proof.push(leaf) {
            return Ok(proof);
        }
    }

    // We reached the end of the range of interest without encountering a 3-chain. Thus, if the last
    // leaf in the chain is not already assumed finalized by the client, we must prove it finalized
    // by appending two more QCs.
    if finalized.is_none() {
        let Some([committing_qc, deciding_qc]) = qc_chain else {
            return Err(not_found("missing QC 2-chain to prove finality"));
        };
        proof.add_qc_chain(Arc::new(committing_qc), Arc::new(deciding_qc));
    }

    Ok(proof)
}

async fn get_header_proof<State>(
    state: &State,
    root: u64,
    requested: BlockId<SeqTypes>,
    fetch_timeout: Duration,
) -> Result<HeaderProof, Error>
where
    State: AvailabilityDataSource<SeqTypes>
        + MerklizedStateDataSource<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>
        + VersionedDataSource,
{
    let header = state
        .get_header(requested)
        .await
        .with_timeout(fetch_timeout)
        .await
        .ok_or_else(|| not_found(format!("unknown header {requested}")))?;
    if header.height() >= root {
        return Err(Error::Custom {
            message: format!(
                "height ({}) must be less than root ({root})",
                header.height()
            ),
            status: StatusCode::BAD_REQUEST,
        });
    }
    let path = MerklizedStateDataSource::<SeqTypes, BlockMerkleTree, _>::get_path(
        state,
        Snapshot::Index(root),
        header.height(),
    )
    .await
    .map_err(|source| Error::MerklizedState {
        source: source.into(),
    })?;

    Ok(HeaderProof::new(header, path))
}

#[derive(Debug)]
pub(super) struct Options {
    /// Timeout for failing requests due to missing data.
    ///
    /// If data needed to respond to a request is missing, it can (in some cases) be fetched from an
    /// external provider. This parameter controls how long the request handler will wait for
    /// missing data to be fetched before giving up and failing the request.
    pub fetch_timeout: Duration,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            fetch_timeout: Duration::from_millis(500),
        }
    }
}

pub(super) fn define_api<S, ApiVer: StaticVersionType + 'static>(
    opt: Options,
    api_ver: semver::Version,
) -> Result<Api<S, Error, ApiVer>>
where
    S: ReadState + Send + Sync + 'static,
    S::State: AvailabilityDataSource<SeqTypes>
        + MerklizedStateDataSource<SeqTypes, BlockMerkleTree, { BlockMerkleTree::ARITY }>
        + NodeStateDataSource
        + StakeTableDataSource<SeqTypes>
        + VersionedDataSource,
    for<'a> <S::State as VersionedDataSource>::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/light-client.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;
    api.with_version(api_ver);

    let fetch_timeout = opt.fetch_timeout;

    api.get("leaf", move |req, state| {
        async move {
            let requested = leaf_height_from_req(&req, state, fetch_timeout).await?;
            let finalized = req
                .opt_integer_param("finalized")
                .map_err(bad_param("finalized"))?;
            get_leaf_proof(state, requested, finalized, fetch_timeout).await
        }
        .boxed()
    })?
    .get("header", move |req, state| {
        async move {
            let root = req.integer_param("root").map_err(bad_param("root"))?;
            let requested = if let Some(height) = req
                .opt_integer_param("height")
                .map_err(bad_param("height"))?
            {
                BlockId::Number(height)
            } else if let Some(hash) = req.opt_blob_param("hash").map_err(bad_param("hash"))? {
                BlockId::Hash(hash)
            } else if let Some(hash) = req
                .opt_blob_param("payload-hash")
                .map_err(bad_param("payload-hash"))?
            {
                BlockId::PayloadHash(hash)
            } else {
                return Err(Error::Custom {
                    message: "missing parameter: requested header must be identified by height, \
                              hash, or payload hash"
                        .into(),
                    status: StatusCode::BAD_REQUEST,
                });
            };
            get_header_proof(state, root, requested, fetch_timeout).await
        }
        .boxed()
    })?
    .get("stake_table", move |req, state| {
        async move {
            let epoch: u64 = req.integer_param("epoch").map_err(bad_param("epoch"))?;

            let node_state = state.node_state().await;
            let epoch_height = node_state.epoch_height.ok_or_else(|| Error::Custom {
                message: "epoch state not set".into(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })?;
            let first_epoch = epoch_from_block_number(node_state.epoch_start_block, epoch_height);

            if epoch < first_epoch + 2 {
                return Err(Error::Custom {
                    message: format!("epoch must be at least {}", first_epoch + 2),
                    status: StatusCode::BAD_REQUEST,
                });
            }

            // Find the range of L1 block containing events for this epoch. This is determined by
            // the `l1_finalized` field of the epoch root (from two epochs prior) and the previous
            // epoch's epoch root.
            let epoch_root_height = root_block_in_epoch(epoch - 2, epoch_height) as usize;
            let epoch_root = state
                .get_header(epoch_root_height)
                .await
                .with_timeout(fetch_timeout)
                .await
                .ok_or_else(|| {
                    not_found(format!("missing epoch root header {epoch_root_height}"))
                })?;
            let to_l1_block = epoch_root
                .l1_finalized()
                .ok_or_else(|| Error::Custom {
                    message: "epoch root header is missing L1 finalized block".into(),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                })?
                .number();

            let from_l1_block = if epoch >= first_epoch + 3 {
                let prev_epoch_root_height = root_block_in_epoch(epoch - 3, epoch_height) as usize;
                let prev_epoch_root = state
                    .get_header(prev_epoch_root_height)
                    .await
                    .with_timeout(fetch_timeout)
                    .await
                    .ok_or_else(|| {
                        not_found(format!(
                            "missing previous epoch root header {prev_epoch_root_height}"
                        ))
                    })?;
                prev_epoch_root
                    .l1_finalized()
                    .ok_or_else(|| Error::Custom {
                        message: "previous epoch root header is missing L1 finalized block".into(),
                        status: StatusCode::INTERNAL_SERVER_ERROR,
                    })?
                    .number()
                    + 1
            } else {
                0
            };

            state
                .stake_table_events(from_l1_block, to_l1_block)
                .await
                .map_err(|err| Error::Custom {
                    message: format!("failed to load stake table events: {err:#}"),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                })
        }
        .boxed()
    })?;

    Ok(api)
}

async fn leaf_height_from_req<S>(
    req: &RequestParams,
    state: &S,
    fetch_timeout: Duration,
) -> Result<usize, Error>
where
    S: AvailabilityDataSource<SeqTypes>,
{
    if let Some(height) = req
        .opt_integer_param("height")
        .map_err(bad_param("height"))?
    {
        return Ok(height);
    } else if let Some(hash) = req.opt_blob_param("hash").map_err(bad_param("hash"))? {
        let leaf = state
            .get_leaf(LeafId::Hash(hash))
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found(format!("unknown leaf hash {hash}")))?;
        return Ok(leaf.height() as usize);
    } else if let Some(hash) = req
        .opt_blob_param("block-hash")
        .map_err(bad_param("block-hash"))?
    {
        let header = state
            .get_header(BlockId::Hash(hash))
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found(format!("unknown block hash {hash}")))?;
        return Ok(header.height() as usize);
    } else if let Some(hash) = req
        .opt_blob_param("payload-hash")
        .map_err(bad_param("payload-hash"))?
    {
        let header = state
            .get_header(BlockId::PayloadHash(hash))
            .await
            .with_timeout(fetch_timeout)
            .await
            .ok_or_else(|| not_found(format!("unknown payload hash {hash}")))?;
        return Ok(header.height() as usize);
    }

    Err(Error::Custom {
        message: "missing parameter: requested leaf must be identified by height, hash, block \
                  hash, or payload hash"
            .into(),
        status: StatusCode::BAD_REQUEST,
    })
}

fn bad_param<E>(name: &'static str) -> impl FnOnce(E) -> Error
where
    E: Display,
{
    move |err| Error::Custom {
        message: format!("{name}: {err:#}"),
        status: StatusCode::BAD_REQUEST,
    }
}

fn not_found(msg: impl Into<String>) -> Error {
    Error::Custom {
        message: msg.into(),
        status: StatusCode::NOT_FOUND,
    }
}

#[cfg(test)]
mod test {
    use espresso_types::{DrbAndHeaderUpgradeVersion, EpochVersion, BLOCK_MERKLE_TREE_HEIGHT};
    use hotshot_query_service::{
        data_source::{storage::UpdateAvailabilityStorage, Transaction},
        merklized_state::UpdateStateData,
    };
    use hotshot_types::simple_certificate::CertificatePair;
    use jf_merkle_tree_compat::{AppendableMerkleTreeScheme, ToTraversalPath};
    use light_client::{
        consensus::leaf::{FinalityProof, LeafProofHint},
        testing::{
            leaf_chain, leaf_chain_with_upgrade, AlwaysTrueQuorum, EnableEpochs, LegacyVersion,
            VersionCheckQuorum,
        },
    };
    use tide_disco::Error;

    use super::*;
    use crate::api::{
        data_source::{testing::TestableSequencerDataSource, SequencerDataSource},
        sql::DataSource,
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_two_chain() {
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = DataSource::create(
            DataSource::persistence_options(&storage),
            Default::default(),
            false,
        )
        .await
        .unwrap();

        // Insert some leaves, forming a chain.
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.insert_leaf(leaves[1].clone()).await.unwrap();
            tx.insert_leaf(leaves[2].clone()).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Ask for the first leaf; it is proved finalized by the chain formed along with the second.
        let proof = get_leaf_proof(&ds, 1, None, Duration::MAX).await.unwrap();
        assert_eq!(
            proof
                .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
                .await
                .unwrap(),
            leaves[0]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_finalized() {
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = DataSource::create(
            DataSource::persistence_options(&storage),
            Default::default(),
            false,
        )
        .await
        .unwrap();

        // Insert a single leaf. We will not be able to provide proofs ending in a leaf chain, but
        // we can return a leaf if the leaf after it is already known to be finalized.
        let leaves = leaf_chain::<EpochVersion>(1..=2).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.commit().await.unwrap();
        }

        let proof = get_leaf_proof(&ds, 1, Some(2), Duration::MAX)
            .await
            .unwrap();
        assert_eq!(
            proof
                .verify(LeafProofHint::assumption(leaves[1].leaf()))
                .await
                .unwrap(),
            leaves[0]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_bad_finalized() {
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = DataSource::create(
            DataSource::persistence_options(&storage),
            Default::default(),
            false,
        )
        .await
        .unwrap();

        // Insert a single leaf. If we request this leaf but provide a finalized leaf which is
        // earlier, we should fail.
        let leaves = leaf_chain::<EpochVersion>(1..2).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.commit().await.unwrap();
        }

        let err = get_leaf_proof(&ds, 1, Some(0), Duration::MAX)
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_no_chain() {
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = DataSource::create(
            DataSource::persistence_options(&storage),
            Default::default(),
            false,
        )
        .await
        .unwrap();

        // Insert multiple leaves that don't chain. We will not be able to prove these are
        // finalized.
        let leaves = leaf_chain::<EpochVersion>(1..=4).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.insert_leaf(leaves[2].clone()).await.unwrap();
            tx.insert_leaf(leaves[3].clone()).await.unwrap();
            tx.commit().await.unwrap();
        }

        let err = get_leaf_proof(&ds, 1, None, Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Even if we start from a finalized leave that extends one of the leaves we do have (4,
        // extends 3) we fail to generate a proof because we can't generate a chain from the
        // requested leaf (1) to the finalized leaf (4), since leaf 2 is missing.
        let err = get_leaf_proof(&ds, 1, Some(4), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_final_qcs() {
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = DataSource::create(
            DataSource::persistence_options(&storage),
            Default::default(),
            false,
        )
        .await
        .unwrap();

        // Insert a single leaf, plus an extra QC chain proving it finalized.
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        let qcs = [
            CertificatePair::for_parent(leaves[1].leaf()),
            CertificatePair::for_parent(leaves[2].leaf()),
        ];
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf_with_qc_chain(leaves[0].clone(), Some(qcs.clone()))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        let proof = get_leaf_proof(&ds, 1, None, Duration::MAX).await.unwrap();
        assert_eq!(
            proof
                .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
                .await
                .unwrap(),
            leaves[0]
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_upgrade_to_epochs() {
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = DataSource::create(
            DataSource::persistence_options(&storage),
            Default::default(),
            false,
        )
        .await
        .unwrap();

        // Upgrade to epochs (and enabling HotStuff2) in the middle of a leaf chain, so that the
        // last leaf in the chain only requires 2 QCs to verify, even though at the start of the
        // chain we would have required 3.
        let leaves = leaf_chain_with_upgrade::<EnableEpochs>(1..=4, 2).await;
        assert_eq!(leaves[0].header().version(), LegacyVersion::version());
        assert_eq!(
            leaves[1].header().version(),
            DrbAndHeaderUpgradeVersion::version()
        );
        let qcs = [
            CertificatePair::for_parent(leaves[2].leaf()),
            CertificatePair::for_parent(leaves[3].leaf()),
        ];
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.insert_leaf_with_qc_chain(leaves[1].clone(), Some(qcs.clone()))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        let proof = get_leaf_proof(&ds, 1, None, Duration::MAX).await.unwrap();
        assert_eq!(
            proof
                .verify(LeafProofHint::Quorum(&VersionCheckQuorum::new(
                    leaves.iter().map(|leaf| leaf.leaf().clone())
                )))
                .await
                .unwrap(),
            leaves[0]
        );
        assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. }))
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_header_proof() {
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = DataSource::create(
            DataSource::persistence_options(&storage),
            Default::default(),
            false,
        )
        .await
        .unwrap();

        // Construct a chain of leaves, plus the corresponding block Merkle tree at each leaf.
        let leaves = leaf_chain::<EpochVersion>(0..=2).await;
        let mts = leaves
            .iter()
            .scan(
                BlockMerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT),
                |mt, leaf| {
                    assert_eq!(mt.commitment(), leaf.header().block_merkle_tree_root());
                    let item = mt.clone();
                    mt.push(leaf.block_hash()).unwrap();
                    Some(item)
                },
            )
            .collect::<Vec<_>>();

        // Save all those objects in the DB.
        {
            let mut tx = ds.write().await.unwrap();
            for (leaf, mt) in leaves.iter().zip(&mts) {
                tx.insert_leaf(leaf.clone()).await.unwrap();

                if leaf.height() > 0 {
                    let merkle_path = mt.lookup(leaf.height() - 1).expect_ok().unwrap().1;
                    UpdateStateData::<SeqTypes, BlockMerkleTree, _>::insert_merkle_nodes(
                        &mut tx,
                        merkle_path,
                        ToTraversalPath::<{ BlockMerkleTree::ARITY }>::to_traversal_path(
                            &(leaf.height() - 1),
                            BLOCK_MERKLE_TREE_HEIGHT,
                        ),
                        leaf.height(),
                    )
                    .await
                    .unwrap();
                    UpdateStateData::<SeqTypes, BlockMerkleTree, _>::set_last_state_height(
                        &mut tx,
                        leaf.height() as usize,
                    )
                    .await
                    .unwrap();
                }
            }
            tx.commit().await.unwrap();
        }

        // Test happy path.
        for (root, mt) in mts.iter().enumerate().skip(1) {
            for (height, leaf) in leaves.iter().enumerate().take(root) {
                tracing::info!(root, height, "test happy path");
                let proof =
                    get_header_proof(&ds, root as u64, BlockId::Number(height), Duration::MAX)
                        .await
                        .unwrap();
                assert_eq!(proof.verify_ref(mt.commitment()).unwrap(), leaf.header());
            }
        }

        // Test unknown leaf.
        let err = get_header_proof(&ds, 5, BlockId::Number(4), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        // Test height >= root.
        let err = get_header_proof(&ds, 1, BlockId::Number(1), Duration::MAX)
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    }
}
