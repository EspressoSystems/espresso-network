use std::future::Future;

use anyhow::Result;
use espresso_types::{v0_3::StakeTableEvent, NamespaceId, SeqTypes};
use hotshot_query_service::{
    availability::{LeafId, LeafQueryData},
    node::BlockId,
};
use hotshot_types::data::EpochNumber;
use surf_disco::Url;
use vbs::version::StaticVersion;

use crate::{
    consensus::{
        header::HeaderProof, leaf::LeafProof, namespace::NamespaceProof, payload::PayloadProof,
    },
    storage::LeafRequest,
};

/// Interface to a query server providing the light client API.
pub trait Client: Send + Sync + 'static {
    /// Get a finality proof for the requested leaf.
    ///
    /// Optionally, the client may specify the height of a known-finalized leaf. In this case, the
    /// server _may_ terminate the proof in a leaf chain ending at this height, rather than a QC
    /// chain.
    fn leaf_proof(
        &self,
        id: impl Into<LeafRequest> + Send,
        finalized: Option<u64>,
    ) -> impl Send + Future<Output = Result<LeafProof>>;

    /// Get an inclusion proof for the requested header relative to the Merkle tree at height `root`.
    fn header_proof(
        &self,
        root: u64,
        id: BlockId<SeqTypes>,
    ) -> impl Send + Future<Output = Result<HeaderProof>>;

    /// Get all leaves in the given range `[start, end)`.
    fn get_leaves_in_range(
        &self,
        start: usize,
        end: usize,
    ) -> impl Send
           + Future<
        Output = Result<Vec<hotshot_query_service::availability::LeafQueryData<SeqTypes>>>,
    >;

    /// Get a proof for the requested payload.
    ///
    /// This method accepts only a `height`, not the more flexible [`BlockId`] type, because a
    /// [`Header`](espresso_types::Header) is needed to verify the resulting proof, so the height
    /// must already be known anyways.
    fn payload_proof(&self, height: u64) -> impl Send + Future<Output = Result<PayloadProof>>;

    /// Get a proof for the requested namespace.
    ///
    /// This method accepts only a `height`, not the more flexible [`BlockId`] type, because a
    /// [`Header`](espresso_types::Header) is needed to verify the resulting proof, so the height
    /// must already be known anyways.
    fn namespace_proof(
        &self,
        height: u64,
        namespace: NamespaceId,
    ) -> impl Send + Future<Output = Result<NamespaceProof>>;

    /// Get stake table events for the given epoch.
    ///
    /// This returns the list of events that must be applied to transform the stake table from
    /// `epoch - 1` into the stake table for `epoch`.
    fn stake_table_events(
        &self,
        epoch: EpochNumber,
    ) -> impl Send + Future<Output = Result<Vec<StakeTableEvent>>>;
}

/// A [`Client`] connected to the HotShot query service.
#[derive(Clone, Debug)]
pub struct QueryServiceClient {
    client: surf_disco::Client<hotshot_query_service::Error, StaticVersion<0, 1>>,
}

impl QueryServiceClient {
    /// Connect to a HotShot query service at the given base URL.
    pub fn new(url: Url) -> Self {
        Self {
            client: surf_disco::Client::new(url),
        }
    }
}

impl Client for QueryServiceClient {
    async fn leaf_proof(
        &self,
        id: impl Into<LeafRequest> + Send,
        finalized: Option<u64>,
    ) -> Result<LeafProof> {
        let path = "/light-client/leaf";
        let path = match id.into() {
            LeafRequest::Leaf(LeafId::Number(n)) | LeafRequest::Header(BlockId::Number(n)) => {
                format!("{path}/{n}")
            },
            LeafRequest::Leaf(LeafId::Hash(h)) => format!("{path}/hash/{h}"),
            LeafRequest::Header(BlockId::Hash(h)) => format!("{path}/block-hash/{h}"),
            LeafRequest::Header(BlockId::PayloadHash(h)) => format!("{path}/payload-hash/{h}"),
        };
        let path = match finalized {
            Some(finalized) => format!("{path}/{finalized}"),
            None => path,
        };
        let proof = self.client.get(&path).send().await?;
        Ok(proof)
    }

    /// Get all leaves in the given range `[start, end)`.
    async fn get_leaves_in_range(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let path = format!("/availability/leaf/{start}/{end}");
        let leaves = self.client.get(&path).send().await?;
        Ok(leaves)
    }

    async fn header_proof(&self, root: u64, id: BlockId<SeqTypes>) -> Result<HeaderProof> {
        let path = format!("/light-client/header/{root}/{}", fmt_block_id(id));
        let proof = self.client.get(&path).send().await?;
        Ok(proof)
    }

