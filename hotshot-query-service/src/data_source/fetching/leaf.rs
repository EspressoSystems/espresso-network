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

//! [`Fetchable`] implementation for [`LeafQueryData`].

use std::{cmp::Ordering, future::IntoFuture, iter::once, ops::RangeBounds, sync::Arc};

use anyhow::bail;
use async_trait::async_trait;
use committable::Committable;
use derivative::Derivative;
use derive_more::From;
use futures::future::{BoxFuture, FutureExt};
use hotshot_types::traits::node_implementation::NodeType;
use tokio::spawn;
use tracing::Instrument;

use super::{
    header::HeaderCallback, AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Heights,
    Notifiers, RangedFetchable, Storable,
};
use crate::{
    availability::{LeafId, LeafQueryData, QueryablePayload},
    data_source::{
        storage::{
            pruning::PrunedHeightStorage, AvailabilityStorage, NodeStorage,
            UpdateAvailabilityStorage,
        },
        VersionedDataSource,
    },
    fetching::{self, request, Callback},
    types::HeightIndexed,
    Payload, QueryError, QueryResult,
};

pub(super) type LeafFetcher<Types, S, P> =
    fetching::Fetcher<request::LeafRequest<Types>, LeafCallback<Types, S, P>>;

impl<Types> FetchRequest for LeafId<Types>
where
    Types: NodeType,
{
    fn might_exist(self, heights: Heights) -> bool {
        if let LeafId::Number(n) = self {
            heights.might_exist(n as u64)
        } else {
            true
        }
    }
}

#[async_trait]
impl<Types> Fetchable<Types> for LeafQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = LeafId<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        match req {
            LeafId::Number(n) => self.height() == n as u64,
            LeafId::Hash(h) => self.hash() == h,
        }
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .leaf
            .wait_for(move |leaf| leaf.satisfies(req))
            .await
            .into_future()
            .boxed()
    }

    async fn active_fetch<S, P>(
        tx: &mut impl AvailabilityStorage<Types>,
        fetcher: Arc<Fetcher<Types, S, P>>,
        req: Self::Request,
    ) -> anyhow::Result<()>
    where
        S: VersionedDataSource + 'static,
        S::Transaction: UpdateAvailabilityStorage<Types>,
        S::ReadOnly: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
        P: AvailabilityProvider<Types>,
    {
        fetch_leaf_with_callbacks(tx, fetcher, req, None).await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_leaf(req).await
    }
}

pub(super) async fn fetch_leaf_with_callbacks<Types, S, P, I>(
    tx: &mut impl AvailabilityStorage<Types>,
    fetcher: Arc<Fetcher<Types, S, P>>,
    req: LeafId<Types>,
    callbacks: I,
) -> anyhow::Result<()>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    S::Transaction: UpdateAvailabilityStorage<Types>,
    S::ReadOnly: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
    I: IntoIterator<Item = LeafCallback<Types, S, P>> + Send + 'static,
    I::IntoIter: Send,
{
    match req {
        LeafId::Number(n) => {
            // We need the next leaf in the chain so we can figure out what hash we expect for this
            // leaf, so we can fetch it securely from an untrusted provider.
            let next = (n + 1) as u64;
            let next = match tx.first_available_leaf(next).await {
                Ok(leaf) if leaf.height() == next => leaf,
                Ok(leaf) => {
                    // If we don't have the immediate successor leaf, but we have some later leaf,
                    // then we can't trigger this exact fetch, but we can fetch the (apparently)
                    // missing parent of the leaf we do have, which will trigger a chain of fetches
                    // that eventually reaches all the way back to the desired leaf.
                    tracing::debug!(
                        n,
                        fetching = leaf.height() - 1,
                        "do not have necessary leaf; trigger fetch of a later leaf"
                    );

                    let mut callbacks = vec![LeafCallback::Leaf {
                        fetcher: fetcher.clone(),
                    }];

                    if !fetcher.leaf_only {
                        callbacks.push(
                            HeaderCallback::Payload {
                                fetcher: fetcher.clone(),
                            }
                            .into(),
                        );
                        callbacks.push(
                            HeaderCallback::VidCommon {
                                fetcher: fetcher.clone(),
                            }
                            .into(),
                        );
                    }

                    fetcher.leaf_fetcher.clone().spawn_fetch(
                        request::LeafRequest::new(
                            leaf.height() - 1,
                            leaf.leaf().parent_commitment(),
                            leaf.leaf().justify_qc().commit(),
                        ),
                        fetcher.provider.clone(),
                        // After getting the leaf, grab the other data as well; that will be missing
                        // whenever the leaf was.
                        callbacks,
                    );
                    return Ok(());
                },
                Err(QueryError::Missing | QueryError::NotFound) => {
                    // We successfully queried the database, but the next leaf wasn't there. We
                    // know for sure that based on the current state of the DB, we cannot fetch this
                    // leaf.
                    tracing::debug!(n, "not fetching leaf with unknown successor");
                    return Ok(());
                },
                Err(QueryError::Error { message }) => {
                    // An error occurred while querying the database. We don't know if we need to
                    // fetch the leaf or not. Return an error so we can try again.
                    bail!("failed to fetch successor for leaf {n}: {message}");
                },
            };

            let fetcher = fetcher.clone();
            fetcher.leaf_fetcher.clone().spawn_fetch(
                request::LeafRequest::new(
                    n as u64,
                    next.leaf().parent_commitment(),
                    next.leaf().justify_qc().commit(),
                ),
                fetcher.provider.clone(),
                once(LeafCallback::Leaf { fetcher }).chain(callbacks),
            );
        },
        LeafId::Hash(h) => {
            // We don't actively fetch leaves when requested by hash, because we have no way of
            // knowing whether a leaf with such a hash actually exists, and we don't want to bother
            // peers with requests for non-existent leaves.
            tracing::debug!("not fetching unknown leaf {h}");
        },
    }

    Ok(())
}

