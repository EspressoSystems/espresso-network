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

//! Fetching for light client state update certificates.

use std::{future::IntoFuture, sync::Arc};

use crate::{
    availability::{QueryablePayload, StateCertQueryData},
    data_source::{
        fetching::{
            AvailabilityProvider, FetchRequest, Fetchable, Fetcher, Notifiers, PassiveFetch,
        },
        storage::{
            pruning::PrunedHeightStorage, AvailabilityStorage, NodeStorage,
            UpdateAvailabilityStorage,
        },
        VersionedDataSource,
    },
    Payload, QueryError, QueryResult,
};
use anyhow::bail;
use async_trait::async_trait;
use derive_more::From;
use futures::future::FutureExt;
use hotshot_types::traits::node_implementation::{ConsensusTime, NodeType};

use super::Storable;

#[derive(Clone, Copy, Debug, From)]
pub(super) struct StateCertRequest(u64);

impl FetchRequest for StateCertRequest {}

#[async_trait]
impl<Types> Fetchable<Types> for StateCertQueryData<Types>
where
    Types: NodeType,
    Payload<Types>: QueryablePayload<Types>,
{
    type Request = StateCertRequest;

    /// Does this object satisfy the given request?
    fn satisfies(&self, req: Self::Request) -> bool {
        self.0.epoch.u64() == req.0
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
        match tx.get_state_cert(req.0).await {
            Ok(state_cert) => {
                fetcher.store_and_notify(state_cert).await;
            },
            Err(QueryError::NotFound | QueryError::Missing) => {
                tracing::debug!("state cert not found; trying fetch");
            },
            Err(QueryError::Error { message }) => {
                bail!("failed to get state cert for epoch {}: {message}", req.0);
            },
        }
        Ok(())
    }

    /// Wait for someone else to fetch the object.
    async fn passive_fetch(notifiers: &Notifiers<Types>, req: Self::Request) -> PassiveFetch<Self> {
        notifiers
            .state_cert
            .wait_for(move |data| data.satisfies(req))
            .await
            .into_future()
            .boxed()
    }

    async fn load<S>(storage: &mut S, req: Self::Request) -> QueryResult<Self>
    where
        S: AvailabilityStorage<Types>,
    {
        storage.get_state_cert(req.0).await
    }
}

impl<Types> Storable<Types> for StateCertQueryData<Types>
where
    Types: NodeType,
{
    fn name() -> &'static str {
        "State cert"
    }

    async fn notify(&self, notifiers: &Notifiers<Types>) {
        notifiers.state_cert.notify(self).await;
    }

    async fn store(
        self,
        storage: &mut (impl UpdateAvailabilityStorage<Types> + Send),
        _leaf_only: bool,
    ) -> anyhow::Result<()> {
        storage.insert_state_cert(self).await
    }
}
