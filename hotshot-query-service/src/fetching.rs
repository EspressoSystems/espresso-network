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

use anyhow::ensure;
use async_lock::{Mutex, Semaphore};
use backoff::{ExponentialBackoff, backoff::Backoff};
use derivative::Derivative;
use derive_more::Into;
use serde::{Deserialize, Serialize};
use tokio::{spawn, time::sleep};

pub mod provider;
pub mod request;

pub use provider::Provider;
pub use request::Request;

use crate::types::HeightIndexed;

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

/// Added type safety for objects which are fetched in batches.
///
/// A [`NonEmptyRange`] has a similar interface as a [`Vec`], but it enforces, via the methods with
/// which it can be constructed, that the data it contains is always
/// * at least one object of type `T`
/// * a contiguous range of objects ordered by increasing [`height`](HeightIndexed::height).
#[derive(Clone, Debug, Into, Deserialize, Serialize, PartialEq, Eq)]
#[serde(
    // Important: use `try_from` when deserializing so that we perform the necessary invariant
    // checks.
    try_from = "Vec<T>",
    into = "Vec<T>",
    bound(
        deserialize = "T: HeightIndexed + serde::de::DeserializeOwned",
        serialize = "T: Clone + Serialize"
    )
)]
pub struct NonEmptyRange<T>(Vec<T>);

impl<T> NonEmptyRange<T>
where
    T: HeightIndexed,
{
    /// Construct a [`NonEmptyRange`] from a sequence of elements.
    ///
    /// # Errors
    ///
    /// This constructor will fail if the given sequence is empty, or if its elements do not
    /// represent a contiguous range by height.
    pub fn new(elems: impl IntoIterator<Item = T>) -> anyhow::Result<Self> {
        elems.into_iter().collect::<Vec<_>>().try_into()
    }

    /// The inclusive lower bound of the range of heights of objects in this [`NonEmptyRange`].
    pub fn start(&self) -> u64 {
        self.0[0].height()
    }

    /// The exclusive upper bound of the range of heights of objects in this [`NonEmptyRange`].
    pub fn end(&self) -> u64 {
        self.start() + (self.len() as u64)
    }

    /// The number of objects in this [`NonEmptyRange`].
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the [`NonEmptyRange`] is empty.
    ///
    /// This function always returns `false`. It is included only because it is idiomatically
    /// paired with [`Self::len`], as demanded by Clippy.
    pub fn is_empty(&self) -> bool {
        false
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.0.iter()
    }

    /// Convert a range of objects into an equivalent range of sub-objects with the same heights.
    pub(crate) fn as_ref_cloned<U>(&self) -> NonEmptyRange<U>
    where
        T: AsRef<U>,
        U: Clone,
    {
        NonEmptyRange(self.0.iter().map(|t| t.as_ref().clone()).collect())
    }
}

impl<T> TryFrom<Vec<T>> for NonEmptyRange<T>
where
    T: HeightIndexed,
{
    type Error = anyhow::Error;

    fn try_from(elems: Vec<T>) -> Result<Self, Self::Error> {
        ensure!(
            !elems.is_empty(),
            "cannot construct a non-empty range from an empty vector"
        );
        for (x, y) in elems.iter().zip(&elems[1..]) {
            ensure!(
                x.height() + 1 == y.height(),
                "cannot construct a non-empty range from a non-contiguous vector"
            );
        }
        Ok(Self(elems))
    }
}

impl<T> PartialEq<Vec<T>> for NonEmptyRange<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Vec<T>) -> bool {
        self.0.eq(other)
    }
}

impl<T> IntoIterator for NonEmptyRange<T> {
    type IntoIter = <Vec<T> as IntoIterator>::IntoIter;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a NonEmptyRange<T> {
    type IntoIter = <&'a Vec<T> as IntoIterator>::IntoIter;
    type Item = &'a T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<T> AsRef<[T]> for NonEmptyRange<T> {
    fn as_ref(&self) -> &[T] {
        self.0.as_ref()
    }
}
