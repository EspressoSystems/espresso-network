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

//! Asynchronous fetching from external data availability providers.
//!
//! Occasionally, data will be missing from the local persistent storage of this query service. This
//! may be because the query service never received the data from the attached HotShot instance,
//! which happens for each block payload committed while the attached HotShot instance is not a
//! member of the DA committee. It may also be because the query service was started some time after
//! the start of consensus, and needs to catch up on existing data. Or it may simply be a result of
//! failures in the local storage medium.
//!
//! In any case, the query service is able to fetch missing data asynchronously and out of
//! chronological order, and pending [`Fetch`](crate::availability::Fetch) requests for the missing
//! data will resolve as soon as the data is available. Data can be fetched from any external data
//! availability service, including the HotShot CDN, the data availability committee which is
//! responsible for providing a particular payload, or another instance of this same query service.
//!
//! This module defines an abstract interface [`Provider`], which allows data to be fetched from any
//! data availability provider, as well as various implementations for different data sources,
//! including:
//! * [`QueryServiceProvider`]
//!
//! We also provide combinators for modularly adding functionality to existing fetchers:
//! * [`AnyProvider`]
//! * [`TestProvider`]
//!

use std::sync::Arc;

use async_trait::async_trait;

use super::Request;

mod any;
mod query_service;
mod testing;

pub use any::AnyProvider;
pub use query_service::QueryServiceProvider;
#[cfg(any(test, feature = "testing"))]
pub use testing::TestProvider;

/// A provider which is able to satisfy requests for data of type `T`.
///
/// This trait use boxed future return types (`#[async_trait]`) instead of `impl Future` return
/// types, so that it can be object safe.
#[async_trait]
pub trait Provider<Types, T: Request<Types>>: Send + Sync {
    /// Fetch a resource.
    async fn fetch(&self, req: T) -> Option<T::Response>;
}

/// Trivial [`Provider`] where fetching always fails.
///
/// Useful for examples and tests which should never have need of a fetcher.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoFetching;

#[async_trait]
impl<Types, T: Send + Request<Types> + 'static> Provider<Types, T> for NoFetching {
    async fn fetch(&self, _req: T) -> Option<T::Response> {
        None
    }
}

#[async_trait]
impl<Types, T, P> Provider<Types, T> for Arc<P>
where
    T: Request<Types> + 'static,
    P: Provider<Types, T> + Sync,
{
    async fn fetch(&self, req: T) -> Option<T::Response> {
        (**self).fetch(req).await
    }
}
