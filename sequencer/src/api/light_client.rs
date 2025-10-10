use std::time::Duration;

use anyhow::{ensure, Context, Result};
use committable::Committable;
use espresso_types::{Leaf2, SeqTypes};
use futures::{future::FutureExt, stream::StreamExt};
use hotshot_query_service::{
    availability::{AvailabilityDataSource, LeafQueryData},
    data_source::{storage::NodeStorage, VersionedDataSource},
    Error,
};
use hotshot_types::simple_certificate::QuorumCertificate2;
use serde::{Deserialize, Serialize};
use tide_disco::{method::ReadState, Api, StatusCode};
use vbs::version::StaticVersionType;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct LeafProof {
    /// A chain of leaves from a requested leaf to a provably finalized leaf.
    ///
    /// The chain is in chronological order, so `leaves[0]` is the requested leaf and
    /// `leaves.last()` is a leaf which is known or can be proven to be finalized. The chain is
    /// joined by `parent_commitment`, so it can be validated by recomputing the commitment of each
    /// leaf and comparing to the parent commitment of the next.
    pub leaves: Vec<Leaf2>,

    /// A chain of quorum certificates proving finality for the last leaf in `leaves`.
    ///
    /// The requirements for checking finality of a leaf given a 2-chain of QCs are:
    /// * `qcs[0].data.leaf_commit == leaf.commit()`
    /// * `qcs[0].view_number == leaf.view_number()`
    /// * `qcs[1].view_number == qcs[0].view_number + 1`
    /// * Both QCs have a valid threshold signature given a stake table
    ///
    /// These QCs are provided only if they are necessary to prove the last leaf in `leaves`
    /// finalized. If the last leaf is the parent of a known finalized leaf (that is, its commitment
    /// is equal to the `parent_commitment` field of a leaf which is already known to be finalized)
    /// these QCs are omitted.
    pub qcs: Option<[QuorumCertificate2<SeqTypes>; 2]>,
}

impl LeafProof {
    /// Verify the proof.
    ///
    /// If successful, returns the leaf which is proven finalized.
    pub fn verify(&self, finalized: Option<&Leaf2>) -> Result<LeafQueryData<SeqTypes>> {
        let mut leaves = self.leaves.iter();
        let leaf = leaves.next().context("empty leaf chain")?;
        let mut opt_qc = None;

        // Verify chaining by recomputing hashes.
        let mut curr = leaf;
        for next in leaves {
            ensure!(Committable::commit(curr) == next.parent_commitment());
            curr = next;

            if opt_qc.is_none() {
                // Get the QC signing `leaf` from the justify QC of the subsequent leaf.
                opt_qc = Some(next.justify_qc().clone());
            }
        }

        // Check that the final leaf is actually finalized.
        let qc;
        if let Some(finalized) = finalized {
            ensure!(Committable::commit(curr) == finalized.parent_commitment());

            // If the final leaf is also the requested leaf, save the QC which proves it finalized.
            qc = opt_qc.unwrap_or_else(|| finalized.justify_qc().clone());
        } else {
            let qcs = self
                .qcs
                .as_ref()
                .context("no finalized leaf and no QC chain provided")?;
            ensure!(qcs[0].view_number == curr.view_number());
            ensure!(qcs[0].data.leaf_commit == Committable::commit(curr));
            ensure!(qcs[1].view_number == qcs[0].view_number + 1);
            // TODO check threshold signatures

            // If the final leaf is also the requested leaf, save the QC which proves it finalized.
            qc = opt_qc.unwrap_or_else(|| qcs[0].clone());
        }

        let info = LeafQueryData::new(leaf.clone(), qc)?;
        Ok(info)
    }

    /// Append a new leaf to the proof's chain.
    ///
    /// Returns `true` if and only if we have enough data to prove at least the first leaf in the
    /// chain finalized.
    fn push(&mut self, leaf: LeafQueryData<SeqTypes>) -> bool {
        // Check if the new leaf forms a 2-chain.
        if let Some(last) = self.leaves.last() {
            let justify_qc = leaf.leaf().justify_qc();
            let qc = leaf.qc();
            if qc.view_number == justify_qc.view_number + 1
                && justify_qc.data.leaf_commit == Committable::commit(last)
            {
                self.qcs = Some([justify_qc, qc.clone()]);
                return true;
            }
        }

        self.leaves.push(leaf.leaf().clone());
        false
    }
}

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
        let Some(qc_chain) = qc_chain else {
            return Err(Error::Custom {
                message: "missing QC 2-chain to prove finality".into(),
                status: StatusCode::NOT_FOUND,
            });
        };
        proof.qcs = Some(qc_chain);
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
    use committable::Committable;
    use espresso_types::{Leaf2, NodeState};
    use hotshot_example_types::node_types::TestVersions;
    use hotshot_query_service::data_source::{storage::UpdateAvailabilityStorage, Transaction};
    use hotshot_types::{
        data::{QuorumProposal2, QuorumProposalWrapper, ViewNumber},
        traits::node_implementation::ConsensusTime,
    };
    use tide_disco::Error;

    use super::*;
    use crate::api::{
        data_source::{testing::TestableSequencerDataSource, SequencerDataSource},
        sql::DataSource,
    };

    async fn leaf_chain(range: impl IntoIterator<Item = u64>) -> Vec<LeafQueryData<SeqTypes>> {
        let genesis_leaf: Leaf2 =
            Leaf2::genesis::<TestVersions>(&Default::default(), &NodeState::mock()).await;
        let mut qc =
            QuorumCertificate2::genesis::<TestVersions>(&Default::default(), &NodeState::mock())
                .await;
        let mut quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
            proposal: QuorumProposal2::<SeqTypes> {
                epoch: None,
                block_header: genesis_leaf.block_header().clone(),
                view_number: genesis_leaf.view_number(),
                justify_qc: qc.clone(),
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                state_cert: None,
            },
        };

        let mut leaves = vec![];
        for height in range {
            *quorum_proposal.proposal.block_header.height_mut() = height;
            quorum_proposal.proposal.view_number = ViewNumber::new(height);
            let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);

            qc.view_number = ViewNumber::new(height);
            qc.data.leaf_commit = Committable::commit(&leaf);

            leaves.push(LeafQueryData::new(leaf, qc.clone()).unwrap());
            quorum_proposal.proposal.justify_qc = qc.clone();
        }

        leaves
    }

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
        let leaves = leaf_chain(1..=2).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.insert_leaf(leaves[1].clone()).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Ask for the first leaf; it is proved finalized by the chain formed along with the second.
        let proof = get_leaf_proof(&ds, 1, None, Duration::MAX).await.unwrap();
        assert_eq!(proof.verify(None).unwrap(), leaves[0]);
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
        let leaves = leaf_chain(1..=2).await;
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf(leaves[0].clone()).await.unwrap();
            tx.commit().await.unwrap();
        }

        let proof = get_leaf_proof(&ds, 1, Some(2), Duration::MAX)
            .await
            .unwrap();
        assert_eq!(proof.verify(Some(leaves[1].leaf())).unwrap(), leaves[0]);
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
        let leaves = leaf_chain(1..=3).await;
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
        let leaves = leaf_chain(1..=2).await;
        let qcs = [leaves[0].qc().clone(), leaves[1].qc().clone()];
        {
            let mut tx = ds.write().await.unwrap();
            tx.insert_leaf_with_qc_chain(leaves[0].clone(), Some(qcs.clone()))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }

        let proof = get_leaf_proof(&ds, 1, None, Duration::MAX).await.unwrap();
        assert_eq!(proof.verify(None).unwrap(), leaves[0]);
    }
}
