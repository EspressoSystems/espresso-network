use std::future::Future;

use anyhow::Result;
use espresso_types::SeqTypes;
use hotshot_query_service::availability::LeafId;
use surf_disco::Url;
use vbs::version::StaticVersion;

use crate::consensus::leaf::LeafProof;

/// Interface to a query server providing the light client API.
pub trait Client: Send + Sync + 'static {
    /// Get a finality proof for the requested leaf.
    ///
    /// Optionally, the client may specify the height of a known-finalized leaf. In this case, the
    /// server _may_ terminate the proof in a leaf chain ending at this height, rather than a QC
    /// chain.
    fn leaf_proof(
        &self,
        id: LeafId<SeqTypes>,
        finalized: Option<u64>,
    ) -> impl Send + Future<Output = Result<LeafProof>>;
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
    async fn leaf_proof(&self, id: LeafId<SeqTypes>, finalized: Option<u64>) -> Result<LeafProof> {
        let path = "/light-client/leaf";
        let path = match id {
            LeafId::Number(n) => format!("{path}/{n}"),
            LeafId::Hash(h) => format!("{path}/hash/{h}"),
        };
        let path = match finalized {
            Some(finalized) => format!("{path}/{finalized}"),
            None => path,
        };
        let proof = self.client.get(&path).send().await?;
        Ok(proof)
    }
}

#[cfg(test)]
mod test {
    use espresso_types::{EpochVersion, SequencerVersions};
    use futures::{stream::StreamExt, TryStreamExt};
    use hotshot_query_service::availability::LeafQueryData;
    use portpicker::pick_unused_port;
    use sequencer::{
        api::{
            data_source::testing::TestableSequencerDataSource,
            sql::DataSource,
            test_helpers::{TestNetwork, TestNetworkConfigBuilder},
            Options,
        },
        testing::TestConfigBuilder,
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

        // Get the same proof by hash.
        let proof = client
            .leaf_proof(LeafId::Hash(leaves[0].hash()), None)
            .await
            .unwrap();
        assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. }));
        assert_eq!(
            proof
                .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
                .await
                .unwrap(),
            leaves[0]
        );
    }
}