    async fn payload_proof(&self, height: u64) -> Result<PayloadProof> {
        let path = format!("/light-client/payload/{height}");
        Ok(self.client.get(&path).send().await?)
    }

    async fn namespace_proof(&self, height: u64, namespace: NamespaceId) -> Result<NamespaceProof> {
        Ok(self
            .client
            .get(&format!("/light-client/namespace/{height}/{namespace}"))
            .send()
            .await?)
    }

    async fn stake_table_events(&self, epoch: EpochNumber) -> Result<Vec<StakeTableEvent>> {
        Ok(self
            .client
            .get(&format!("/light-client/stake-table/{epoch}"))
            .send()
            .await?)
    }
}

fn fmt_block_id(id: BlockId<SeqTypes>) -> String {
    match id {
        BlockId::Number(n) => format!("{n}"),
        BlockId::Hash(h) => format!("hash/{h}"),
        BlockId::PayloadHash(h) => format!("payload-hash/{h}"),
    }
}

#[cfg(test)]
mod test {
    use espresso_types::{EpochVersion, SequencerVersions, Transaction};
    use futures::{stream::StreamExt, TryStreamExt};
    use hotshot_query_service::availability::{BlockQueryData, LeafQueryData};
    use portpicker::pick_unused_port;
    use pretty_assertions::assert_eq;
    use sequencer::{
        api::{
            data_source::testing::TestableSequencerDataSource,
            sql::DataSource,
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
            Options,
        },
        testing::{wait_for_decide_on_handle, TestConfigBuilder},
    };

    use super::*;
    use crate::{
        consensus::leaf::{FinalityProof, LeafProofHint},
        testing::AlwaysTrueQuorum,
    };

