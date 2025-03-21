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

//! Header fetching.

use std::{cmp::Ordering, future::IntoFuture, sync::Arc};

use anyhow::bail;
use async_trait::async_trait;
use committable::Committable;
use derivative::Derivative;
use futures::{future::BoxFuture, FutureExt};
use hotshot_types::traits::{block_contents::BlockHeader, node_implementation::NodeType};

use super::{
    block::fetch_block_with_header, leaf::fetch_leaf_with_callbacks,
    vid::fetch_vid_common_with_header, AvailabilityProvider, Fetcher,
};
use crate::{
    availability::{BlockId, QueryablePayload},
    data_source::{
        fetching::{Fetchable, HeaderQueryData, LeafQueryData, Notifiers},
        storage::{
            pruning::PrunedHeightStorage, AvailabilityStorage, NodeStorage,
            UpdateAvailabilityStorage,
        },
        update::VersionedDataSource,
    },
    Header, Payload, QueryError, QueryResult,
};

impl<Types: NodeType> From<LeafQueryData<Types>> for HeaderQueryData<Types> {
    fn from(leaf: LeafQueryData<Types>) -> Self {
        let header = leaf.header().clone();

        Self { header }
    }
}

fn satisfies_header_req_from_leaf<Types>(leaf: &LeafQueryData<Types>, req: BlockId<Types>) -> bool
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    HeaderQueryData::satisfies(&HeaderQueryData::new(leaf.header().clone()), req)
}

#[async_trait]
impl<Types> Fetchable<Types> for HeaderQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = BlockId<Types>;

    fn satisfies(&self, req: Self::Request) -> bool {
        let header = self.header();
        match req {
            BlockId::Number(n) => header.block_number() as usize == n,
            BlockId::Hash(h) => header.commit() == h,
            BlockId::PayloadHash(h) => header.payload_commitment() == h,
        }
    }

    async fn passive_fetch(
        notifiers: &Notifiers<Types>,
        req: Self::Request,
    ) -> BoxFuture<'static, Option<Self>> {
        notifiers
            .leaf
            .wait_for(move |leaf| satisfies_header_req_from_leaf(leaf, req))
            .await
            .into_future()
            .map(|leaf| leaf.map(HeaderQueryData::from))
            .boxed()
    }

    async fn active_fetch<S, P>(
        tx: &mut impl AvailabilityStorage<Types>,
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
        // Note: if leaf only mode is enabled
        // the payload callback will not do any active fetching and just return
        // This is because we don't have payload fetcher for leaf only mode
        fetch_header_and_then(
            tx,
            req,
            HeaderCallback::Payload {
                fetcher: fetcher.clone(),
            },
        )
        .await
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_header(req).await.map(|header| Self { header })
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub(super) enum HeaderCallback<Types, S, P>
where
    Types: NodeType,
{
    /// Callback when fetching the leaf in order to then look up the corresponding block.
    Payload {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
    },
    /// Callback when fetching the leaf in order to then look up the corresponding VID common data.
    VidCommon {
        #[derivative(Debug = "ignore")]
        fetcher: Arc<Fetcher<Types, S, P>>,
    },
}

impl<Types: NodeType, S, P> PartialEq for HeaderCallback<Types, S, P> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl<Types: NodeType, S, P> Eq for HeaderCallback<Types, S, P> {}

impl<Types: NodeType, S, P> PartialOrd for HeaderCallback<Types, S, P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<Types: NodeType, S, P> Ord for HeaderCallback<Types, S, P> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Arbitrarily, we choose to run payload callbacks before VID callbacks.
        match (self, other) {
            (Self::Payload { .. }, Self::VidCommon { .. }) => Ordering::Less,
            (Self::VidCommon { .. }, Self::Payload { .. }) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    }
}

impl<Types, S, P> HeaderCallback<Types, S, P>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    P: AvailabilityProvider<Types>,
{
    fn fetcher(&self) -> Arc<Fetcher<Types, S, P>> {
        match self {
            Self::Payload { fetcher } => fetcher.clone(),
            Self::VidCommon { fetcher } => fetcher.clone(),
        }
    }

    pub(super) fn run(self, header: Header<Types>) {
        match self {
            Self::Payload { fetcher } => {
                tracing::info!(
                    "fetched leaf {}, will now fetch payload",
                    header.block_number()
                );
                fetch_block_with_header(fetcher, header);
            },
            Self::VidCommon { fetcher } => {
                tracing::info!(
                    "fetched leaf {}, will now fetch VID common",
                    header.block_number()
                );
                fetch_vid_common_with_header(fetcher, header);
            },
        }
    }
}

pub(super) async fn fetch_header_and_then<Types, S, P>(
    tx: &mut impl AvailabilityStorage<Types>,
    req: BlockId<Types>,
    callback: HeaderCallback<Types, S, P>,
) -> anyhow::Result<()>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
    S: VersionedDataSource + 'static,
    for<'a> S::Transaction<'a>: UpdateAvailabilityStorage<Types>,
    for<'a> S::ReadOnly<'a>: AvailabilityStorage<Types> + NodeStorage<Types> + PrunedHeightStorage,
    P: AvailabilityProvider<Types>,
{
    // Check if at least the header is available in local storage. If it is, we benefit two ways:
    // 1. We know for sure the corresponding block exists, so we can unconditionally trigger an
    //    active fetch without unnecessarily bothering our peers.
    // 2. We only need to fetch the missing data (e.g. payload or VID common), not the full block.
    //    Not only is this marginally less data to download, there are some providers that may only
    //    be able to provide certain data. For example, the HotShot DA committee members may be able
    //    to provide paylaods, but not full blocks. Or, in the case where VID recovery is needed,
    //    the VID common data may be available but the full block may not exist anywhere.
    match tx.get_header(req).await {
        Ok(header) => {
            callback.run(header);
            return Ok(());
        },
        Err(QueryError::Missing | QueryError::NotFound) => {
            // We successfully queried the database, but the header wasn't there. Fall through to
            // fetching it.
            tracing::debug!(?req, "header not available locally; trying fetch");
        },
        Err(QueryError::Error { message }) => {
            // An error occurred while querying the database. We don't know if we need to fetch the
            // header or not. Return an error so we can try again.
            bail!("failed to fetch header for block {req:?}: {message}");
        },
    }

    // If the header is _not_ present, we may still be able to fetch the request, but we need to
    // fetch the header (in fact, the entire leaf) first. This is because we have an invariant that
    // we should not store derived objects in the database unless we already have the corresponding
    // header and leaf.
    match req {
        BlockId::Number(n) => {
            fetch_leaf_with_callbacks(tx, callback.fetcher(), n.into(), [callback.into()]).await?;
        },
        BlockId::Hash(h) => {
            // Given only the hash, we cannot tell if the corresponding leaf actually exists, since
            // we don't have a corresponding header. Therefore, we will not spawn an active fetch.
            tracing::debug!("not fetching unknown block {h}");
        },
        BlockId::PayloadHash(h) => {
            // Same as above, we don't fetch a block with a payload that is not known to exist.
            tracing::debug!("not fetching block with unknown payload {h}");
        },
    }

    Ok(())
}
