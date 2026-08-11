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

//! Axum router serving the status API wire protocol.
//!
//! Route paths, response forms, status codes and the wire error envelope (the crate-level
//! [`Error`](crate::Error)) match the old tide-disco handlers and the `status.toml` route specs,
//! so existing clients keep working unchanged. Every route reports a snapshot of live state, so
//! nothing here can be absent: every failure is an internal error, and 500 is the only error
//! status this module serves.
//!
//! The router is an [`ApiRouter`] so that the OpenAPI documentation travels with the routes:
//! an application mounting this module gets the summaries and descriptions without restating
//! them. Use [`From`] to get a plain [`Router`] where the docs are not wanted.

use std::sync::Arc;

use aide::axum::{ApiRouter, routing::get_with};
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
    routing::get,
};
use http_wire::{self as wire, body_limit_layer, cors_layer, healthcheck_response};
use serde::Serialize;

use super::{Error, Options, StatusDataSource};
use crate::Error as AppError;

/// The status module's routes: snapshots of this node's consensus state, summary statistics, and
/// the Prometheus export of every metric it registers.
pub fn status_router<S>(options: &Options, data_source: S) -> ApiRouter
where
    S: StatusDataSource + Send + Sync + 'static,
{
    ApiRouter::new()
        .api_route(
            "/block-height",
            get_with(get_block_height::<S>, |op| {
                op.summary("Get the latest block height")
                    .description("Get the height of the latest committed block.")
            }),
        )
        .api_route(
            "/success-rate",
            get_with(get_success_rate::<S>, |op| {
                op.summary("Get the view success rate").description(
                    "Get the fraction of views which resulted in a committed block, as a floating \
                     point number.",
                )
            }),
        )
        .api_route(
            "/time-since-last-decide",
            get_with(get_time_since_last_decide::<S>, |op| {
                op.summary("Get the time since the last decide")
                    .description(
                        "Get the number of seconds elapsed since this node last decided a block.",
                    )
            }),
        )
        .api_route(
            "/metrics",
            get_with(get_metrics::<S>, |op| {
                op.summary("Get Prometheus metrics").description(
                    "Prometheus endpoint exposing various consensus-related metrics. The response \
                     is the Prometheus text exposition format, not the wire protocol used by the \
                     other routes.",
                )
            }),
        )
        .with_state(RouterState::new(options, data_source))
}

/// Wraps a status router with the app-level `healthcheck`, a request body limit, and permissive
/// CORS headers. Mounting the module prefix is up to the caller.
pub fn app(api: Router) -> Router {
    Router::new()
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .merge(api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}

/// Encode a handler result, wrapping the module error in the crate-level
/// [`Error`](crate::Error) envelope the old tide app served.
fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, Error>) -> Response {
    wire::respond::<AppError, _>(headers, result.map_err(AppError::from))
}

/// Handler context: the data source. [`Options`] carries no settings yet; the router takes it for
/// symmetry with the other modules, and a setting added later lands here rather than in every
/// handler.
struct RouterState<S> {
    data_source: S,
}

impl<S> RouterState<S> {
    fn new(_options: &Options, data_source: S) -> Arc<Self> {
        Arc::new(Self { data_source })
    }
}

async fn get_block_height<S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    S: StatusDataSource + Send + Sync + 'static,
{
    let height = state.data_source.block_height().await;
    respond(&headers, height.map_err(Error::internal))
}

async fn get_success_rate<S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    S: StatusDataSource + Send + Sync + 'static,
{
    let rate = state.data_source.success_rate().await;
    respond(&headers, rate.map_err(Error::internal))
}

async fn get_time_since_last_decide<S>(
    State(state): State<Arc<RouterState<S>>>,
    headers: HeaderMap,
) -> Response
where
    S: StatusDataSource + Send + Sync + 'static,
{
    let elapsed = state.data_source.elapsed_time_since_last_decide().await;
    respond(&headers, elapsed.map_err(Error::internal))
}

