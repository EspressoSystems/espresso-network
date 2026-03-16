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

//! Fetching missing data from remote providers.
//!
//! This module provides a mechanism to fetch data that is missing from this query service's storage
//! from a remote data availability provider. [`Fetcher`] can be used to handle concurrent requests
//! for data, ensuring that each distinct resource is only fetched once at a time.
//!
//! Fetching is ultimately dispatched to a [`Provider`], which implements fetching for a specific
//! type of resource from a specific source. The [`provider`] module contains built-in
//! implementations of [`Provider`] for various data availability sources.
//!

use std::{
    collections::{BTreeSet, HashMap, hash_map::Entry},
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

use async_lock::{Mutex, Semaphore};
use backoff::{ExponentialBackoff, backoff::Backoff};
use derivative::Derivative;
use tokio::{spawn, time::sleep};

pub mod provider;
pub mod request;

pub use provider::Provider;
pub use request::Request;

/// A callback to process the result of a request.
///
/// Sometimes, we may fetch the same object for multiple purposes, so a request may have more than
/// one callback registered. For example, we may fetch a leaf for its own sake and also to
/// reconstruct a block. Or, we may fetch the same payload for two different blocks. In both of
/// these cases, there are two objects that must be processed and stored after the fetch completes.
///
/// In these cases, we only want one task to actually fetch the resource, but there may be several
/// unrelated actions to take after the resource is fetched. This trait allows us to identify a
/// callback, so that when the task that actually fetched the resource completes, it will run one
/// instance of each distinct callback which was registered. Callbacks will run in the order
/// determined by `Ord`.
#[trait_variant::make(Callback: Send)]
pub trait LocalCallback<T>: Debug + Ord {
    async fn run(self, response: T);
}

/// Management of concurrent requests to fetch resources.
#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
pub struct Fetcher<T, C> {
    #[derivative(Debug = "ignore")]
    in_progress: Arc<Mutex<HashMap<T, BTreeSet<C>>>>,
    backoff: ExponentialBackoff,
    permit: Arc<Semaphore>,
}

impl<T, C> Fetcher<T, C> {
    pub fn new(permit: Arc<Semaphore>, backoff: ExponentialBackoff) -> Self {
        Self {
            in_progress: Default::default(),
            permit,
            backoff,
        }
    }
}

impl<T, C> Fetcher<T, C> {
    /// Fetch a resource, if it is not already being fetched.
    ///
    /// This function will spawn a new task to fetch the resource in the background, using callbacks
    /// to process the fetched resource upon success. If the resource is already being fetched, the
    /// spawned task will terminate without fetching the resource, but only after registering the
    /// provided callbacks to be executed by the existing fetching task upon completion, as long as
    /// there are not already equivalent callbacks registered.
    ///
    /// We spawn a (short-lived) task even if the resource is already being fetched, because the
    /// check that the resource is being fetched requires an exclusive lock, and we do not want to
    /// block the caller, which might be on the critical path of request handling.
    ///
    /// Note that while callbacks are allowed to be async, they are executed sequentially while an
    /// exclusive lock is held, and thus they should not take too long to run or block indefinitely.
    ///
    /// The spawned task will continue trying to fetch the object until it succeeds, so it is the
    /// caller's responsibility only to use this method for resources which are known to exist and
    /// be fetchable by `provider`.
    pub fn spawn_fetch<Types>(
        &self,
        req: T,
        provider: impl Provider<Types, T> + 'static,
        callbacks: impl IntoIterator<Item = C> + Send + 'static,
    ) where
        T: Request<Types> + 'static,
        C: Callback<T::Response> + 'static,
    {
        let in_progress = self.in_progress.clone();
        let permit = self.permit.clone();
        let mut backoff = self.backoff.clone();

        spawn(async move {
            tracing::info!("spawned active fetch for {req:?}");

            // Check if the requested object is already being fetched. If not, take a lock on it so
            // we are the only ones to fetch this particular object.
            {
                let mut in_progress = in_progress.lock().await;
                match in_progress.entry(req) {
                    Entry::Occupied(mut e) => {
                        // If the object is already being fetched, add our callback for the fetching
                        // task to execute upon completion.
                        e.get_mut().extend(callbacks);
                        tracing::info!(?req, callbacks = ?e.get(), "resource is already being fetched");
                        return;
                    },
                    Entry::Vacant(e) => {
                        // If the object is not being fetched, we will register our own callback and
                        // then fetch it ourselves.
                        e.insert(callbacks.into_iter().collect());
                    },
                }
            }

            // Now we are responsible for fetching the object, reach out to the provider.
            backoff.reset();
            let mut delay = backoff.next_backoff().unwrap_or(Duration::from_secs(1));
            let res = loop {
                // Acquire a permit from the semaphore to rate limit the number of concurrent fetch requests
                let permit = permit.acquire().await;
                if let Some(res) = provider.fetch(req).await {
                    break res;
                }

                // We only fetch objects which are known to exist, so we should eventually succeed
                // in fetching if we retry enough. For example, we may be fetching a block from a
                // peer who hasn't received the block yet.
                //
                // To understand why it is ok to retry indefinitely, think about manual
                // intervention: if we don't retry, or retry with a limit, we may require manual
                // intervention whenever a query service fails to fetch a resource that should exist
                // and stops retrying, since it now may never receive that resource. With indefinite
                // fetching, we require manual intervention only when active fetches are
                // accumulating because a peer which _should_ have the resource isn't providing it.
                // In this case, we would require manual intervention on the peer anyways.
                tracing::warn!("failed to fetch {req:?}, will retry in {delay:?}");
                drop(permit);
                sleep(delay).await;

                if let Some(next_delay) = backoff.next_backoff() {
                    delay = next_delay;
                }
            };

            // Done fetching, remove our lock on the object and execute all callbacks.
            //
            // We will keep this lock the whole time we are running the callbacks. We can't release
            // it earlier because we can't allow another task to register a callback after we have
            // taken the list of callbacks that we will execute. We also don't want to allow any new
            // fetches until we have executed the callbacks, because one of the callbacks may store
            // some resource that obviates the need for another fetch.
            //
            // The callbacks may acquire arbitrary locks from this task, while we already hold the
            // lock on `in_progress`. This is fine because we are always running in a freshly
            // spawned task. Therefore we know that this task holds no locks _before_ acquiring
            // `in_progress`, and so it is safe to acquire any lock _after_ acquiring `in_progress`.
            let mut in_progress = in_progress.lock().await;
            let callbacks = in_progress.remove(&req).unwrap_or_default();
            for callback in callbacks {
                callback.run(res.clone()).await;
            }
        });
    }
}