    #[tokio::test]
    #[test_log::test]
    async fn test_leaf_proof() {
        let port = pick_unused_port().expect("No ports free");
        let url: Url = format!("http://localhost:{port}").parse().unwrap();

        let test_config = TestConfigBuilder::default().build();
        let storage = DataSource::create_storage().await;
        let persistence =
            <DataSource as TestableSequencerDataSource>::persistence_options(&storage);

        let config = TestNetworkConfigBuilder::<1, _, _>::with_num_nodes()
            .api_config(
                DataSource::options(&storage, Options::with_port(port))
                    .light_client(Default::default()),
            )
            .persistences([persistence])
            .network_config(test_config)
            .build();

        let _network = TestNetwork::new(
            config,
            SequencerVersions::<EpochVersion, EpochVersion>::new(),
        )
        .await;
        let client = QueryServiceClient::new(url);

        // Wait for a chain of leaves to be produced.
        let leaves: Vec<LeafQueryData<SeqTypes>> = client
            .client
            .socket("availability/stream/leaves/1")
            .subscribe()
            .await
            .unwrap()
            .take(2)
            .try_collect()
            .await
            .unwrap();

        // Get leaf proof by height.
        let proof = client.leaf_proof(LeafId::Number(1), Some(2)).await.unwrap();
        assert!(matches!(proof.proof(), FinalityProof::Assumption));
        assert_eq!(
            proof
                .verify(LeafProofHint::assumption(leaves[1].leaf()))
                .await
                .unwrap(),
            leaves[0]
        );

        // Get the same proof by various other IDs.
        for req in [
            LeafRequest::Header(BlockId::Number(1)),
            LeafRequest::Leaf(LeafId::Hash(leaves[0].hash())),
            LeafRequest::Header(BlockId::Hash(leaves[0].block_hash())),
        ] {
            tracing::info!(?req, "get proof by alternative ID");
            let proof = client.leaf_proof(req, None).await.unwrap();
            assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. }));
            assert_eq!(
                proof
                    .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
                    .await
                    .unwrap(),
                leaves[0]
            );
        }

        // Get a proof by payload hash (this doesn't necessarily return a unique leaf, since
        // multiple) leaves may have the same payload.
        let proof = client
            .leaf_proof(BlockId::PayloadHash(leaves[0].payload_hash()), None)
            .await
            .unwrap();
        assert_eq!(
            proof
                .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
                .await
                .unwrap()
                .payload_hash(),
            leaves[0].payload_hash()
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_header_proof() {
        let port = pick_unused_port().expect("No ports free");
        let url: Url = format!("http://localhost:{port}").parse().unwrap();

        let test_config = TestConfigBuilder::default().build();
        let storage = DataSource::create_storage().await;
        let persistence =
            <DataSource as TestableSequencerDataSource>::persistence_options(&storage);

        let config = TestNetworkConfigBuilder::<1, _, _>::with_num_nodes()
            .api_config(
                DataSource::options(&storage, Options::with_port(port))
                    .light_client(Default::default()),
            )
            .persistences([persistence])
            .network_config(test_config)
            .build();

        let _network = TestNetwork::new(
            config,
            SequencerVersions::<EpochVersion, EpochVersion>::new(),
        )
        .await;
        let client = QueryServiceClient::new(url);

        // Wait for a chain of leaves to be produced.
        let leaves: Vec<LeafQueryData<SeqTypes>> = client
            .client
            .socket("availability/stream/leaves/1")
            .subscribe()
            .await
            .unwrap()
            .take(2)
            .try_collect()
            .await
            .unwrap();

        // Get header proof by height.
        let proof = client.header_proof(2, BlockId::Number(1)).await.unwrap();
        assert_eq!(
            proof
                .verify(leaves[1].header().block_merkle_tree_root())
                .unwrap(),
            *leaves[0].header()
        );

        // Get the same proof by block hash.
        let proof = client
            .header_proof(2, BlockId::Hash(leaves[0].block_hash()))
            .await
            .unwrap();
        assert_eq!(
            proof
                .verify(leaves[1].header().block_merkle_tree_root())
                .unwrap(),
            *leaves[0].header()
        );

        // Get a proof by payload hash (this doesn't necessarily return a unique header, since
        // multiple headers may have the same payload).
        let proof = client
            .header_proof(2, BlockId::PayloadHash(leaves[0].payload_hash()))
            .await
            .unwrap();
        assert_eq!(
            proof
                .verify(leaves[1].header().block_merkle_tree_root())
                .unwrap()
                .payload_commitment(),
            leaves[0].payload_hash()
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_payload_proof() {
        let port = pick_unused_port().expect("No ports free");
        let url: Url = format!("http://localhost:{port}").parse().unwrap();

        let test_config = TestConfigBuilder::default().build();
        let storage = DataSource::create_storage().await;
        let persistence =
            <DataSource as TestableSequencerDataSource>::persistence_options(&storage);

        let config = TestNetworkConfigBuilder::<1, _, _>::with_num_nodes()
            .api_config(
                DataSource::options(&storage, Options::with_port(port))
                    .light_client(Default::default()),
            )
            .persistences([persistence])
            .network_config(test_config)
            .build();

        let _network = TestNetwork::new(
            config,
            SequencerVersions::<EpochVersion, EpochVersion>::new(),
        )
        .await;
        let client = QueryServiceClient::new(url);

        // Wait for a block to be produced.
        let block: BlockQueryData<SeqTypes> = client
            .client
            .socket("availability/stream/blocks/1")
            .subscribe()
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();

        let proof = client.payload_proof(1).await.unwrap();
        assert_eq!(proof.verify(block.header()).unwrap(), *block.payload());

        // The block will be empty, but check at least that the namespace proof method hits the
        // correct endpoint and deserializes correctly.
        let ns = NamespaceId::from(0u64);
        let ns_proof = client.namespace_proof(1, ns).await.unwrap();
        assert_eq!(ns_proof.verify(block.header(), ns).unwrap(), vec![]);
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_namespace_proof() {
        let port = pick_unused_port().expect("No ports free");
        let url: Url = format!("http://localhost:{port}").parse().unwrap();

        let test_config = TestConfigBuilder::default().build();
        let storage = DataSource::create_storage().await;
        let persistence =
            <DataSource as TestableSequencerDataSource>::persistence_options(&storage);

        let config = TestNetworkConfigBuilder::<1, _, _>::with_num_nodes()
            .api_config(
                DataSource::options(&storage, Options::with_port(port))
                    .light_client(Default::default()),
            )
            .persistences([persistence])
            .network_config(test_config)
            .build();

        let network = TestNetwork::new(
            config,
            SequencerVersions::<EpochVersion, EpochVersion>::new(),
        )
        .await;

        // Submit a transaction to form a non-empty block.
        let mut events = network.server.event_stream().await;
        let tx = Transaction::random(&mut rand::thread_rng());
        network
            .server
            .consensus()
            .read()
            .await
            .submit_transaction(tx.clone())
            .await
            .unwrap();
        let block = wait_for_decide_on_handle(&mut events, &tx).await.0;

        let client = QueryServiceClient::new(url);
        let header = client
            .client
            .get(&format!("availability/header/{block}"))
            .send()
            .await
            .unwrap();
        let proof = client.namespace_proof(block, tx.namespace()).await.unwrap();
        assert_eq!(proof.verify(&header, tx.namespace()).unwrap(), [tx]);
    }
}
