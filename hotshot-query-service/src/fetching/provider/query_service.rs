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

use async_trait::async_trait;
use committable::Committable;
use hotshot_types::{
    data::{ns_table, VidCommitment},
    traits::{block_contents::BlockHeader, node_implementation::NodeType, EncodeBytes},
    vid::{
        advz::{advz_scheme, ADVZScheme},
        avidm::{init_avidm_param, AvidMScheme},
    },
};
use jf_vid::VidScheme;
use surf_disco::{Client, Url};
use vbs::{version::StaticVersionType, BinarySerializer};

use super::Provider;
use crate::{
    availability::{
        ADVZCommonQueryData, ADVZPayloadQueryData, LeafQueryData, LeafQueryDataLegacy,
        PayloadQueryData, VidCommonQueryData,
    },
    fetching::request::{LeafRequest, PayloadRequest, VidCommonRequest},
    types::HeightIndexed,
    Error, Header, Payload, VidCommon,
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
    async fn deserialize_legacy_payload<Types: NodeType>(
        &self,
        payload_bytes: Vec<u8>,
        common_bytes: Vec<u8>,
        req: PayloadRequest,
    ) -> Option<Payload<Types>> {
        let client_url = self.client.base_url();

        let PayloadRequest(VidCommitment::V0(advz_commit)) = req else {
            return None;
        };

        let payload = match vbs::Serializer::<Ver>::deserialize::<ADVZPayloadQueryData<Types>>(
            &payload_bytes,
        ) {
            Ok(payload) => payload,
            Err(err) => {
                tracing::warn!(%err, ?req, "failed to deserialize ADVZPayloadQueryData");
                return None;
            },
        };

        let common = match vbs::Serializer::<Ver>::deserialize::<ADVZCommonQueryData<Types>>(
            &common_bytes,
        ) {
            Ok(common) => common,
            Err(err) => {
                tracing::warn!(%err, ?req, "failed to deserialize ADVZPayloadQueryData");
                return None;
            },
        };

        let num_storage_nodes = ADVZScheme::get_num_storage_nodes(common.common()) as usize;
        let bytes = payload.data.encode();

        let commit = advz_scheme(num_storage_nodes)
            .commit_only(bytes)
            .inspect_err(|err| {
                tracing::error!(%err, ?req, "failed to compute legacy VID commitment");
            })
            .ok()?;

        if commit != advz_commit {
            tracing::error!(
                ?req,
                expected_commit=%advz_commit,
                actual_commit=%commit,
                %client_url,
                "received inconsistent legacy payload"
            );
            return None;
        }

        Some(payload.data)
    }

    async fn deserialize_legacy_vid_common<Types: NodeType>(
        &self,
        bytes: Vec<u8>,
        req: VidCommonRequest,
    ) -> Option<VidCommon> {
        let client_url = self.client.base_url();
        let VidCommonRequest(VidCommitment::V0(advz_commit)) = req else {
            return None;
        };

        match vbs::Serializer::<Ver>::deserialize::<ADVZCommonQueryData<Types>>(&bytes) {
            Ok(res) => {
                if ADVZScheme::is_consistent(&advz_commit, &res.common).is_ok() {
                    Some(VidCommon::V0(res.common))
                } else {
                    tracing::error!(%client_url, ?req, ?res.common, "fetched inconsistent VID common data");
                    None
                }
            },
            Err(err) => {
                tracing::warn!(
                    %client_url,
                    ?req,
                    %err,
                    "failed to deserialize ADVZCommonQueryData"
                );
                None
            },
        }
    }
    async fn deserialize_legacy_leaf<Types: NodeType>(
        &self,
        bytes: Vec<u8>,
        req: LeafRequest<Types>,
    ) -> Option<LeafQueryData<Types>> {
        let client_url = self.client.base_url();

        match vbs::Serializer::<Ver>::deserialize::<LeafQueryDataLegacy<Types>>(&bytes) {
            Ok(mut leaf) => {
                if leaf.height() != req.height {
                    tracing::error!(
                        %client_url, ?req,
                        expected_height = req.height,
                        actual_height = leaf.height(),
                        "received leaf with the wrong height"
                    );
                    return None;
                }

                let expected_leaf_commit: [u8; 32] = req.expected_leaf.into();
                let actual_leaf_commit: [u8; 32] = leaf.hash().into();
                if actual_leaf_commit != expected_leaf_commit {
                    tracing::error!(
                        %client_url, ?req,
                        expected_leaf = %req.expected_leaf,
                        actual_leaf = %leaf.hash(),
                        "received leaf with the wrong hash"
                    );
                    return None;
                }

                let expected_qc_commit: [u8; 32] = req.expected_qc.into();
                let actual_qc_commit: [u8; 32] = leaf.qc().commit().into();
                if actual_qc_commit != expected_qc_commit {
                    tracing::error!(
                        %client_url, ?req,
                        expected_qc = %req.expected_qc,
                        actual_qc = %leaf.qc().commit(),
                        "received leaf with the wrong QC"
                    );
                    return None;
                }

                // There is a potential DOS attack where the peer sends us a leaf with the full
                // payload in it, which uses redundant resources in the database, since we fetch and
                // store payloads separately. We can defend ourselves by simply dropping the payload
                // if present.
                leaf.leaf.unfill_block_payload();

                Some(leaf.into())
            },
            Err(err) => {
                tracing::warn!(
                    %client_url, ?req, %err,
                    "failed to deserialize legacy LeafQueryData"
                );
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
    /// Fetches the `Payload` for a given request.
    ///
    /// Attempts to fetch and deserialize the requested data using the new type first.
    /// If deserialization into the new type fails (e.g., because the provider is still returning
    /// legacy data), it falls back to attempt deserialization using an older, legacy type instead.
    /// This fallback ensures compatibility with older nodes or providers that have not yet upgraded.
    ///
    async fn fetch(&self, req: PayloadRequest) -> Option<Payload<Types>> {
        let client_url = self.client.base_url();
        let req_hash = req.0;
        // Fetch the payload and the VID common data. We need the common data to recompute the VID
        // commitment, to ensure the payload we received is consistent with the commitment we
        // requested.
        let payload_bytes = self
            .client
            .get::<()>(&format!("availability/payload/hash/{}", req.0))
            .bytes()
            .await
            .inspect_err(|err| {
                tracing::info!(%err, %req_hash, %client_url, "failed to fetch payload bytes");
            })
            .ok()?;

        let common_bytes = self
            .client
            .get::<()>(&format!("availability/vid/common/payload-hash/{}", req.0))
            .bytes()
            .await
            .inspect_err(|err| {
                tracing::info!(%err, %req_hash, %client_url, "failed to fetch VID common bytes");
            })
            .ok()?;

        let payload =
            vbs::Serializer::<Ver>::deserialize::<PayloadQueryData<Types>>(&payload_bytes)
                .inspect_err(|err| {
                    tracing::info!(%err, %req_hash, "failed to deserialize PayloadQueryData");
                })
                .ok();

        let common =
            vbs::Serializer::<Ver>::deserialize::<VidCommonQueryData<Types>>(&common_bytes)
                .inspect_err(|err| {
                    tracing::info!(%err, %req_hash,
                        "error deserializing VidCommonQueryData",
                    );
                })
                .ok();

        let (payload, common) = match (payload, common) {
            (Some(payload), Some(common)) => (payload, common),
            _ => {
                tracing::info!(%req_hash, "falling back to legacy payload deserialization");

                // fallback deserialization
                return self
                    .deserialize_legacy_payload::<Types>(payload_bytes, common_bytes, req)
                    .await;
            },
        };

        match common.common() {
            VidCommon::V0(common) => {
                let num_storage_nodes = ADVZScheme::get_num_storage_nodes(common) as usize;
                let bytes = payload.data().encode();

                let commit = advz_scheme(num_storage_nodes)
                    .commit_only(bytes)
                    .map(VidCommitment::V0)
                    .inspect_err(|err| {
                        tracing::error!(%err, %req_hash,  %num_storage_nodes, "failed to compute VID commitment (V0)");
                    })
                    .ok()?;

                if commit != req.0 {
                    tracing::error!(
                        expected = %req_hash,
                        actual = ?commit,
                        %client_url,
                        "VID commitment mismatch (V0)"
                    );

                    return None;
                }
            },
            VidCommon::V1(common) => {
                let bytes = payload.data().encode();

                let avidm_param = init_avidm_param(common.total_weights)
                    .inspect_err(|err| {
                        tracing::error!(%err, %req_hash, "failed to initialize AVIDM params. total_weight={}", common.total_weights);
                    })
                    .ok()?;

                let header = self
                    .client
                    .get::<Header<Types>>(&format!("availability/header/{}", payload.height()))
                    .send()
                    .await
                    .inspect_err(|err| {
                        tracing::warn!(%client_url, %err, "failed to fetch header for payload. height={}", payload.height());
                    })
                    .ok()?;

                if header.payload_commitment() != req.0 {
                    tracing::error!(
                        expected = %req_hash,
                        actual = %header.payload_commitment(),
                        %client_url,
                        "header payload commitment mismatch (V1)"
                    );
                    return None;
                }

                let metadata = header.metadata().encode();
                let commit = AvidMScheme::commit(
                    &avidm_param,
                    &bytes,
                    ns_table::parse_ns_table(bytes.len(), &metadata),
                )
                .map(VidCommitment::V1)
                .inspect_err(|err| {
                    tracing::error!(%err, %req_hash, "failed to compute AVIDM commitment");
                })
                .ok()?;

                // Compare calculated commitment with requested commitment
                if commit != req.0 {
                    tracing::warn!(
                        expected = %req_hash,
                        actual = %commit,
                        %client_url,
                        "commitment type mismatch for AVIDM check"
                    );
                    return None;
                }
            },
        }

        Some(payload.data)
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, LeafRequest<Types>>
    for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    /// Fetches the `Leaf` for a given request.
    ///
    /// Attempts to fetch and deserialize the requested data using the new type first.
    /// If deserialization into the new type fails (e.g., because the provider is still returning
    /// legacy data), it falls back to attempt deserialization using an older, legacy type instead.
    /// This fallback ensures compatibility with older nodes or providers that have not yet upgraded.
    ///
    async fn fetch(&self, req: LeafRequest<Types>) -> Option<LeafQueryData<Types>> {
        let client_url = self.client.base_url();

        let bytes = self
            .client
            .get::<()>(&format!("availability/leaf/{}", req.height))
            .bytes()
            .await;
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::info!(%client_url, ?req, %err, "failed to fetch bytes for leaf");

                return None;
            },
        };

        // Attempt to deserialize using the new type

        match vbs::Serializer::<Ver>::deserialize::<LeafQueryData<Types>>(&bytes) {
            Ok(mut leaf) => {
                if leaf.height() != req.height {
                    tracing::error!(
                        %client_url, ?req, ?leaf,
                        expected_height = req.height,
                        actual_height = leaf.height(),
                        "received leaf with the wrong height"
                    );
                    return None;
                }
                if leaf.hash() != req.expected_leaf {
                    tracing::error!(
                        %client_url, ?req, ?leaf,
                        expected_hash = %req.expected_leaf,
                        actual_hash = %leaf.hash(),
                        "received leaf with the wrong hash"
                    );
                    return None;
                }
                if leaf.qc().commit() != req.expected_qc {
                    tracing::error!(
                        %client_url, ?req, ?leaf,
                        expected_qc = %req.expected_qc,
                        actual_qc = %leaf.qc().commit(),
                        "received leaf with the wrong QC"
                    );
                    return None;
                }

                // There is a potential DOS attack where the peer sends us a leaf with the full
                // payload in it, which uses redundant resources in the database, since we fetch and
                // store payloads separately. We can defend ourselves by simply dropping the payload
                // if present.
                leaf.leaf.unfill_block_payload();

                Some(leaf)
            },
            Err(err) => {
                tracing::info!(
                    ?req, %err,
                    "failed to deserialize LeafQueryData, falling back to legacy deserialization"
                );
                // Fallback deserialization
                self.deserialize_legacy_leaf(bytes, req).await
            },
        }
    }
}

#[async_trait]
impl<Types, Ver: StaticVersionType> Provider<Types, VidCommonRequest> for QueryServiceProvider<Ver>
where
    Types: NodeType,
{
    /// Fetches the `VidCommon` for a given request.
    ///
    /// Attempts to fetch and deserialize the requested data using the new type first.
    /// If deserialization into the new type fails (e.g., because the provider is still returning
    /// legacy data), it falls back to attempt deserialization using an older, legacy type instead.
    /// This fallback ensures compatibility with older nodes or providers that have not yet upgraded.
    ///
    async fn fetch(&self, req: VidCommonRequest) -> Option<VidCommon> {
        let client_url = self.client.base_url();
        let bytes = self
            .client
            .get::<()>(&format!("availability/vid/common/payload-hash/{}", req.0))
            .bytes()
            .await;
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(err) => {
                tracing::info!(
                    %client_url, ?req, %err,
                    "failed to fetch VID common bytes"
                );
                return None;
            },
        };

        match vbs::Serializer::<Ver>::deserialize::<VidCommonQueryData<Types>>(&bytes) {
            Ok(res) => match req.0 {
                VidCommitment::V0(commit) => {
                    if let VidCommon::V0(common) = res.common {
                        if ADVZScheme::is_consistent(&commit, &common).is_ok() {
                            Some(VidCommon::V0(common))
                        } else {
                            tracing::error!(
                                %client_url, ?req, ?commit, ?common,
                                "VID V0 common data is inconsistent with commitment"
                            );
                            None
                        }
                    } else {
                        tracing::warn!(?req, ?res, "Expect VID common data but found None");
                        None
                    }
                },
                VidCommitment::V1(_) => {
                    if let VidCommon::V1(common) = res.common {
                        Some(VidCommon::V1(common))
                    } else {
                        tracing::warn!(?req, ?res, "Expect VID common data but found None");
                        None
                    }
                },
            },
            Err(err) => {
                tracing::info!(
                    %client_url, ?req, %err,
                    "failed to deserialize as V1 VID common data, trying legacy fallback"
                );
                // Fallback deserialization
                self.deserialize_legacy_vid_common::<Types>(bytes, req)
                    .await
            },
        }
    }
}

// These tests run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(all(test, not(target_os = "windows")))]
mod test {
    use std::{future::IntoFuture, time::Duration};

    use committable::Committable;
    use futures::{
        future::{join, FutureExt},
        stream::StreamExt,
    };
    use generic_array::GenericArray;
    use hotshot_example_types::node_types::{EpochsTestVersions, TestVersions};
    use hotshot_types::traits::node_implementation::Versions;
    use portpicker::pick_unused_port;
    use rand::RngCore;
    use tide_disco::{error::ServerError, App};
    use vbs::version::StaticVersion;

    use super::*;
    use crate::{
        api::load_api,
        availability::{
            define_api, AvailabilityDataSource, BlockId, BlockInfo, BlockQueryData,
            BlockWithTransaction, Fetch, UpdateAvailabilityData,
        },
        data_source::{
            sql::{self, SqlDataSource},
            storage::{
                fail_storage::{FailStorage, FailableAction},
                pruning::{PrunedHeightStorage, PrunerCfg},
                sql::testing::TmpDb,
                AvailabilityStorage, SqlStorage, UpdateAvailabilityStorage,
            },
            AvailabilityProvider, FetchingDataSource, Transaction, VersionedDataSource,
        },
        fetching::provider::{NoFetching, Provider as ProviderTrait, TestProvider},
        node::{data_source::NodeDataSource, SyncStatus},
        task::BackgroundTask,
        testing::{
            consensus::{MockDataSource, MockNetwork},
            mocks::{mock_transaction, MockBase, MockTypes, MockVersions},
            sleep,
        },
        types::HeightIndexed,
        ApiState,
    };

    type Provider = TestProvider<QueryServiceProvider<MockBase>>;
    type EpochProvider = TestProvider<QueryServiceProvider<<EpochsTestVersions as Versions>::Base>>;

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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let mut network = MockNetwork::<MockDataSource, EpochsTestVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                <EpochsTestVersions as Versions>::Base::instance(),
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        network.spawn(
            "server",
            app.serve(
                format!("0.0.0.0:{port}"),
                <EpochsTestVersions as Versions>::Base::instance(),
            ),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        // Use our special test provider that handles epoch version transitions
        let db = TmpDb::init().await;
        let provider = EpochProvider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            <EpochsTestVersions as Versions>::Base::instance(),
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        tx.insert_leaf(finalized_leaves[2].clone()).await.unwrap();
        tx.insert_leaf(finalized_leaves[4].clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
                .append(BlockInfo::new(leaf, Some(block.clone()), None, None, None))
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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

    fn random_vid_commit() -> VidCommitment {
        let mut bytes = [0; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        VidCommitment::V0(GenericArray::from(bytes).into())
    }

    async fn malicious_server(port: u16) {
        let mut api = load_api::<(), ServerError, MockBase>(
            None::<std::path::PathBuf>,
            include_str!("../../../api/availability.toml"),
            vec![],
        )
        .unwrap();

        api.get("get_payload", move |_, _| {
            async move {
                // No matter what data we are asked for, always respond with dummy data.
                Ok(PayloadQueryData::<MockTypes>::genesis::<TestVersions>(
                    &Default::default(),
                    &Default::default(),
                )
                .await)
            }
            .boxed()
        })
        .unwrap()
        .get("get_vid_common", move |_, _| {
            async move {
                // No matter what data we are asked for, always respond with dummy data.
                Ok(VidCommonQueryData::<MockTypes>::genesis::<TestVersions>(
                    &Default::default(),
                    &Default::default(),
                )
                .await)
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
        let port = pick_unused_port().unwrap();
        let _server = BackgroundTask::spawn("malicious server", malicious_server(port));

        let provider = QueryServiceProvider::new(
            format!("http://localhost:{port}").parse().unwrap(),
            MockBase::instance(),
        );
        provider.client.connect(None).await;

        // Query for a random payload, the server will respond with a different payload, and we
        // should detect the error.
        let res =
            ProviderTrait::<MockTypes, _>::fetch(&provider, PayloadRequest(random_vid_commit()))
                .await;
        assert_eq!(res, None);

        // Query for a random VID common, the server will respond with a different one, and we
        // should detect the error.
        let res =
            ProviderTrait::<MockTypes, _>::fetch(&provider, VidCommonRequest(random_vid_commit()))
                .await;
        assert_eq!(res, None);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_archive_recovery() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
            .with_minor_scan_interval(Duration::from_secs(1))
            .with_major_scan_interval(1)
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
            if (SyncStatus {
                missing_vid_shares: 0,
                ..sync_status
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
            (SyncStatus {
                missing_vid_shares: 0,
                ..sync_status
            })
            .is_fully_synced(),
            "{sync_status:?}"
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
        tx.insert_leaf(last_leaf.clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
        tx.insert_leaf(last_leaf.clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        data_source.append(leaf.clone().into()).await.unwrap();

        // We will eventually retrieve the corresponding block and VID common, triggered by seeing
        // the leaf.
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
        tx.insert_leaf(last_leaf.clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
        tx.insert_leaf(leaf.clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
            tx.insert_leaf(leaf.clone()).await.unwrap();
            tx.insert_block(block.clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
        tx.insert_leaf(last_leaf.clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
        tx.insert_leaf(last_leaf.clone()).await.unwrap();
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
        let mut network = MockNetwork::<MockDataSource, MockVersions>::init().await;

        // Start a web server that the non-consensus node can use to fetch blocks.
        let port = pick_unused_port().unwrap();
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
        let storage = FailStorage::from(SqlStorage::connect(db.config()).await.unwrap());
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
        tx.insert_leaf(last_leaf.clone()).await.unwrap();
        tx.commit().await.unwrap();

        // Send the first object to the disconnected data source, so we hit all the cases:
        // * leaf present but not full object (from the last leaf)
        // * full object present but inaccessible due to storage failures (first object)
        // * nothing present (middle object)
        let leaf = network.data_source().get_leaf(1).await.await;
        let block = network.data_source().get_block(1).await.await;
        let vid = network.data_source().get_vid_common(1).await.await;
        data_source
            .append(BlockInfo::new(leaf, Some(block), Some(vid), None, None))
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

    // This helper function starts up a mock network
    // with v0 and v1 availability query modules,
    // trigger fetches for a datasource from the provider,
    // and asserts that the fetched data is correct
    async fn run_fallback_deserialization_test_helper<V: Versions>(port: u16, version: &str) {
        let mut network = MockNetwork::<MockDataSource, V>::init().await;

        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));

        // Register availability APIs for two versions: v0 and v1
        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                StaticVersion::<0, 1> {},
                "0.0.1".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                StaticVersion::<0, 1> {},
                "1.0.0".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), StaticVersion::<0, 1> {}),
        );

        let db = TmpDb::init().await;

        let provider_url = format!("http://localhost:{port}/{version}")
            .parse()
            .expect("Invalid URL");

        let provider = Provider::new(QueryServiceProvider::new(
            provider_url,
            StaticVersion::<0, 1> {},
        ));

        let ds = data_source(&db, &provider).await;
        network.start().await;

        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;
        let test_leaf = &leaves[0];
        let test_payload = &leaves[2];
        let test_common = &leaves[3];

        let mut fetches = vec![];
        // Issue requests for missing data (these should initially remain unresolved):
        fetches.push(ds.get_leaf(test_leaf.height() as usize).await.map(ignore));
        fetches.push(ds.get_payload(test_payload.block_hash()).await.map(ignore));
        fetches.push(
            ds.get_vid_common(test_common.block_hash())
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

        // Append the latest known leaf to the local store
        // This would trigger fetches for the corresponding missing data
        // such as header, vid and payload
        // This would also trigger fetches for the parent data
        ds.append(leaves.last().cloned().unwrap().into())
            .await
            .unwrap();

        // check that the data has been fetches and matches the network data source
        {
            let leaf = ds.get_leaf(test_leaf.height() as usize).await;
            let payload = ds.get_payload(test_payload.height() as usize).await;
            let common = ds.get_vid_common(test_common.height() as usize).await;

            let truth = network.data_source();
            assert_eq!(
                leaf.await,
                truth.get_leaf(test_leaf.height() as usize).await.await
            );
            assert_eq!(
                payload.await,
                truth
                    .get_payload(test_payload.height() as usize)
                    .await
                    .await
            );
            assert_eq!(
                common.await,
                truth
                    .get_vid_common(test_common.height() as usize)
                    .await
                    .await
            );
        }
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fallback_deserialization_for_fetch_requests_v0() {
        let port = pick_unused_port().unwrap();

        // This run will call v0 availalbilty api for fetch requests.
        // The fetch initially attempts deserialization with new types,
        // which fails because the v0 provider returns legacy types.
        // It then falls back to deserializing as legacy types,
        // and the fetch passes
        run_fallback_deserialization_test_helper::<MockVersions>(port, "v0").await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fallback_deserialization_for_fetch_requests_v1() {
        let port = pick_unused_port().unwrap();

        // Fetch from the v1 availability API using MockVersions.
        // this one fetches from the v1 provider.
        // which would correctly deserialize the bytes in the first attempt, so no fallback deserialization is needed
        run_fallback_deserialization_test_helper::<MockVersions>(port, "v1").await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fallback_deserialization_for_fetch_requests_pos() {
        let port = pick_unused_port().unwrap();

        // Fetch Proof of Stake (PoS) data using the v1 availability API
        // with proof of stake version
        run_fallback_deserialization_test_helper::<EpochsTestVersions>(port, "v1").await;
    }
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fallback_deserialization_for_fetch_requests_v0_pos() {
        // Run with the PoS version against a v0 provider.
        // Fetch requests are expected to fail because PoS commitments differ from the legacy commitments
        // returned by the v0 provider.
        // For example: a PoS Leaf2 commitment will not match the downgraded commitment from a legacy Leaf1.

        let mut network = MockNetwork::<MockDataSource, EpochsTestVersions>::init().await;

        let port = pick_unused_port().unwrap();
        let mut app = App::<_, Error>::with_state(ApiState::from(network.data_source()));

        app.register_module(
            "availability",
            define_api(
                &Default::default(),
                StaticVersion::<0, 1> {},
                "0.0.1".parse().unwrap(),
            )
            .unwrap(),
        )
        .unwrap();

        network.spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), StaticVersion::<0, 1> {}),
        );

        let db = TmpDb::init().await;
        let provider = Provider::new(QueryServiceProvider::new(
            format!("http://localhost:{port}/v0").parse().unwrap(),
            StaticVersion::<0, 1> {},
        ));
        let ds = data_source(&db, &provider).await;

        network.start().await;

        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(5).collect::<Vec<_>>().await;
        let test_leaf = &leaves[0];
        let test_payload = &leaves[2];
        let test_common = &leaves[3];

        let leaf = ds.get_leaf(test_leaf.height() as usize).await;
        let payload = ds.get_payload(test_payload.height() as usize).await;
        let common = ds.get_vid_common(test_common.height() as usize).await;

        sleep(Duration::from_secs(3)).await;

        // fetches fail because of different commitments
        leaf.try_resolve().unwrap_err();
        payload.try_resolve().unwrap_err();
        common.try_resolve().unwrap_err();
    }
}