/// Trigger a fetch of the parent of the given `leaf`, if it is missing.
///
/// Leaves have a unique constraint among fetchable objects: we cannot fetch a given leaf at height
/// `h` unless we have its child at height `h + 1`. This is because the child, through its
/// `parent_commitment`, tells us what the hash of the parent should be, which lets us authenticate
/// it when fetching from an untrusted provider. Thus, requests for leaf `h` might block if `h + 1`
/// is not available. To ensure all these requests are eventually unblocked, and all leaves are
/// eventually fetched, we call this function whenever we receive leaf `h + 1` to check if we need
/// to then fetch leaf `h`.
pub(super) fn trigger_fetch_for_parent<Types, S, P>(
    fetcher: &Arc<Fetcher<Types, S, P>>,
    leaf: &LeafQueryData<Types>,
) where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    S::Transaction: UpdateAvailabilityStorage<Types>,
    S::ReadOnly: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    let height = leaf.height();
    let parent = leaf.leaf().parent_commitment();
    let parent_qc = leaf.leaf().justify_qc().commit();

    // Check that there is a parent to fetch.
    if height == 0 {
        return;
    }

    // Spawn an async task; we're triggering a fire-and-forget fetch of a leaf that might now be
    // available; we don't need to block the caller on this.
    let fetcher = fetcher.clone();
    let span = tracing::info_span!("fetch parent leaf", height, %parent, %parent_qc);
    spawn(
        async move {
            // Check if we already have the parent.
            match fetcher.storage.read().await {
                Ok(mut tx) => {
                    // Don't bother fetching a pruned leaf.
                    if let Ok(pruned_height) = tx.load_pruned_height().await {
                        if pruned_height.is_some_and(|ph| height <= ph) {
                            tracing::info!(
                                height,
                                ?pruned_height,
                                "not fetching pruned parent leaf"
                            );
                            return;
                        }
                    }

                    if tx.get_leaf(((height - 1) as usize).into()).await.is_ok() {
                        return;
                    }
                },
                Err(err) => {
                    // If we can't open a transaction, we can't be sure that we already have the
                    // parent, so we fall through to fetching it just to be safe.
                    tracing::warn!(
                        height,
                        %parent,
                        "error opening transaction to check for parent leaf: {err:#}",
                    );
                },
            }

            tracing::info!(height, %parent, "received new leaf; fetching missing parent");
            fetcher.leaf_fetcher.clone().spawn_fetch(
                request::LeafRequest::new(height - 1, parent, parent_qc),
                fetcher.provider.clone(),
                // After getting the leaf, grab the other data as well; that will be missing
                // whenever the leaf was.
                [
                    LeafCallback::Leaf {
                        fetcher: fetcher.clone(),
                    },
                    HeaderCallback::Payload {
                        fetcher: fetcher.clone(),
                    }
                    .into(),
                    HeaderCallback::VidCommon {
                        fetcher: fetcher.clone(),
                    }
                    .into(),
                ],
            );
        }
        .instrument(span),
    );
}

#[async_trait]
impl<Types> RangedFetchable<Types> for LeafQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type RangedRequest = LeafId<Types>;

    async fn load_range<S, R>(storage: &mut S, range: R) -> QueryResult<Vec<QueryResult<Self>>>
    where
        S: AvailabilityStorage<Types>,
        R: RangeBounds<usize> + Send + 'static,
    {
        storage.get_leaf_range(range).await
    }
}

impl<Types> Storable<Types> for LeafQueryData<Types>
where
    Types: NodeType,
{
    fn name() -> &'static str {
        "leaf"
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        notifiers.leaf.notify(self).await;
    }

    async fn store(
        self,
        storage: &mut (impl UpdateAvailabilityStorage<Types> + Send),
        _leaf_only: bool,
    ) -> anyhow::Result<()> {
        storage.insert_leaf(self).await
    }
}

#[derive(Derivative, From)]
#[derivative(Debug(bound = ""))]
pub(super) enum LeafCallback<Types: NodeType, S, P> {
    /// Callback when fetching the leaf for its own sake.
    #[from(ignore)]
    Leaf {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
    },
    /// Callback when fetching the leaf in order to then look up something else.
    Continuation {
        callback: HeaderCallback<Types, S, P>,
    },
}

impl<Types: NodeType, S, P> PartialEq for LeafCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for LeafCallback<Types, S, P> {}

impl<Types: NodeType, S, P> Ord for LeafCallback<Types, S, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Store leaves in the database before storing derived objects.
            (Self::Leaf { .. }, Self::Continuation { .. }) => Ordering::Less,
            (Self::Continuation { .. }, Self::Leaf { .. }) => Ordering::Greater,

            (Self::Continuation { callback: cb1 }, Self::Continuation { callback: cb2 }) => {
                cb1.cmp(cb2)
            },
            _ => Ordering::Equal,
        }
    }
}

impl<Types: NodeType, S, P> PartialOrd for LeafCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Callback<LeafQueryData<Types>> for LeafCallback<Types, S, P>
where
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    S::Transaction: UpdateAvailabilityStorage<Types>,
    S::ReadOnly: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    async fn run(self, leaf: LeafQueryData<Types>) {
        match self {
            Self::Leaf { fetcher } => {
                tracing::info!("fetched leaf {}", leaf.height());
                // Trigger a fetch of the parent leaf, if we don't already have it.
                trigger_fetch_for_parent(&fetcher, &leaf);
                fetcher.store_and_notify(leaf).await;
            },
            Self::Continuation { callback } => callback.run(leaf.leaf.block_header().clone()),
        }
    }
}
