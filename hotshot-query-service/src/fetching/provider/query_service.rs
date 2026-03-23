// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use std::fmt::Debug;

use anyhow::{Context, ensure};
use async_trait::async_trait;
use committable::Committable;
use hotshot_types::{
    data::{VidCommitment, VidCommon, ns_table},
    traits::{EncodeBytes, node_implementation::NodeType},
    vid::{
        advz::{ADVZScheme, advz_scheme},
        avidm::{AvidMScheme, init_avidm_param},
        avidm_gf2::AvidmGf2Scheme,
    },
};
use jf_advz::VidScheme;
use surf_disco::{Client, Url};
use vbs::version::StaticVersionType;

use super::Provider;
use crate::{
    Error, Payload,
    availability::{BlockQueryData, LeafQueryData, VidCommonQueryData},
    fetching::request::{
        LeafRangeRequest, LeafRequest, PayloadRangeRequest, PayloadRequest, RangeRequest,
        VidCommonRangeRequest, VidCommonRequest,
    },
    types::HeightIndexed,
};

/// Data availability provider backed by another instance of this query service.
///
/// This fetcher implements the [`Provider`] interface by querying the REST API provided by another
/// instance of this query service to try and retrieve missing objects.
#[derive(Clone, Debug)]
pub struct QueryServiceProvider<Ver: StaticVersionType> {
    client: Client<Error, Ver>,
}

impl<Ver: StaticVersionType> QueryServiceProvider<Ver> {
    pub fn new(url: Url, _: Ver) -> Self {
        Self {
            client: Client::new(url),
        }
    }
}

impl<Ver: StaticVersionType> QueryServiceProvider<Ver> {
    pub async fn fetch_payload<Types: NodeType>(
        &self,
        req: PayloadRequest,
    ) -> anyhow::Result<Payload<Types>> {
        let req_hash = req.0;

        // Fetch the payload and the VID common data. We need the common data to recompute the VID
        // commitment, to ensure the payload we received is consistent with the commitment we
        // requested.
        let block = self
            .client
            .get::<BlockQueryData<Types>>(&format!("availability/block/payload-hash/{req_hash}"))
            .send()
            .await
            .context("fetching block")?;
        let common = self
            .client
            .get::<VidCommonQueryData<Types>>(&format!(
                "availability/vid/common/payload-hash/{req_hash}",
            ))
            .send()
            .await
            .context("fetching VID common")?;

        let comm =
            recompute_payload_commitment(&block, &common).context("computing VID commitment")?;
        ensure!(
            comm == req_hash,
            "VID commitment {comm} does not match request"
        );

        Ok(block.payload)
    }

    pub async fn fetch_payload_range<Types: NodeType>(
        &self,
        req: PayloadRangeRequest,
    ) -> anyhow::Result<Vec<Payload<Types>>> {
        let req = RangeRequest::from(req);

        // Fetch the payload and the VID common data. We need the common data to recompute the VID
        // commitment, to ensure the payloads we received are consistent with the expected
        // commitments.
        let blocks = self
            .client
            .get::<Vec<BlockQueryData<Types>>>(&format!(
                "availability/block/{}/{}",
                req.start, req.end
            ))
            .send()
            .await
            .context("fetching blocks")?;
        let common = self
            .client
            .get::<Vec<VidCommonQueryData<Types>>>(&format!(
                "availability/vid/common/{}/{}",
                req.start, req.end
            ))
            .send()
            .await
            .context("fetching VID common")?;

        ensure!(
            blocks.len() == (req.end - req.start) as usize,
            "wrong number of blocks ({})",
            blocks.len(),
        );
        ensure!(
            common.len() == (req.end - req.start) as usize,
            "wrong number of VID common ({})",
            common.len(),
        );

        let commits = blocks
            .iter()
            .zip(&common)
            .map(|(block, common)| recompute_payload_commitment(block, common))
            .collect::<Result<Vec<VidCommitment>, _>>()
            .context("computing VID commitments")?;
        let hash = RangeRequest::hash_payloads(commits.iter().copied());
        ensure!(
            hash == req.expected_hash,
            "server returned blocks with wrong payload hash ({hash}, commits {commits:?})",
        );

        Ok(blocks.into_iter().map(|block| block.payload).collect())
    }