/// Serves the Prometheus text exposition format directly, as tide-disco's `METRICS` method did,
/// so scrapers can read the body without knowing this crate's wire protocol. Failures still use
/// the wire error envelope.
async fn get_metrics<S>(State(state): State<Arc<RouterState<S>>>, headers: HeaderMap) -> Response
where
    S: StatusDataSource + Send + Sync + 'static,
{
    match state.data_source.metrics().export() {
        Ok(text) => ([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], text).into_response(),
        Err(err) => wire::encode_err(&headers, AppError::from(Error::internal(err))),
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use http_client::Client;
    use test_utils::reserve_tcp_port;

    use super::*;
    use crate::testing::{
        consensus::{MockDataSource, MockNetwork},
        mocks::MockBase,
        sleep,
    };

    /// Serve `api` under the `/status` prefix on a fresh port and return a connected client rooted
    /// at that prefix.
    async fn start_client(api: ApiRouter) -> Client<AppError, MockBase> {
        let port = reserve_tcp_port().unwrap();
        let url = format!("http://0.0.0.0:{port}").parse().unwrap();
        let _server =
            wire::spawn_serve(&url, app(Router::new().nest("/status", Router::from(api))));

        let client = Client::new(format!("http://localhost:{port}/status").parse().unwrap());
        assert!(client.connect(Some(Duration::from_secs(60))).await);
        client
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_api() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;

        // Start the web server.
        let client = start_client(status_router(&Default::default(), network.data_source())).await;

        // The block height is initially zero.
        assert_eq!(client.get::<u64>("block-height").send().await.unwrap(), 0);

        // The metrics route serves the Prometheus text format, not the wire protocol, so read the
        // body as bytes.
        let prometheus = client.get::<()>("metrics").bytes().await.unwrap();
        let prometheus = String::from_utf8(prometheus).unwrap();
        assert!(
            prometheus
                .lines()
                .any(|line| line == "consensus_current_view 0"),
            "Missing consensus_current_view in metrics:\n{prometheus}"
        );

        // Start the validators and wait for a block to be finalized. There is some delay between
        // the metrics being updated and the decide event being published, so retry until the
        // height catches up.
        network.start().await;
        while client.get::<u64>("block-height").send().await.unwrap() <= 1 {
            tracing::info!("waiting for block height to update");
            sleep(Duration::from_secs(1)).await;
        }

        let success_rate = client.get::<f64>("success-rate").send().await.unwrap();
        // If metrics are populating correctly, we should get a finite number. If not, we might get
        // NaN or infinity due to division by 0.
        assert!(success_rate.is_finite(), "{success_rate}");
        // We know at least some views have been successful, since we finalized a block.
        assert!(success_rate > 0.0, "{success_rate}");

        // Now that a block has been decided, the gauge this route reads is populated.
        client
            .get::<u64>("time-since-last-decide")
            .send()
            .await
            .unwrap();

        network.shut_down().await;
    }

    /// Applications mount this router and serve its documentation as part of their own OpenAPI
    /// spec, so every route it registers must carry a summary.
    #[tokio::test]
    async fn router_documents_every_route() {
        let dir = tempfile::TempDir::new().unwrap();
        let data_source = MockDataSource::create(dir.path(), Default::default())
            .await
            .unwrap();

        let mut api = aide::openapi::OpenApi::default();
        let _ = status_router(&Options::default(), data_source).finish_api(&mut api);

        let paths = api.paths.expect("router registered paths");
        for route in [
            "/block-height",
            "/success-rate",
            "/time-since-last-decide",
            "/metrics",
        ] {
            let aide::openapi::ReferenceOr::Item(item) = &paths.paths[route] else {
                panic!("{route} is a reference, not an operation");
            };
            let op = item
                .get
                .as_ref()
                .unwrap_or_else(|| panic!("{route} has no GET"));
            assert!(op.summary.is_some(), "{route} has no summary");
            assert!(op.description.is_some(), "{route} has no description");
        }
    }
}
