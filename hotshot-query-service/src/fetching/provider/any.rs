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

use std::{fmt::Debug, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use hotshot_types::traits::node_implementation::NodeType;

use super::{Provider, Request};
use crate::{
    availability::LeafQueryData,
    data_source::AvailabilityProvider,
    fetching::request::{LeafRequest, PayloadRequest, VidCommonRequest},
    Payload, VidCommon,
};

/// Blanket trait combining [`Debug`] and [`Provider`].
///
/// This is necessary to create a fetcher trait object (`dyn Provider`, see [`PayloadProvider`] and
/// [`LeafProvider`]) which also implements [`Debug`], since trait objects can only have one
/// non-auto trait bound.
trait DebugProvider<Types, T>: Provider<Types, T> + Debug
where
    Types: NodeType,
    T: Request<Types>,
{
}

impl<Types, T, P> DebugProvider<Types, T> for P
where
    Types: NodeType,
    T: Request<Types>,
    P: Provider<Types, T> + Debug,
{
}

type PayloadProvider<Types> = Arc<dyn DebugProvider<Types, PayloadRequest>>;
type LeafProvider<Types> = Arc<dyn DebugProvider<Types, LeafRequest<Types>>>;
type VidCommonProvider<Types> = Arc<dyn DebugProvider<Types, VidCommonRequest>>;

/// Adaptor combining multiple data availability providers.
///
/// This provider adaptor implements the [`Provider`](super::Provider) protocol by fetching
/// requested objects from several different underlying providers. If any of the underlying sources
/// have the object, the request will eventually succeed.
///
/// This can be used to combine multiple instances of the same kind of provider, like using
/// [`QueryServiceProvider`](super::QueryServiceProvider) to request objects from a number of
/// different query services. It can also be used to search different kinds of data providers for
/// the same object, like searching for a block both in another instance of the query service and in
/// the HotShot DA committee. Finally, [`AnyProvider`] can be used to combine a provider which only
/// provides blocks and one which only provides leaves into a provider which provides both, and thus
/// can be used as a provider for the availability API module.
///
/// # Examples
///
/// Fetching from multiple query services, for resiliency.
///
/// ```
/// # use vbs::version::StaticVersionType;
/// # use hotshot_types::traits::node_implementation::NodeType;
/// # async fn doc<Types>() -> anyhow::Result<()>
/// # where
/// #   Types: NodeType,
/// # {
/// use hotshot_query_service::{
///     fetching::provider::{AnyProvider, QueryServiceProvider},
///     testing::mocks::MockBase,
/// };
///
/// let qs1 = QueryServiceProvider::new("https://backup.query-service.1".parse()?, MockBase::instance());
/// let qs2 = QueryServiceProvider::new("https://backup.query-service.2".parse()?, MockBase::instance());
/// let provider = AnyProvider::<Types>::default()
///     .with_provider(qs1)
///     .with_provider(qs2);
/// # Ok(())
/// # }
/// ```
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""), Default(bound = ""))]
pub struct AnyProvider<Types>
where
    Types: NodeType,
{
    payload_providers: Vec<PayloadProvider<Types>>,
    leaf_providers: Vec<LeafProvider<Types>>,
    vid_common_providers: Vec<VidCommonProvider<Types>>,
}

#[async_trait]
impl<Types> Provider<Types, PayloadRequest> for AnyProvider<Types>
where
    Types: NodeType,
{
    async fn fetch(&self, req: PayloadRequest) -> Option<Payload<Types>> {
        any_fetch(&self.payload_providers, req).await
    }
}

#[async_trait]
impl<Types> Provider<Types, LeafRequest<Types>> for AnyProvider<Types>
where
    Types: NodeType,
{
    async fn fetch(&self, req: LeafRequest<Types>) -> Option<LeafQueryData<Types>> {
        any_fetch(&self.leaf_providers, req).await
    }
}

#[async_trait]
impl<Types> Provider<Types, VidCommonRequest> for AnyProvider<Types>
where
    Types: NodeType,
{
    async fn fetch(&self, req: VidCommonRequest) -> Option<VidCommon> {
        any_fetch(&self.vid_common_providers, req).await
    }
}