    fn handle_result<R: Debug, T>(&self, req: R, res: anyhow::Result<T>) -> Option<T> {
        match res {
            Ok(res) => Some(res),
            Err(err) => {
                tracing::warn!(upstream = %self.client.base_url(), ?req, "failed to fetch: {err:#}");
                None
            },
        }
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, PayloadRequest> for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    async fn fetch(&self, req: PayloadRequest) -> Option<Payload<Types>> {
        self.handle_result(req, self.fetch_payload::<Types>(req).await)
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, PayloadRangeRequest>
    for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    async fn fetch(&self, req: PayloadRangeRequest) -> Option<Vec<Payload<Types>>> {
        self.handle_result(req, self.fetch_payload_range::<Types>(req).await)
    }
}

fn recompute_payload_commitment<Types>(
    block: &BlockQueryData<Types>,
    common: &VidCommonQueryData<Types>,
) -> anyhow::Result<VidCommitment>
where
    Types: NodeType,
{
    match common.common() {
        VidCommon::V0(common) => {
            let num_storage_nodes = ADVZScheme::get_num_storage_nodes(common) as usize;
            let bytes = block.payload().encode();
            advz_scheme(num_storage_nodes)
                .commit_only(bytes)
                .map(VidCommitment::V0)
                .context("failed to compute VID commitment (V0)")
        },
        VidCommon::V1(common) => {
            let bytes = block.payload().encode();
            let avidm_param = init_avidm_param(common.total_weights).context(format!(
                "failed to initialize AVIDM params. total_weight={}",
                common.total_weights
            ))?;
            let metadata = block.metadata().encode();
            AvidMScheme::commit(
                &avidm_param,
                &bytes,
                ns_table::parse_ns_table(bytes.len(), &metadata),
            )
            .map(VidCommitment::V1)
            .map_err(anyhow::Error::msg)
            .context("failed to compute AVIDM commitment")
        },
        VidCommon::V2(common) => {
            let bytes = block.payload().encode();
            let metadata = block.metadata().encode();
            AvidmGf2Scheme::commit(
                &common.param,
                &bytes,
                ns_table::parse_ns_table(bytes.len(), &metadata),
            )
            .map(|(commit, _)| VidCommitment::V2(commit))
            .map_err(anyhow::Error::msg)
            .context("failed to compute AvidmGf2 commitment")
        },
    }
}

impl<Ver: StaticVersionType> QueryServiceProvider<Ver> {
    pub async fn fetch_leaf<Types: NodeType>(
        &self,
        req: LeafRequest<Types>,
    ) -> anyhow::Result<LeafQueryData<Types>> {
        let mut leaf = self
            .client
            .get::<LeafQueryData<Types>>(&format!("availability/leaf/{}", req.height))
            .send()
            .await
            .context("fetching leaf")?;

        ensure!(
            leaf.height() == req.height,
            "received leaf with the wrong height ({})",
            leaf.height(),
        );
        ensure!(
            leaf.hash() == req.expected_leaf,
            "received leaf with the wrong hash ({})",
            leaf.hash()
        );
        ensure!(
            leaf.qc().commit() == req.expected_qc,
            "received leaf with the wrong QC ({})",
            leaf.qc().commit()
        );

        // There is a potential DOS attack where the peer sends us a leaf with the full
        // payload in it, which uses redundant resources in the database, since we fetch and
        // store payloads separately. We can defend ourselves by simply dropping the payload
        // if present.
        leaf.leaf.unfill_block_payload();

        Ok(leaf)
    }

    pub async fn fetch_leaf_range<Types: NodeType>(
        &self,
        req: LeafRangeRequest<Types>,
    ) -> anyhow::Result<Vec<LeafQueryData<Types>>> {
        let mut leaves = self
            .client
            .get::<Vec<LeafQueryData<Types>>>(&format!(
                "availability/leaf/{}/{}",
                req.start, req.end
            ))
            .send()
            .await
            .context("fetching leaf chain")?;

        ensure!(
            leaves.len() == (req.end - req.start) as usize,
            "server returned wrong number of leaves ({})",
            leaves.len()
        );

        // Verify hash chaining.
        let mut expected_leaf = req.last_leaf;
        let mut expected_qc = req.last_qc;
        for leaf in leaves.iter_mut().rev() {
            // There is a potential DOS attack where the peer sends us a leaf with the full payload
            // in it, which uses redundant resources in the database, since we fetch and store
            // payloads separately. We can defend ourselves by simply dropping the payload if
            // present.
            leaf.leaf.unfill_block_payload();

            let leaf_hash = leaf.hash();
            let qc_hash = leaf.qc().commit();
            ensure!(
                leaf_hash == expected_leaf,
                "received leaf {} with wrong hash {leaf_hash}",
                leaf.height(),
            );
            ensure!(
                qc_hash == expected_qc,
                "received leaf {} with wrong QC {qc_hash}",
                leaf.height()
            );
            expected_leaf = leaf.leaf().parent_commitment();
            expected_qc = leaf.leaf().justify_qc().commit();
        }

        Ok(leaves)
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, LeafRequest<Types>>
    for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    async fn fetch(&self, req: LeafRequest<Types>) -> Option<LeafQueryData<Types>> {
        self.handle_result(req, self.fetch_leaf(req).await)
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, LeafRangeRequest<Types>>
    for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    async fn fetch(&self, req: LeafRangeRequest<Types>) -> Option<Vec<LeafQueryData<Types>>> {
        self.handle_result(req, self.fetch_leaf_range(req).await)
    }
}

impl<Ver: StaticVersionType> QueryServiceProvider<Ver> {
    pub async fn fetch_vid_common<Types: NodeType>(
        &self,
        req: VidCommonRequest,
    ) -> anyhow::Result<VidCommon> {
        let res = self
            .client
            .get::<VidCommonQueryData<Types>>(&format!(
                "availability/vid/common/payload-hash/{}",
                req.0
            ))
            .send()
            .await
            .context("fetching VID common")?;

        ensure!(
            res.common().is_consistent(&req.0),
            "inconsistent VID common data {:?}",
            res.common,
        );
        Ok(res.common)
    }

    pub async fn fetch_vid_common_range<Types: NodeType>(
        &self,
        req: VidCommonRangeRequest,
    ) -> anyhow::Result<Vec<VidCommon>> {
        let req = RangeRequest::from(req);
        let common = self
            .client
            .get::<Vec<VidCommonQueryData<Types>>>(&format!(
                "availability/vid/common/{}/{}",
                req.start, req.end
            ))
            .send()
            .await
            .context("fetching VID common")?;

        ensure!(
            common.len() == (req.end - req.start) as usize,
            "server returned wrong number of VID common ({})",
            common.len()
        );

        let commits = common
            .iter()
            .map(|common| {
                // Check that the given `VidCommon` matches the claimed payload commitment.
                ensure!(
                    common.common().is_consistent(&common.payload_hash()),
                    "server returned VID common with inconsistent commitment {common:?}"
                );

                // Yield the payload commitment for hashing, to check that the full sequence of payloads
                // is consistent with the request.
                Ok(common.payload_hash())
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let hash = RangeRequest::hash_payloads(commits.iter().copied());
        ensure!(
            hash == req.expected_hash,
            "server returned wrong VID common (hash {hash}, commits {commits:?})"
        );

        Ok(common.into_iter().map(|common| common.common).collect())
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, VidCommonRequest> for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    async fn fetch(&self, req: VidCommonRequest) -> Option<VidCommon> {
        self.handle_result(req, self.fetch_vid_common::<Types>(req).await)
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, VidCommonRangeRequest>
    for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    async fn fetch(&self, req: VidCommonRangeRequest) -> Option<Vec<VidCommon>> {
        self.handle_result(req, self.fetch_vid_common_range::<Types>(req).await)
    }
}

// These tests run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(all(test, not(target_os = "windows")))]
mod test {
    use std::{future::IntoFuture, time::Duration};

    use committable::{Commitment, Committable};
    use futures::{
        future::{FutureExt, join},
        stream::StreamExt,
    };
    // generic-array 0.14.x is deprecated, but VidCommitment requires this version
    // for From<Output<H>> impl in jf-merkle-tree
    #[allow(deprecated)]
    use generic_array::GenericArray;
    use hotshot_example_types::node_types::{EpochVersion, TEST_VERSIONS};
    use rand::RngCore;
    use test_utils::reserve_tcp_port;
    use tide_disco::{App, error::ServerError};

    use super::*;
    use crate::{
        ApiState,
        api::load_api,
        availability::{
            AvailabilityDataSource, BlockId, BlockInfo, BlockQueryData, BlockWithTransaction,
            Fetch, UpdateAvailabilityData, define_api,
        },
        data_source::{
            AvailabilityProvider, FetchingDataSource, Transaction, VersionedDataSource,
            sql::{self, SqlDataSource},
            storage::{
                AvailabilityStorage, SqlStorage, StorageConnectionType, UpdateAvailabilityStorage,
                fail_storage::{FailStorage, FailableAction},
                pruning::{PrunedHeightStorage, PrunerCfg},
                sql::testing::TmpDb,
            },
        },
        fetching::provider::{NoFetching, Provider as ProviderTrait, TestProvider},
        node::{SyncStatusQueryData, data_source::NodeDataSource},
        task::BackgroundTask,
        testing::{
            consensus::{MockDataSource, MockNetwork},
            mocks::{MockBase, MockTypes, mock_transaction},
            sleep,
        },
        types::HeightIndexed,
    };

    type Provider = TestProvider<QueryServiceProvider<MockBase>>;
    type EpochProvider = TestProvider<QueryServiceProvider<EpochVersion>>;

    fn ignore<T>(_: T) {}

    /// Build a data source suitable for this suite of tests.
    async fn builder<P: AvailabilityProvider<MockTypes> + Clone>(
        db: &TmpDb,
        provider: &P,
    ) -> sql::Builder<MockTypes, P> {
        db.config()
            .builder((*provider).clone())
            .await
            .unwrap()
            // We disable proactive fetching for these tests, since we are intending to test on
            // demand fetching, and proactive fetching could lead to false successes.
            .disable_proactive_fetching()
    }

    /// A data source suitable for this suite of tests, with the default options.
    async fn data_source<P: AvailabilityProvider<MockTypes> + Clone>(
        db: &TmpDb,
        provider: &P,
    ) -> SqlDataSource<MockTypes, P> {
        builder(db, provider).await.build().await.unwrap()
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_on_request() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let data_source = data_source(&db, &provider).await;

        // Start consensus.
        network.start().await;

        // Wait until the block height reaches 6. This gives us the genesis block, one additional
        // block at the end, and then one block to play around with fetching each type of resource:
        // * Leaf
        // * Block
        // * Payload
        // * VID common
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;
        let test_leaf = &leaves[0];
        let test_block = &leaves[1];
        let test_payload = &leaves[2];
        let test_common = &leaves[3];

        // Make requests for missing data that should _not_ trigger an active fetch:
        tracing::info!("requesting unfetchable resources");
        let mut fetches = vec![];
        // * An unknown leaf hash.
        fetches.push(data_source.get_leaf(test_leaf.hash()).await.map(ignore));
        // * An unknown leaf height.
        fetches.push(
            data_source
                .get_leaf(test_leaf.height() as usize)
                .await
                .map(ignore),
        );
        // * An unknown block hash.
        fetches.push(
            data_source
                .get_block(test_block.block_hash())
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_payload(test_payload.block_hash())
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_vid_common(test_common.block_hash())
                .await
                .map(ignore),
        );
        // * An unknown block height.
        fetches.push(
            data_source
                .get_block(test_block.height() as usize)
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_payload(test_payload.height() as usize)
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_vid_common(test_common.height() as usize)
                .await
                .map(ignore),
        );
        // * Genesis VID common (no VID for genesis)
        fetches.push(data_source.get_vid_common(0).await.map(ignore));
        // * An unknown transaction.
        fetches.push(
            data_source
                .get_block_containing_transaction(mock_transaction(vec![]).commit())
                .await
                .map(ignore),
        );

        // Even if we give data extra time to propagate, these requests will not resolve, since we
        // didn't trigger any active fetches.
        sleep(Duration::from_secs(1)).await;
        for (i, fetch) in fetches.into_iter().enumerate() {
            tracing::info!("checking fetch {i} is unresolved");
            fetch.try_resolve().unwrap_err();
        }

        // Now we will actually fetch the missing data. First, since our node is not really
        // connected to consensus, we need to give it a leaf after the range of interest so it
        // learns about the correct block height. We will temporarily lock requests to the provider
        // so that we can verify that without the provider, the node does _not_ get the data.
        provider.block().await;
        data_source
            .append(leaves.last().cloned().unwrap().into())
            .await
            .unwrap();

        tracing::info!("requesting fetchable resources");
        let req_leaf = data_source.get_leaf(test_leaf.height() as usize).await;
        let req_block = data_source.get_block(test_block.height() as usize).await;
        let req_payload = data_source
            .get_payload(test_payload.height() as usize)
            .await;
        let req_common = data_source
            .get_vid_common(test_common.height() as usize)
            .await;

        // Give the requests some extra time to complete, and check that they still haven't
        // resolved, since the provider is blocked. This just ensures the integrity of the test by
        // checking the node didn't mysteriously get the block from somewhere else, so that when we
        // unblock the provider and the node finally gets the block, we know it came from the
        // provider.
        sleep(Duration::from_secs(1)).await;
        req_leaf.try_resolve().unwrap_err();
        req_block.try_resolve().unwrap_err();
        req_payload.try_resolve().unwrap_err();
        req_common.try_resolve().unwrap_err();

        // Unblock the request and see that we eventually receive the data.
        provider.unblock().await;
        let leaf = data_source
            .get_leaf(test_leaf.height() as usize)
            .await
            .await;
        let block = data_source
            .get_block(test_block.height() as usize)
            .await
            .await;
        let payload = data_source
            .get_payload(test_payload.height() as usize)
            .await
            .await;
        let common = data_source
            .get_vid_common(test_common.height() as usize)
            .await
            .await;
        {
            // Verify the data.
            let truth = network.data_source();
            assert_eq!(
                leaf,
                truth.get_leaf(test_leaf.height() as usize).await.await
            );
            assert_eq!(
                block,
                truth.get_block(test_block.height() as usize).await.await
            );
            assert_eq!(
                payload,
                truth
                    .get_payload(test_payload.height() as usize)
                    .await
                    .await
            );
            assert_eq!(
                common,
                truth
                    .get_vid_common(test_common.height() as usize)
                    .await
                    .await
            );
        }

        // Fetching the block and payload should have also fetched the corresponding leaves, since
        // we have an invariant that we should not store a block in the database without its
        // corresponding leaf and header. Thus we should be able to get the leaves even if the
        // provider is blocked.
        provider.block().await;
        for leaf in [test_block, test_payload] {
            tracing::info!("fetching existing leaf {}", leaf.height());
            let fetched_leaf = data_source.get_leaf(leaf.height() as usize).await.await;
            assert_eq!(*leaf, fetched_leaf);
        }

        // On the other hand, fetching the block corresponding to `leaf` _will_ trigger a fetch,
        // since fetching a leaf does not necessarily fetch the corresponding block. We can fetch by
        // hash now, since the presence of the corresponding leaf allows us to confirm that a block
        // with this hash exists, and trigger a fetch for it.
        tracing::info!("fetching block by hash");
        provider.unblock().await;
        {
            let block = data_source.get_block(test_leaf.block_hash()).await.await;
            assert_eq!(block.hash(), leaf.block_hash());
        }

        // Test a similar scenario, but with payload instead of block: we are aware of
        // `leaves.last()` but not the corresponding payload, but we can fetch that payload by block
        // hash.
        tracing::info!("fetching payload by hash");
        {
            let leaf = leaves.last().unwrap();
            let payload = data_source.get_payload(leaf.block_hash()).await.await;
            assert_eq!(payload.height(), leaf.height());
            assert_eq!(payload.block_hash(), leaf.block_hash());
            assert_eq!(payload.hash(), leaf.payload_hash());
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_on_request_epoch_version() {
        // This test verifies that our provider can handle fetching things by their hashes,
        // specifically focused on epoch version transitions
        tracing::info!("Starting test_fetch_on_request_epoch_version");

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                EpochVersion::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), EpochVersion::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        // Use our special test provider that handles epoch version transitions
        let db = TmpDb::init().await;
        let provider = EpochProvider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            EpochVersion::instance(),
        ));
        let data_source = data_source(&db, &provider).await;

        // Start consensus.
        network.start().await;

        // Wait until the block height reaches 6. This gives us the genesis block, one additional
        // block at the end, and then one block to play around with fetching each type of resource:
        // * Leaf
        // * Block
        // * Payload
        // * VID common
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;
        let test_leaf = &leaves[0];
        let test_block = &leaves[1];
        let test_payload = &leaves[2];
        let test_common = &leaves[3];

        // Make requests for missing data that should _not_ trigger an active fetch:
        let mut fetches = vec![];
        // * An unknown leaf hash.
        fetches.push(data_source.get_leaf(test_leaf.hash()).await.map(ignore));
        // * An unknown leaf height.
        fetches.push(
            data_source
                .get_leaf(test_leaf.height() as usize)
                .await
                .map(ignore),
        );
        // * An unknown block hash.
        fetches.push(
            data_source
                .get_block(test_block.block_hash())
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_payload(test_payload.block_hash())
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_vid_common(test_common.block_hash())
                .await
                .map(ignore),
        );
        // * An unknown block height.
        fetches.push(
            data_source
                .get_block(test_block.height() as usize)
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_payload(test_payload.height() as usize)
                .await
                .map(ignore),
        );
        fetches.push(
            data_source
                .get_vid_common(test_common.height() as usize)
                .await
                .map(ignore),
        );
        // * Genesis VID common (no VID for genesis)
        fetches.push(data_source.get_vid_common(0).await.map(ignore));
        // * An unknown transaction.
        fetches.push(
            data_source
                .get_block_containing_transaction(mock_transaction(vec![]).commit())
                .await
                .map(ignore),
        );

        // Even if we give data extra time to propagate, these requests will not resolve, since we
        // didn't trigger any active fetches.
        sleep(Duration::from_secs(1)).await;
        for (i, fetch) in fetches.into_iter().enumerate() {
            tracing::info!("checking fetch {i} is unresolved");
            fetch.try_resolve().unwrap_err();
        }

        // Now we will actually fetch the missing data. First, since our node is not really
        // connected to consensus, we need to give it a leaf after the range of interest so it
        // learns about the correct block height. We will temporarily lock requests to the provider
        // so that we can verify that without the provider, the node does _not_ get the data.
        provider.block().await;
        data_source
            .append(leaves.last().cloned().unwrap().into())
            .await
            .unwrap();

        let req_leaf = data_source.get_leaf(test_leaf.height() as usize).await;
        let req_block = data_source.get_block(test_block.height() as usize).await;
        let req_payload = data_source
            .get_payload(test_payload.height() as usize)
            .await;
        let req_common = data_source
            .get_vid_common(test_common.height() as usize)
            .await;

        // Give the requests some extra time to complete, and check that they still haven't
        // resolved, since the provider is blocked. This just ensures the integrity of the test by
        // checking the node didn't mysteriously get the block from somewhere else, so that when we
        // unblock the provider and the node finally gets the block, we know it came from the
        // provider.
        sleep(Duration::from_secs(1)).await;
        req_leaf.try_resolve().unwrap_err();
        req_block.try_resolve().unwrap_err();
        req_payload.try_resolve().unwrap_err();
        req_common.try_resolve().unwrap_err();

        // Unblock the request and see that we eventually receive the data.
        provider.unblock().await;
        let leaf = data_source
            .get_leaf(test_leaf.height() as usize)
            .await
            .await;
        let block = data_source
            .get_block(test_block.height() as usize)
            .await
            .await;
        let payload = data_source
            .get_payload(test_payload.height() as usize)
            .await
            .await;
        let common = data_source
            .get_vid_common(test_common.height() as usize)
            .await
            .await;
        {
            // Verify the data.
            let truth = network.data_source();
            assert_eq!(
                leaf,
                truth.get_leaf(test_leaf.height() as usize).await.await
            );
            assert_eq!(
                block,
                truth.get_block(test_block.height() as usize).await.await
            );
            assert_eq!(
                payload,
                truth
                    .get_payload(test_payload.height() as usize)
                    .await
                    .await
            );
            assert_eq!(
                common,
                truth
                    .get_vid_common(test_common.height() as usize)
                    .await
                    .await
            );
        }

        // Fetching the block and payload should have also fetched the corresponding leaves, since
        // we have an invariant that we should not store a block in the database without its
        // corresponding leaf and header. Thus we should be able to get the leaves even if the
        // provider is blocked.
        provider.block().await;
        for leaf in [test_block, test_payload] {
            tracing::info!("fetching existing leaf {}", leaf.height());
            let fetched_leaf = data_source.get_leaf(leaf.height() as usize).await.await;
            assert_eq!(*leaf, fetched_leaf);
        }

        // On the other hand, fetching the block corresponding to `leaf` _will_ trigger a fetch,
        // since fetching a leaf does not necessarily fetch the corresponding block. We can fetch by
        // hash now, since the presence of the corresponding leaf allows us to confirm that a block
        // with this hash exists, and trigger a fetch for it.
        provider.unblock().await;
        {
            let block = data_source.get_block(test_leaf.block_hash()).await.await;
            assert_eq!(block.hash(), leaf.block_hash());
        }

        // Test a similar scenario, but with payload instead of block: we are aware of
        // `leaves.last()` but not the corresponding payload, but we can fetch that payload by block
        // hash.
        {
            let leaf = leaves.last().unwrap();
            let payload = data_source.get_payload(leaf.block_hash()).await.await;
            assert_eq!(payload.height(), leaf.height());
            assert_eq!(payload.block_hash(), leaf.block_hash());
            assert_eq!(payload.hash(), leaf.payload_hash());
        }

        // Add more debug logs throughout the test
        tracing::info!("Test completed successfully!");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_block_and_leaf_concurrently() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let data_source = data_source(&db, &provider).await;

        // Start consensus.
        network.start().await;

        // Wait until the block height reaches 3. This gives us the genesis block, one additional
        // block at the end, and then one block that we can use to test fetching.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(2).collect::<Vec<_>>().await;
        let test_leaf = &leaves[0];

        // Tell the node about a leaf after the one of interest so it learns about the block height.
        data_source.append(leaves[1].clone().into()).await.unwrap();

        // Fetch a leaf and the corresponding block at the same time. This will result in two tasks
        // trying to fetch the same leaf, but one should win and notify the other, which ultimately
        // ends up not fetching anything.
        let (leaf, block) = join(
            data_source
                .get_leaf(test_leaf.height() as usize)
                .await
                .into_future(),
            data_source
                .get_block(test_leaf.height() as usize)
                .await
                .into_future(),
        )
        .await;
        assert_eq!(leaf, *test_leaf);
        assert_eq!(leaf.header(), block.header());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_different_blocks_same_payload() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let data_source = data_source(&db, &provider).await;

        // Start consensus.
        network.start().await;

        // Wait until the block height reaches 4. This gives us the genesis block, one additional
        // block at the end, and then two blocks that we can use to test fetching.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(4).collect::<Vec<_>>().await;

        // Tell the node about a leaf after the range of interest so it learns about the block
        // height.
        data_source
            .append(leaves.last().cloned().unwrap().into())
            .await
            .unwrap();

        // All the blocks here are empty, so they have the same payload:
        assert_eq!(leaves[0].payload_hash(), leaves[1].payload_hash());
        // If we fetch both blocks at the same time, we can check that we haven't broken anything
        // with whatever optimizations we add to deduplicate payload fetching.
        let (block1, block2) = join(
            data_source
                .get_block(leaves[0].height() as usize)
                .await
                .into_future(),
            data_source
                .get_block(leaves[1].height() as usize)
                .await
                .into_future(),
        )
        .await;
        assert_eq!(block1.header(), leaves[0].header());
        assert_eq!(block2.header(), leaves[1].header());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_stream() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let data_source = data_source(&db, &provider).await;

        // Start consensus.
        network.start().await;

        // Subscribe to objects from the future.
        let blocks = data_source.subscribe_blocks(0).await;
        let leaves = data_source.subscribe_leaves(0).await;
        let common = data_source.subscribe_vid_common(0).await;

        // Wait for a few blocks to be finalized.
        let finalized_leaves = network.data_source().subscribe_leaves(0).await;
        let finalized_leaves = finalized_leaves.take(5).collect::<Vec<_>>().await;

        // Tell the node about a leaf after the range of interest so it learns about the block
        // height.
        data_source
            .append(finalized_leaves.last().cloned().unwrap().into())
            .await
            .unwrap();

        // Check the subscriptions.
        let blocks = blocks.take(5).collect::<Vec<_>>().await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;
        let common = common.take(5).collect::<Vec<_>>().await;
        for i in 0..5 {
            tracing::info!("checking block {i}");
            assert_eq!(leaves[i], finalized_leaves[i]);
            assert_eq!(blocks[i].header(), finalized_leaves[i].header());
            assert_eq!(common[i], data_source.get_vid_common(i).await.await);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_range_start() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let data_source = data_source(&db, &provider).await;

        // Start consensus.
        network.start().await;

        // Wait for a few blocks to be finalized.
        let finalized_leaves = network.data_source().subscribe_leaves(0).await;
        let finalized_leaves = finalized_leaves.take(5).collect::<Vec<_>>().await;

        // Tell the node about a leaf after the range of interest (so it learns about the block
        // height) and one in the middle of the range, to test what happens when data is missing
        // from the beginning of the range but other data is available.
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(&finalized_leaves[2]).await.unwrap();
        tx.insert_leaf(&finalized_leaves[4]).await.unwrap();
        tx.commit().await.unwrap();

        // Get the whole range of leaves.
        let leaves = data_source
            .get_leaf_range(..5)
            .await
            .then(Fetch::resolve)
            .collect::<Vec<_>>()
            .await;
        for i in 0..5 {
            tracing::info!("checking leaf {i}");
            assert_eq!(leaves[i], finalized_leaves[i]);
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn fetch_transaction() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus. We don't give it a
        // fetcher since transactions are always fetched passively anyways.
        let db = TmpDb::init().await;
        let data_source = data_source(&db, &NoFetching).await;

        // Subscribe to blocks.
        let mut leaves = network.data_source().subscribe_leaves(1).await;
        let mut blocks = network.data_source().subscribe_blocks(1).await;

        // Start consensus.
        network.start().await;

        // Subscribe to a transaction which hasn't been sequenced yet. This is completely passive
        // and works without a fetcher; we don't trigger fetches for transactions that we don't know
        // exist.
        let tx = mock_transaction(vec![1, 2, 3]);
        let fut = data_source
            .get_block_containing_transaction(tx.commit())
            .await;

        // Sequence the transaction.
        network.submit_transaction(tx.clone()).await;

        // Send blocks to the query service, the future will resolve as soon as it sees a block
        // containing the transaction.
        let block = loop {
            let leaf = leaves.next().await.unwrap();
            let block = blocks.next().await.unwrap();

            data_source
                .append(BlockInfo::new(leaf, Some(block.clone()), None, None))
                .await
                .unwrap();

            if block.transaction_by_hash(tx.commit()).is_some() {
                break block;
            }
        };
        tracing::info!("transaction included in block {}", block.height());

        let fetched_tx = fut.await;
        assert_eq!(
            fetched_tx,
            BlockWithTransaction::with_hash(block, tx.commit()).unwrap()
        );

        // Future queries for this transaction resolve immediately.
        assert_eq!(
            fetched_tx,
            data_source
                .get_block_containing_transaction(tx.commit())
                .await
                .await
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_retry() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let data_source = builder(&db, &provider)
            .await
            .with_max_retry_interval(Duration::from_secs(1))
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until the block height reaches 3. This gives us the genesis block, one additional
        // block at the end, and one block to try fetching.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(2).collect::<Vec<_>>().await;
        let test_leaf = &leaves[0];

        // Cause requests to fail temporarily, so we can test retries.
        provider.fail();

        // Give the node a leaf after the range of interest so it learns about the correct block
        // height.
        data_source
            .append(leaves.last().cloned().unwrap().into())
            .await
            .unwrap();

        tracing::info!("requesting leaf from failing providers");
        let fut = data_source.get_leaf(test_leaf.height() as usize).await;

        // Wait a few retries and check that the request has not completed, since the provider is
        // failing.
        sleep(Duration::from_secs(5)).await;
        fut.try_resolve().unwrap_err();

        // As soon as the provider recovers, the request can complete.
        provider.unfail();
        assert_eq!(
            data_source
                .get_leaf(test_leaf.height() as usize)
                .await
                .await,
            *test_leaf
        );
    }

    // Uses deprecated generic-array 0.14.x via digest, required for VidCommitment compatibility
    #[allow(deprecated)]
    fn random_vid_commit() -> VidCommitment {
        let mut bytes = [0; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        VidCommitment::V0(GenericArray::from(bytes).into())
    }

    fn random_leaf_request() -> LeafRequest<MockTypes> {
        let mut bytes = [0; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        LeafRequest {
            height: 1,
            expected_leaf: Commitment::from_raw(bytes),
            expected_qc: Commitment::from_raw(bytes),
        }
    }

    async fn malicious_server(port: u16) {
        let mut api = load_api::<(), ServerError, MockBase>(
            None::<std::path::PathBuf>,
            include_str!("../../../api/availability.toml"),
            vec![],
        )
        .unwrap();

        api.get("get_leaf", move |req, _| {
            async move {
                let height = req.integer_param("height")?;

                // Respond with a leaf of the correct height, but with a dummy hash.
                let mut leaf = LeafQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test,
                )
                .await;
                leaf.leaf.block_header_mut().block_number = height;
                Ok(leaf)
            }
            .boxed()
        })
        .unwrap()
        .get("get_leaf_range", move |req, _| {
            async move {
                let start = req.integer_param("from")?;
                let end = req.integer_param("until")?;

                // Respond with the correct range, but using leaves with dummy data.
                let leaf = LeafQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test,
                )
                .await;
                let leaves = (start..end)
                    .map(|i| {
                        let mut leaf = leaf.clone();
                        leaf.leaf.block_header_mut().block_number = i;
                        leaf
                    })
                    .collect::<Vec<_>>();

                Ok(leaves)
            }
            .boxed()
        })
        .unwrap()
        .get("get_block", move |_, _| {
            async move {
                // No matter what data we are asked for, always respond with dummy data.
                Ok(BlockQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test.base,
                )
                .await)
            }
            .boxed()
        })
        .unwrap()
        .get("get_block_range", move |req, _| {
            async move {
                let start = req.integer_param("from")?;
                let end = req.integer_param("until")?;

                // Respond with the correct range, but using blocks with dummy data.
                let block = BlockQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test.base,
                )
                .await;
                let blocks = (start..end)
                    .map(|i| {
                        let mut block = block.clone();
                        block.header.block_number = i;
                        block
                    })
                    .collect::<Vec<_>>();

                Ok(blocks)
            }
            .boxed()
        })
        .unwrap()
        .get("get_vid_common", move |_, _| {
            async move {
                // No matter what data we are asked for, always respond with dummy data.
                Ok(VidCommonQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test.base,
                )
                .await)
            }
            .boxed()
        })
        .unwrap()
        .get("get_vid_common_range", move |req, _| {
            async move {
                let start = req.integer_param("from")?;
                let end = req.integer_param("until")?;

                // Respond with the correct range, but using VID with dummy data.
                let vid = VidCommonQueryData::<MockTypes>::genesis(
                    &Default::default(),
                    &Default::default(),
                    TEST_VERSIONS.test.base,
                )
                .await;
                let vids = (start..end)
                    .map(|i| {
                        let mut vid = vid.clone();
                        vid.height = i;
                        vid
                    })
                    .collect::<Vec<_>>();

                Ok(vids)
            }
            .boxed()
        })
        .unwrap();

        let mut app = App::<(), ServerError>::with_state(());
        app.register_module("availability", api).unwrap();
        app.serve(format!("0.0.0.0:{port}"), MockBase::instance())
            .await
            .ok();
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_from_malicious_server() {
        let port = reserve_tcp_port().unwrap();
        let _server = BackgroundTask::spawn("malicious server", malicious_server(port));

        let provider = QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        );
        provider.client.connect(None).await;

        // Query for a random leaf, the server will respond with a different leaf, and we should
        // detect the error.
        tracing::info!("fetch leaf");
        let err = provider
            .fetch_leaf(random_leaf_request())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("wrong hash"), "{err:#}");

        // Ranged leaf request.
        tracing::info!("fetch leaf range");
        let req = random_leaf_request();
        let err = provider
            .fetch_leaf_range(LeafRangeRequest {
                start: 0,
                end: req.height + 1,
                last_leaf: req.expected_leaf,
                last_qc: req.expected_qc,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("wrong hash"), "{err:#}");

        // Query for a random payload, the server will respond with a different payload, and we
        // should detect the error.
        tracing::info!("fetch payload");
        let err = provider
            .fetch_payload::<MockTypes>(PayloadRequest(random_vid_commit()))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("does not match request"),
            "{err:#}"
        );

        // Payload range request.
        tracing::info!("fetch payload range");
        let err = provider
            .fetch_payload_range::<MockTypes>(PayloadRangeRequest::from(RangeRequest {
                start: 0,
                end: 2,
                expected_hash: RangeRequest::hash_payloads([
                    random_vid_commit(),
                    random_vid_commit(),
                ]),
            }))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("wrong payload hash"), "{err:#}");

        // Query for a random VID common, the server will respond with a different one, and we
        // should detect the error.
        tracing::info!("fetch VID");
        let err = provider
            .fetch_vid_common::<MockTypes>(VidCommonRequest(random_vid_commit()))
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("inconsistent VID common"),
            "{err:#}"
        );

        // VID range request.
        tracing::info!("fetch VID range");
        let err = provider
            .fetch_vid_common_range::<MockTypes>(VidCommonRangeRequest::from(RangeRequest {
                start: 0,
                end: 2,
                expected_hash: RangeRequest::hash_payloads([
                    random_vid_commit(),
                    random_vid_commit(),
                ]),
            }))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("wrong VID common"), "{err:#}");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_archive_recovery() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer. The
        // data source is at first configured to aggressively prune data.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let mut data_source = db
            .config()
            .pruner_cfg(
                PrunerCfg::new()
                    .with_target_retention(Duration::from_secs(0))
                    .with_interval(Duration::from_secs(5)),
            )
            .unwrap()
            .builder(provider.clone())
            .await
            .unwrap()
            // Set a fast retry for failed operations. Occasionally storage operations will fail due
            // to conflicting write-mode transactions running concurrently. This is ok as they will
            // be retried. Having a fast retry interval speeds up the test.
            .with_min_retry_interval(Duration::from_millis(100))
            // Randomize retries a lot. This will temporarlly separate competing transactions write
            // transactions with high probability, so that one of them quickly gets exclusive access
            // to the database.
            .with_retry_randomization_factor(3.)
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a few blocks are produced.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;

        // The disconnected data source has no data yet, so it hasn't done any pruning.
        let pruned_height = data_source
            .read()
            .await
            .unwrap()
            .load_pruned_height()
            .await
            .unwrap();
        // Either None or 0 is acceptable, depending on whether or not the prover has run yet.
        assert!(matches!(pruned_height, None | Some(0)), "{pruned_height:?}");

        // Send the last leaf to the disconnected data source so it learns about the height and
        // fetches the missing data.
        let last_leaf = leaves.last().unwrap();
        data_source.append(last_leaf.clone().into()).await.unwrap();

        // Trigger a fetch of each leaf so the database gets populated.
        for i in 1..=last_leaf.height() {
            tracing::info!(i, "fetching leaf");
            assert_eq!(
                data_source.get_leaf(i as usize).await.await,
                leaves[i as usize - 1]
            );
        }

        // After a bit of time, the pruner has run and deleted all the missing data we just fetched.
        loop {
            let pruned_height = data_source
                .read()
                .await
                .unwrap()
                .load_pruned_height()
                .await
                .unwrap();
            if pruned_height == Some(last_leaf.height()) {
                break;
            }
            tracing::info!(
                ?pruned_height,
                target_height = last_leaf.height(),
                "waiting for pruner to run"
            );
            sleep(Duration::from_secs(1)).await;
        }

        // Now close the data source and restart it with archive recovery.
        data_source = db
            .config()
            .archive()
            .builder(provider.clone())
            .await
            .unwrap()
            .with_proactive_interval(Duration::from_secs(1))
            .build()
            .await
            .unwrap();

        // Pruned height should be reset.
        let pruned_height = data_source
            .read()
            .await
            .unwrap()
            .load_pruned_height()
            .await
            .unwrap();
        assert_eq!(pruned_height, None);

        // The node has pruned all of it's data including the latest block, so it's forgotten the
        // block height. We need to give it another leaf with some height so it will be willing to
        // fetch.
        data_source.append(last_leaf.clone().into()).await.unwrap();

        // Wait for the data to be restored. It should be restored by the next major scan.
        loop {
            let sync_status = data_source.sync_status().await.unwrap();

            // VID shares are unique to a node and will never be fetched from a peer; this is
            // acceptable since there is redundancy built into the VID scheme. Ignore missing VID
            // shares in the `is_fully_synced` check.
            if (SyncStatusQueryData {
                vid_shares: Default::default(),
                ..sync_status.clone()
            })
            .is_fully_synced()
            {
                break;
            }
            tracing::info!(?sync_status, "waiting for node to sync");
            sleep(Duration::from_secs(1)).await;
        }

        // The node remains fully synced even after some time; no pruning.
        sleep(Duration::from_secs(3)).await;
        let sync_status = data_source.sync_status().await.unwrap();
        assert!(
            (SyncStatusQueryData {
                vid_shares: Default::default(),
                ..sync_status.clone()
            })
            .is_fully_synced(),
            "{sync_status:#?}"
        );
    }

    #[derive(Clone, Copy, Debug)]
    enum FailureType {
        Begin,
        Write,
        Commit,
    }

    async fn test_fetch_storage_failure_helper(failure: FailureType) {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_max_retry_interval(Duration::from_millis(100))
            .with_retry_timeout(Duration::from_secs(1))
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a couple of blocks are produced.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(2).collect::<Vec<_>>().await;

        // Send the last leaf to the disconnected data source so it learns about the height.
        let last_leaf = leaves.last().unwrap();
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(last_leaf).await.unwrap();
        tx.commit().await.unwrap();

        // Trigger a fetch of the first leaf; it should resolve even if we fail to store the leaf.
        tracing::info!("fetch with write failure");
        match failure {
            FailureType::Begin => {
                data_source
                    .as_ref()
                    .fail_begins_writable(FailableAction::Any)
                    .await
            },
            FailureType::Write => data_source.as_ref().fail_writes(FailableAction::Any).await,
            FailureType::Commit => data_source.as_ref().fail_commits(FailableAction::Any).await,
        }
        assert_eq!(leaves[0], data_source.get_leaf(1).await.await);
        data_source.as_ref().pass().await;

        // It is possible that the fetch above completes, notifies the subscriber,
        // and the fetch below quickly subscribes and gets notified by the same loop.
        // We add a delay here to avoid this situation.
        // This is not a bug, as being notified after subscribing is fine.
        sleep(Duration::from_secs(1)).await;

        // We can get the same leaf again, this will again trigger an active fetch since storage
        // failed the first time.
        tracing::info!("fetch with write success");
        let fetch = data_source.get_leaf(1).await;
        assert!(fetch.is_pending());
        assert_eq!(leaves[0], fetch.await);

        sleep(Duration::from_secs(1)).await;

        // Finally, we should have the leaf locally and not need to fetch it.
        tracing::info!("retrieve from storage");
        let fetch = data_source.get_leaf(1).await;
        assert_eq!(leaves[0], fetch.try_resolve().ok().unwrap());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_storage_failure_on_begin() {
        test_fetch_storage_failure_helper(FailureType::Begin).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_storage_failure_on_write() {
        test_fetch_storage_failure_helper(FailureType::Write).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_storage_failure_on_commit() {
        test_fetch_storage_failure_helper(FailureType::Commit).await;
    }

    async fn test_fetch_storage_failure_retry_helper(failure: FailureType) {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_min_retry_interval(Duration::from_millis(100))
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a couple of blocks are produced.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(2).collect::<Vec<_>>().await;

        // Send the last leaf to the disconnected data source so it learns about the height.
        let last_leaf = leaves.last().unwrap();
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(last_leaf).await.unwrap();
        tx.commit().await.unwrap();

        // Trigger a fetch of the first leaf; it should retry until it successfully stores the leaf.
        tracing::info!("fetch with write failure");
        match failure {
            FailureType::Begin => {
                data_source
                    .as_ref()
                    .fail_one_begin_writable(FailableAction::Any)
                    .await
            },
            FailureType::Write => {
                data_source
                    .as_ref()
                    .fail_one_write(FailableAction::Any)
                    .await
            },
            FailureType::Commit => {
                data_source
                    .as_ref()
                    .fail_one_commit(FailableAction::Any)
                    .await
            },
        }
        assert_eq!(leaves[0], data_source.get_leaf(1).await.await);

        // Check that the leaf ended up in local storage.
        let mut tx = data_source.read().await.unwrap();
        assert_eq!(leaves[0], tx.get_leaf(1.into()).await.unwrap());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_storage_failure_retry_on_begin() {
        test_fetch_storage_failure_retry_helper(FailureType::Begin).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_storage_failure_retry_on_write() {
        test_fetch_storage_failure_retry_helper(FailureType::Write).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_storage_failure_retry_on_commit() {
        test_fetch_storage_failure_retry_helper(FailureType::Commit).await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_on_decide() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus.
        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let data_source = builder(&db, &provider)
            .await
            .with_max_retry_interval(Duration::from_secs(1))
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a block has been decided.
        let leaf = network
            .data_source()
            .subscribe_leaves(1)
            .await
            .next()
            .await
            .unwrap();

        // Give the node a decide containing the leaf but no additional information.
        tracing::info!("send decide event");
        data_source.append(leaf.clone().into()).await.unwrap();

        // We will eventually retrieve the corresponding block and VID common, triggered by seeing
        // the leaf.
        tracing::info!("wait");
        sleep(Duration::from_secs(5)).await;

        // Read the missing data directly from storage (via a database transaction), rather than
        // going through the data source, so that this request itself does not trigger a fetch.
        // Thus, this will only work if the data was already fetched, triggered by the leaf.
        let mut tx = data_source.read().await.unwrap();
        let id = BlockId::<MockTypes>::from(leaf.height() as usize);
        let block = tx.get_block(id).await.unwrap();
        let vid = tx.get_vid_common(id).await.unwrap();

        assert_eq!(block.hash(), leaf.block_hash());
        assert_eq!(vid.block_hash(), leaf.block_hash());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_begin_failure() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_min_retry_interval(Duration::from_millis(100))
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a couple of blocks are produced.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(2).collect::<Vec<_>>().await;

        // Send the last leaf to the disconnected data source so it learns about the height.
        let last_leaf = leaves.last().unwrap();
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(last_leaf).await.unwrap();
        tx.commit().await.unwrap();

        // Trigger a fetch of the first leaf; it should retry until it is able to determine
        // the leaf is fetchable and trigger the fetch.
        tracing::info!("fetch with transaction failure");
        data_source
            .as_ref()
            .fail_one_begin_read_only(FailableAction::Any)
            .await;
        assert_eq!(leaves[0], data_source.get_leaf(1).await.await);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_load_failure_block() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_min_retry_interval(Duration::from_millis(100))
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a block is produced.
        let mut leaves = network.data_source().subscribe_leaves(1).await;
        let leaf = leaves.next().await.unwrap();

        // Send the leaf to the disconnected data source, so the corresponding block becomes
        // fetchable.
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(&leaf).await.unwrap();
        tx.commit().await.unwrap();

        // Trigger a fetch of the block by hash; it should retry until it is able to determine the
        // leaf is available, thus the block is fetchable, trigger the fetch.
        //
        // Failing only on the `get_header` call here hits an edge case which is only possible when
        // fetching blocks: we successfully determine that the object is not available locally and
        // that it might exist, so we actually call `active_fetch` to try and get it. If we then
        // fail to load the header and erroneously treat this as the header not being available, we
        // will give up and consider the object unfetchable (since the next step would be to fetch
        // the corresponding leaf, but we cannot do this with just a block hash).
        //
        // Thus, this test will only pass if we correctly retry the `active_fetch` until we are
        // successfully able to load the header from storage and determine that the block is
        // fetchable.
        tracing::info!("fetch with read failure");
        data_source
            .as_ref()
            .fail_one_read(FailableAction::GetHeader)
            .await;
        let fetch = data_source.get_block(leaf.block_hash()).await;

        // Give some time for a few reads to fail before letting them succeed.
        sleep(Duration::from_secs(2)).await;
        data_source.as_ref().pass().await;

        let block: BlockQueryData<MockTypes> = fetch.await;
        assert_eq!(block.hash(), leaf.block_hash());
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fetch_load_failure_tx() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_min_retry_interval(Duration::from_millis(100))
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a transaction is sequenced.
        let tx = mock_transaction(vec![1, 2, 3]);
        network.submit_transaction(tx.clone()).await;
        let tx = network
            .data_source()
            .get_block_containing_transaction(tx.commit())
            .await
            .await;

        // Send the block containing the transaction to the disconnected data source.
        {
            let leaf = network
                .data_source()
                .get_leaf(tx.transaction.block_height() as usize)
                .await
                .await;
            let block = network
                .data_source()
                .get_block(tx.transaction.block_height() as usize)
                .await
                .await;
            let mut tx = data_source.write().await.unwrap();
            tx.insert_leaf(&leaf).await.unwrap();
            tx.insert_block(&block).await.unwrap();
            tx.commit().await.unwrap();
        }

        // Check that the transaction is there.
        tracing::info!("fetch success");
        assert_eq!(
            tx,
            data_source
                .get_block_containing_transaction(tx.transaction.hash())
                .await
                .await
        );

        // Fetch the transaction with storage failures.
        //
        // Failing only one read here hits an edge case that only exists for unfetchable objects
        // (e.g. transactions). This will cause the initial aload of the transaction to fail, but,
        // if we erroneously treat this load failure as the transaction being missing, we will
        // succeed in calling `fetch`, since subsequent loads succeed. However, since a transaction
        // is not active-fetchable, no active fetch will actually be spawned, and this fetch will
        // never resolve.
        //
        // Thus, the test should only pass if we correctly retry the initial load until it succeeds
        // and we discover that the transaction doesn't need to be fetched at all.
        tracing::info!("fetch with read failure");
        data_source
            .as_ref()
            .fail_one_read(FailableAction::Any)
            .await;
        let fetch = data_source
            .get_block_containing_transaction(tx.transaction.hash())
            .await;

        assert_eq!(tx, fetch.await);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_stream_begin_failure() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_min_retry_interval(Duration::from_millis(100))
            .with_range_chunk_size(3)
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a few blocks are produced.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;

        // Send the last leaf to the disconnected data source so it learns about the height.
        let last_leaf = leaves.last().unwrap();
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(last_leaf).await.unwrap();
        tx.commit().await.unwrap();

        // Stream the leaves; it should retry until it is able to determine each leaf is fetchable
        // and trigger the fetch.
        tracing::info!("stream with transaction failure");
        data_source
            .as_ref()
            .fail_one_begin_read_only(FailableAction::Any)
            .await;
        assert_eq!(
            leaves,
            data_source
                .subscribe_leaves(1)
                .await
                .take(5)
                .collect::<Vec<_>>()
                .await
        );
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_stream_load_failure() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_min_retry_interval(Duration::from_millis(100))
            .with_range_chunk_size(3)
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a few blocks are produced.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;

        // Send the last leaf to the disconnected data source, so the blocks becomes fetchable.
        let last_leaf = leaves.last().unwrap();
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(last_leaf).await.unwrap();
        tx.commit().await.unwrap();

        // Stream the blocks with a period of database failures.
        tracing::info!("stream with read failure");
        data_source.as_ref().fail_reads(FailableAction::Any).await;
        let fetches = data_source
            .get_block_range(1..=5)
            .await
            .collect::<Vec<_>>()
            .await;

        // Give some time for a few reads to fail before letting them succeed.
        sleep(Duration::from_secs(2)).await;
        data_source.as_ref().pass().await;

        for (leaf, fetch) in leaves.iter().zip(fetches) {
            let block: BlockQueryData<MockTypes> = fetch.await;
            assert_eq!(block.hash(), leaf.block_hash());
        }
    }

    enum MetadataType {
        Payload,
        Vid,
    }

    async fn test_metadata_stream_begin_failure_helper(stream: MetadataType) {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        ));
        let db = TmpDb::init().await;
        let storage = FailStorage::from(
            SqlStorage::connect(db.config(), StorageConnectionType::Query)
                .await
                .unwrap(),
        );
        let data_source = FetchingDataSource::builder(storage, provider)
            .disable_proactive_fetching()
            .disable_aggregator()
            .with_min_retry_interval(Duration::from_millis(100))
            .with_range_chunk_size(3)
            .build()
            .await
            .unwrap();

        // Start consensus.
        network.start().await;

        // Wait until a few blocks are produced.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(3).collect::<Vec<_>>().await;

        // Send the last leaf to the disconnected data source, so the blocks becomes fetchable.
        let last_leaf = leaves.last().unwrap();
        let mut tx = data_source.write().await.unwrap();
        tx.insert_leaf(last_leaf).await.unwrap();
        tx.commit().await.unwrap();

        // Send the first object to the disconnected data source, so we hit all the cases:
        // * leaf present but not full object (from the last leaf)
        // * full object present but inaccessible due to storage failures (first object)
        // * nothing present (middle object)
        let leaf = network.data_source().get_leaf(1).await.await;
        let block = network.data_source().get_block(1).await.await;
        let vid = network.data_source().get_vid_common(1).await.await;
        data_source
            .append(BlockInfo::new(leaf, Some(block), Some(vid), None))
            .await
            .unwrap();

        // Stream the objects with a period of database failures.
        tracing::info!("stream with transaction failure");
        data_source
            .as_ref()
            .fail_begins_read_only(FailableAction::Any)
            .await;
        match stream {
            MetadataType::Payload => {
                let payloads = data_source.subscribe_payload_metadata(1).await.take(3);

                // Give some time for a few reads to fail before letting them succeed.
                sleep(Duration::from_secs(2)).await;
                tracing::info!("stop failing transactions");
                data_source.as_ref().pass().await;

                let payloads = payloads.collect::<Vec<_>>().await;
                for (leaf, payload) in leaves.iter().zip(payloads) {
                    assert_eq!(payload.block_hash, leaf.block_hash());
                }
            },
            MetadataType::Vid => {
                let vids = data_source.subscribe_vid_common_metadata(1).await.take(3);

                // Give some time for a few reads to fail before letting them succeed.
                sleep(Duration::from_secs(2)).await;
                tracing::info!("stop failing transactions");
                data_source.as_ref().pass().await;

                let vids = vids.collect::<Vec<_>>().await;
                for (leaf, vid) in leaves.iter().zip(vids) {
                    assert_eq!(vid.block_hash, leaf.block_hash());
                }
            },
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_metadata_stream_begin_failure_payload() {
        test_metadata_stream_begin_failure_helper(MetadataType::Payload).await
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_metadata_stream_begin_failure_vid() {
        test_metadata_stream_begin_failure_helper(MetadataType::Vid).await
    }

    #[tokio::test(flavor = "multi_thread")]
    #[test_log::test]
    async fn test_ranged_fetch() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = reserve_tcp_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                MockBase::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start consensus.
        network.start().await;

        // Wait for a few blocks to be produced.
        let leaves = network
            .data_source()
            .subscribe_leaves(0)
            .await
            .take(5)
            .collect::<Vec<_>>()
            .await;
        let payloads = network
            .data_source()
            .subscribe_payloads(0)
            .await
            .take(5)
            .collect::<Vec<_>>()
            .await;
        let vid = network
            .data_source()
            .subscribe_vid_common(0)
            .await
            .take(5)
            .collect::<Vec<_>>()
            .await;

        // Connect a fetching provider.
        let provider = QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        );

        // Make ranged requests.
        tracing::info!("fetch leaf range");
        assert_eq!(
            provider
                .fetch(LeafRangeRequest {
                    start: 0,
                    end: 5,
                    last_leaf: leaves[4].hash(),
                    last_qc: leaves[4].qc().commit(),
                })
                .await
                .unwrap(),
            leaves
        );
        let headers = leaves
            .iter()
            .map(|leaf| leaf.header().clone())
            .collect::<Vec<_>>();
        tracing::info!(?headers, "fetch payload range");
        assert_eq!(
            ProviderTrait::<MockTypes, _>::fetch(
                &provider,
                PayloadRangeRequest::from(RangeRequest::from_headers::<MockTypes>(&headers))
            )
            .await
            .unwrap(),
            payloads
                .into_iter()
                .map(|payload| payload.data)
                .collect::<Vec<_>>()
        );
        tracing::info!(?headers, "fetch VID common range");
        assert_eq!(
            ProviderTrait::<MockTypes, _>::fetch(
                &provider,
                VidCommonRangeRequest::from(RangeRequest::from_headers::<MockTypes>(&headers))
            )
            .await
            .unwrap(),
            vid.into_iter().map(|vid| vid.common).collect::<Vec<_>>()
        );
    }
}
