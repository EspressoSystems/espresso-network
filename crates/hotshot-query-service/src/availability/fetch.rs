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

use std::{future::IntoFuture, time::Duration};

use futures::future::{BoxFuture, Future, FutureExt};
use snafu::{Error, ErrorCompat, IntoError, NoneError, OptionExt};
use tokio::time::timeout;

/// An in-progress request to fetch some data.
///
/// A [`Fetch`] represents the period of uncertainty after some data has been requested, before it
/// is known whether that data exists locally or must be retrieved from an external source (or
/// whether we must wait for the data to be created in the first place).
///
/// For data that is already available locally, a request for that data may return [`Fetch::Ready`]
/// with the data itself. Otherwise, the request will return [`Fetch::Pending`] with a future that
/// will resolve once the data is available.
///
/// Depending on the context, [`Fetch`] can behave a bit like a [`Future`](futures::Future) or a bit
/// like a [`Result`]. Therefore, it implements [`IntoFuture`], so it can be awaited (this is the
/// same as calling [`resolve`](Self::resolve)) and it implements methods [`context`](Self::context)
/// and [`with_context`](Self::with_context), which make it easy to convert a [`Fetch`] to a
/// [`Result`], mimicking the methods from Snafu traits [`OptionExt`](snafu::OptionExt) and
/// [`ResultExt`](snafu::ResultExt).
pub enum Fetch<T> {
    Ready(T),
    Pending(BoxFuture<'static, T>),
}

impl<T> Fetch<T> {
    /// Get the requested data if it is available immediately.
    ///
    /// If the requested data is not immediately available, `Err(self)` is returned so that the
    /// [`Fetch`] object may be used again.
    pub fn try_resolve(self) -> Result<T, Self> {
        match self {
            Self::Ready(obj) => Ok(obj),
            Self::Pending(fut) => Err(Self::Pending(fut)),
        }
    }

    /// Convert this [`Fetch`] to a [`Result`] with the provided error context.
    ///
    /// The result indicates whether the requested data is immediately available. If it is, [`Ok`]
    /// is returned. Otherwise, an error is created from `context` using Snafu.
    pub fn context<C, E>(self, context: C) -> Result<T, E>
    where
        C: IntoError<E, Source = NoneError>,
        E: Error + ErrorCompat,
    {
        self.try_resolve().ok().context(context)
    }

    /// Convert this [`Fetch`] to a [`Result`] with the provided error context.
    ///
    /// The result indicates whether the requested data is immediately available. If it is, [`Ok`]
    /// is returned. Otherwise, an error is created from `context` using Snafu.
    pub fn with_context<F, C, E>(self, context: F) -> Result<T, E>
    where
        F: FnOnce() -> C,
        C: IntoError<E, Source = NoneError>,
        E: Error + ErrorCompat,
    {
        self.try_resolve().ok().with_context(context)
    }

    /// Does this fetch represent an unresolved query?
    pub fn is_pending(&self) -> bool {
        matches!(self, Self::Pending(_))
    }

    pub async fn then_fetch<Fut, R>(
        self,
        f: impl Send + FnOnce(&T) -> Fut + 'static,
    ) -> Fetch<(T, R)>
    where
        Fut: Send + Future<Output = Fetch<R>>,
        T: Send + 'static,
        R: Send + 'static,
    {
        match self {
            Fetch::Ready(t) => f(&t).await.map(|r| (t, r)),
            Fetch::Pending(t) => Fetch::Pending(
                async move {
                    let t = t.await;
                    let r = f(&t).await.await;
                    (t, r)
                }
                .boxed(),
            ),
        }
    }
}

impl<T: Send + 'static> Fetch<T> {
    /// Wait for the data to become available, if it is not already.
    pub async fn resolve(self) -> T {
        self.await
    }

    /// Wait for the requested data to become available, but only for up to `timeout`.
    ///
    /// This function is similar to [`resolve`](Self::resolve), but if the future does not resolve
    /// within `timeout`, then [`with_timeout`](Self::with_timeout) will resolve with [`None`].
    pub async fn with_timeout(self, timeout_duration: Duration) -> Option<T> {
        timeout(timeout_duration, self.into_future()).await.ok()
    }
}

impl<T: 'static> Fetch<T> {
    /// Transform the result of this fetch.
    ///
    /// If the requested data is already available, `f` will be applied immediately. Otherwise, `f`
    /// will be saved and applied after the pending future resolves.
    pub fn map<F, U>(self, f: F) -> Fetch<U>
    where
        F: 'static + Send + FnOnce(T) -> U,
    {
        match self {
            Self::Ready(obj) => Fetch::Ready(f(obj)),
            Self::Pending(fut) => Fetch::Pending(fut.map(f).boxed()),
        }
    }
}

impl<T: Send + 'static> IntoFuture for Fetch<T> {
    type Output = T;
    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            match self {
                Self::Ready(obj) => obj,
                Self::Pending(fut) => fut.await,
            }
        }
        .boxed()
    }
}
