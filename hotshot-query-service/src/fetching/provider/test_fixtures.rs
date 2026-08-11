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

//! Axum test fixture standing in for a peer query service.
//!
//! Provider tests fetch from a second data source over HTTP. The routes come from the real
//! availability router ([`availability::router`]); this module adds the glue those tests need:
//! the `/availability` module prefix the old tide app registered, the app-level healthcheck, the
//! tide-style unknown-route error, and a serve helper that waits for the server to come up.

use std::time::Duration;

use axum::{
    Router,
    http::{HeaderMap, Uri},
    response::Response,
    routing::get,
};
use disco_types::status::StatusCode;
use http_client::Client;
use http_wire::{self as wire, healthcheck_response, spawn_serve};
use test_utils::reserve_tcp_port;
use tokio::task::JoinHandle;
use vbs::version::StaticVersion;

use crate::{
    Error,
    availability::{self, AvailabilityDataSource, router::availability_router},
    testing::mocks::MockTypes,
};

/// Unknown routes are reported the way tide-disco reported them: the provider's ranged-VID
/// fallback keys off the "No route matches" message to detect old peers.
async fn no_route(headers: HeaderMap, uri: Uri) -> Response {
    let err = Error::Custom {
        message: format!("No route matches {}", uri.path()),
        status: StatusCode::NOT_FOUND,
    };
    wire::encode_err(&headers, err)
}

/// Mounts `api` under `/availability` (the module prefix the old tide app registered) with the
/// app-level healthcheck and the tide-style unknown-route error.
pub(crate) fn app(api: Router) -> Router {
    Router::new()
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .nest("/availability", api)
        .fallback(no_route)
}

/// Bind `router` on a fresh port and serve it for the rest of the test process. Waits for the
/// server to answer its healthcheck, so one-shot fetches do not race the bind.
pub(crate) async fn serve(router: Router) -> (u16, JoinHandle<()>) {
    let port = reserve_tcp_port().unwrap();
    let url = format!("http://0.0.0.0:{port}").parse().unwrap();
    let task = spawn_serve(&url, router);
    let client: Client<Error, StaticVersion<0, 1>> =
        Client::new(format!("http://localhost:{port}").parse().unwrap());
    assert!(client.connect(Some(Duration::from_secs(60))).await);
    (port, task)
}

/// Serve the real availability router (default [`Options`](availability::Options)) for
/// `data_source` on a fresh port. Returns the port and the server task.
pub(crate) async fn serve_availability<D>(data_source: D) -> (u16, JoinHandle<()>)
where
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let api = Router::from(availability_router::<MockTypes, D>(
        &availability::Options::default(),
        data_source,
    ));
    serve(app(api)).await
}
