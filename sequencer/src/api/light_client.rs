use std::{sync::Arc, time::Duration};

use anyhow::Result;
use espresso_types::SeqTypes;
use futures::{future::FutureExt, stream::StreamExt};
use hotshot_query_service::{
    availability::AvailabilityDataSource,
    data_source::{storage::NodeStorage, VersionedDataSource},
    Error,
};
use light_client::core::leaf::LeafProof;
use tide_disco::{method::ReadState, Api, StatusCode};
use vbs::version::StaticVersionType;

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
            .ok_or_else(|| Error::Custom {
                message: "missing leaves".into(),
                status: StatusCode::NOT_FOUND,
            })?;

        if proof.push(leaf) {
            return Ok(proof);
        }
    }

    // We reached the end of the range of interest without encountering a 3-chain. Thus, if the last
    // leaf in the chain is not already assumed finalized by the client, we must prove it finalized
    // by appending two more QCs.
    if finalized.is_none() {
        let Some([committing_qc, deciding_qc]) = qc_chain else {
            return Err(Error::Custom {
                message: "missing QC 2-chain to prove finality".into(),
                status: StatusCode::NOT_FOUND,
            });
        };
        proof.add_qc_chain(Arc::new(committing_qc), Arc::new(deciding_qc));
    }

    Ok(proof)
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
    S::State: AvailabilityDataSource<SeqTypes> + VersionedDataSource,
    for<'a> <S::State as VersionedDataSource>::ReadOnly<'a>: NodeStorage<SeqTypes>,
{
    let toml = toml::from_str::<toml::Value>(include_str!("../../api/light-client.toml"))?;
    let mut api = Api::<S, Error, ApiVer>::new(toml)?;
    api.with_version(api_ver);

    let fetch_timeout = opt.fetch_timeout;

    api.get("leaf", move |req, state| {
        async move {
            let requested = req.integer_param("number").map_err(|err| Error::Custom {
                message: err.to_string(),
                status: StatusCode::BAD_REQUEST,
            })?;
            let finalized = req
                .opt_integer_param("finalized")
                .map_err(|err| Error::Custom {
                    message: err.to_string(),
                    status: StatusCode::BAD_REQUEST,
                })?;
            get_leaf_proof(state, requested, finalized, fetch_timeout).await
        }
        .boxed()
    })?;

    Ok(api)
}

#[cfg(test)]
mod test {
    use espresso_types::EpochVersion;
    use hotshot_query_service::data_source::{storage::UpdateAvailabilityStorage, Transaction};
    use light_client::{
        core::leaf::FinalityProof,
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
        let leaves = leaf_chain::<EpochVersion>(1..=2).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.insert_leaf(leaves[1].clone()).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Ask for the first leaf; it is proved finalized by the chain formed along with the second.
        let proof = get_leaf_proof(&ds, 1, None, Duration::MAX).await.unwrap();
        assert_eq!(
            proof.verify(&AlwaysTrueQuorum, None).await.unwrap(),
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
                .verify(&AlwaysTrueQuorum, Some(leaves[1].leaf()))
                .await
                .unwrap(),
            leaves[0]
        );
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
        let leaves = leaf_chain::<EpochVersion>(1..=3).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.insert_leaf(leaves[2].clone()).await.unwrap();
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

        // Insert a single leaf, plus an extra QC proving it finalized.
        let leaves = leaf_chain::<EpochVersion>(1..=2).await;
        let qcs = [leaves[0].qc().clone(), leaves[1].qc().clone()];
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf_with_qc_chain(leaves[0].clone(), Some(qcs.clone()))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        let proof = get_leaf_proof(&ds, 1, None, Duration::MAX).await.unwrap();
        assert_eq!(
            proof.verify(&AlwaysTrueQuorum, None).await.unwrap(),
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
        let leaves = leaf_chain_with_upgrade::<EnableEpochs>(1..=3, 2).await;
        assert_eq!(leaves[0].header().version(), LegacyVersion::version());
        assert_eq!(leaves[1].header().version(), EpochVersion::version());
        let qcs = [leaves[1].qc().clone(), leaves[2].qc().clone()];
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
                .verify(
                    &VersionCheckQuorum::new(leaves.iter().map(|leaf| leaf.leaf().clone())),
                    None
                )
                .await
                .unwrap(),
            leaves[0]
        );
        assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. }))
    }
}
