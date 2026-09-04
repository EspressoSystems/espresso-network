use std::{future::IntoFuture, sync::Arc};

use async_trait::async_trait;
use derivative::Derivative;
use futures::future::{BoxFuture, FutureExt};
use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};
use versions::NEW_PROTOCOL_VERSION;

use super::{AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Heights, Notifiers};
use crate::{
    Header, Payload, QueryError, QueryResult,
    availability::{Certificate2, QueryableHeader, QueryablePayload},
    data_source::{
        VersionedDataSource,
        storage::{
            AvailabilityStorage, NodeStorage, UpdateAvailabilityStorage,
            pruning::PrunedHeightStorage,
        },
    },
    fetching::{self, Callback, request::Certificate2Request},
};

pub(super) type Cert2Fetcher<Types, S, P> =
    fetching::Fetcher<Certificate2Request, Cert2Callback<Types, S, P>>;

impl FetchRequest for Certificate2Request {
    fn might_exist(self, heights: Heights) -> bool {
        heights.might_exist(self.height)
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for Certificate2<Types>
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = Certificate2Request;

    fn satisfies(&self, req: Self::Request) -> bool {
        self.data.block_number == req.height
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .cert2
            .wait_for(move |cert2| cert2.satisfies(req))
            .await
            .into_future()
            .boxed()
    }

    async fn active_fetch<S, P>(
        _tx: &mut impl AvailabilityStorage<Types>,
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: Self::Request,
    ) -> anyhow::Result<()>
    where
        S: VersionedDataSource + 'static,
        for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
        for<'a> S::ReadOnly<'a>:
            AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
        P: AvailabilityProvider<Types>,
    {
        fetch_cert2(&fetcher, req.height);
        Ok(())
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        // Report a missing cert2 as `Missing` so `get` triggers an active fetch.
        storage
            .load_cert2(req.height)
            .await?
            .ok_or(QueryError::Missing)
    }
}

/// Backfill the cert2 for `header`'s block, if it is new enough to have one.
pub(super) fn fetch_cert2_with_header<Types, S, P>(
    fetcher: &Arc<Fetcher<Types, S, P>>,
    header: &Header<Types>,
) where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    if header.version() >= NEW_PROTOCOL_VERSION {
        fetch_cert2(fetcher, header.block_number());
    }
}

pub(super) fn fetch_cert2<Types, S, P>(fetcher: &Arc<Fetcher<Types, S, P>>, height: u64)
where
    Types: NodeType,
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    // No cert2 backfill in leaf-only mode, where derived data is not fetched.
    let Some(cert2_fetcher) = &fetcher.cert2_fetcher else {
        return;
    };
    cert2_fetcher.clone().spawn_fetch(
        Certificate2Request { height },
        fetcher.provider.clone(),
        [Cert2Callback {
            fetcher: fetcher.clone(),
        }],
        false,
    );
}

/// Stores and notifies a fetched cert2, if one was found.
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) struct Cert2Callback<Types: NodeType, S, P> {
    #[derivative(Debug = "ignore")]
    pub(super) fetcher: Arc<Fetcher<Types, S, P>>,
}

impl<Types: NodeType, S, P> PartialEq for Cert2Callback<Types, S, P> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl<Types: NodeType, S, P> Eq for Cert2Callback<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for Cert2Callback<Types, S, P> {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

impl<Types: NodeType, S, P> PartialOrd for Cert2Callback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Callback<Option<Certificate2<Types>>> for Cert2Callback<Types, S, P>
where
    Header<Types>: QueryableHeader<Types>,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, cert2: Option<Certificate2<Types>>) {
        let Some(cert2) = cert2 else {
            return;
        };
        let height = cert2.data.block_number;
        tracing::info!(height, "fetched cert2");
        self.fetcher.store_and_notify(&(height, cert2)).await;
    }
}