impl<Types> AnyProvider<Types>
where
    Types: NodeType,
{
    /// Add a sub-provider which fetches both blocks and leaves.
    pub fn with_provider<P>(mut self, provider: P) -> Self
    where
        P: AvailabilityProvider<Types> + Debug + 'static,
    {
        let provider = Arc::new(provider);
        self.payload_providers.push(provider.clone());
        self.leaf_providers.push(provider.clone());
        self.vid_common_providers.push(provider);
        self
    }

    /// Add a sub-provider which fetches blocks.
    pub fn with_block_provider<P>(mut self, provider: P) -> Self
    where
        P: Provider<Types, PayloadRequest> + Debug + 'static,
    {
        self.payload_providers.push(Arc::new(provider));
        self
    }

    /// Add a sub-provider which fetches leaves.
    pub fn with_leaf_provider<P>(mut self, provider: P) -> Self
    where
        P: Provider<Types, LeafRequest<Types>> + Debug + 'static,
    {
        self.leaf_providers.push(Arc::new(provider));
        self
    }

    /// Add a sub-provider which fetches VID common data.
    pub fn with_vid_common_provider<P>(mut self, provider: P) -> Self
    where
        P: Provider<Types, VidCommonRequest> + Debug + 'static,
    {
        self.vid_common_providers.push(Arc::new(provider));
        self
    }
}

async fn any_fetch<Types, P, T>(providers: &[Arc<P>], req: T) -> Option<T::Response>
where
    Types: NodeType,
    P: Provider<Types, T> + Debug + ?Sized,
    T: Request<Types>,
{
    // There's a policy question of how to decide when to try each fetcher: all in parallel, in
    // serial, or a combination. For now, we do the simplest thing of trying each in order, in
    // serial. This has the best performance in the common case when we succeed on the first
    // fetcher: low latency, and no undue burden on the other providers. However, a more complicated
    // strategy where we slowly ramp up the parallelism as more and more requests fail may provide
    // better worst-case latency.
    for (i, p) in providers.iter().enumerate() {
        match p.fetch(req).await {
            Some(obj) => return Some(obj),
            None => {
                tracing::warn!(
                    "failed to fetch {req:?} from provider {i}/{}: {p:?}",
                    providers.len()
                );
                continue;
            },
        }
    }

    None
}

// These tests run the `postgres` Docker image, which doesn't work on Windows.
#[cfg(all(test, not(target_os = "windows")))]
mod test {
    use futures::stream::StreamExt;
    use portpicker::pick_unused_port;
    use tide_disco::App;
    use vbs::version::StaticVersionType;

    use super::*;
    use crate::{
        availability::{define_api, AvailabilityDataSource, UpdateAvailabilityData},
        data_source::storage::sql::testing::TmpDb,
        fetching::provider::{NoFetching, QueryServiceProvider},
        task::BackgroundTask,
        testing::{
            consensus::{MockDataSource, MockNetwork},
            mocks::{MockBase, MockTypes, MockVersions},
            setup_test,
        },
        types::HeightIndexed,
        ApiState, Error,
    };

    type Provider = AnyProvider<MockTypes>;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_first_provider_fails() {
        setup_test();

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
        let _server = BackgroundTask::spawn(
            "server",
            app.serve(format!("0.0.0.0:{port}"), MockBase::instance()),
        );

        // Start a data source which is not receiving events from consensus, only from a peer.
        let db = TmpDb::init().await;
        let provider =
            Provider::default()
                .with_provider(NoFetching)
                .with_provider(QueryServiceProvider::new(
                    format!("http://localhost:{port}").parse().unwrap(),
                    MockBase::instance(),
                ));
        let data_source = db.config().connect(provider.clone()).await.unwrap();

        // Start consensus.
        network.start().await;

        // Wait until the block height reaches 4. This gives us the genesis block, one additional
        // block at the end, and then one block each for fetching a leaf and a payload.
        let leaves = network.data_source().subscribe_leaves(1).await;
        let leaves = leaves.take(3).collect::<Vec<_>>().await;
        let test_leaf = &leaves[0];
        let test_payload = &leaves[1];

        // Give the node a leaf after the range of interest so it learns about the correct block
        // height.
        data_source
            .append(leaves.last().cloned().unwrap().into())
            .await
            .unwrap();

        tracing::info!("requesting leaf from multiple providers");
        let leaf = data_source
            .get_leaf(test_leaf.height() as usize)
            .await
            .await;
        assert_eq!(leaf, *test_leaf);

        tracing::info!("requesting payload from multiple providers");
        let payload = data_source
            .get_payload(test_payload.height() as usize)
            .await
            .await;
        assert_eq!(payload.height(), test_payload.height());
        assert_eq!(payload.block_hash(), test_payload.block_hash());
        assert_eq!(payload.hash(), test_payload.payload_hash());
    }
}
