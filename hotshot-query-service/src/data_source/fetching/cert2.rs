use std::sync::Arc;

use derivative::Derivative;
use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};
use versions::NEW_PROTOCOL_VERSION;

use super::{AvailabilityProvider, Fetcher};
use crate::{
    Header, Payload,
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

/// Spawn a fetch of the cert2 for `header`'s block, if the block is new enough to have one.
///
/// cert2 exists only from [`NEW_PROTOCOL_VERSION`] (V6) onward, so this skips pre-V6 blocks without
/// bothering peers.
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
    if header.version() < NEW_PROTOCOL_VERSION {
        return;
    }

    // No cert2 backfill in leaf only mode.
    let Some(cert2_fetcher) = &fetcher.cert2_fetcher else {
        return;
    };
    cert2_fetcher.clone().spawn_fetch(
        Certificate2Request {
            height: header.block_number(),
        },
        fetcher.provider.clone(),
        [Cert2Callback {
            fetcher: fetcher.clone(),
        }],
    );
}

/// Callback run when a [`Certificate2Request`] resolves: store the cert2 if the block had one.
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
        self.fetcher.store(&(height, cert2)).await;
    }
}
