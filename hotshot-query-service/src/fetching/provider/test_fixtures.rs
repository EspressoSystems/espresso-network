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
//! Provider tests fetch from a second data source over HTTP. This fixture serves exactly the
//! availability routes [`TrustedQueryServiceProvider`](super::TrustedQueryServiceProvider)
//! requests, replicating the semantics of the old tide-disco handlers: integer height params,
//! TaggedBase64 payload-hash params, the default fetch timeout, missing data as 404, and the
//! crate-level [`Error`] envelope on the wire.

use std::{fmt::Display, marker::PhantomData, ops::Range, sync::Arc, time::Duration};

use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode as HttpStatusCode, Uri},
    response::Response,
    routing::{get, post},
};
use disco_types::status::StatusCode;
use futures::{StreamExt, TryStreamExt};
use hotshot_types::data::VidCommitment;
use http_client::Client;
use http_wire::{self as wire, WireFormat, healthcheck_response, spawn_serve};
use serde::Serialize;
use snafu::OptionExt;
use tagged_base64::TaggedBase64;
use test_utils::reserve_tcp_port;
use tokio::task::JoinHandle;
use vbs::version::{StaticVersion, StaticVersionType};

use crate::{
    Error,
    availability::{
        self, AvailabilityDataSource, BlockId, FetchBlockSnafu, FetchLeafSnafu, LeafId,
    },
    data_source::{VersionedDataSource, storage::AvailabilityStorage},
    testing::mocks::MockTypes,
};

/// Wire format of the fixture: `Ver` VBS framing and the crate-level [`Error`] envelope, the
/// same envelope the old tide app served and the provider's client decodes.
struct QueryServiceWireFormat<Ver>(PhantomData<Ver>);

impl<Ver: StaticVersionType + 'static> WireFormat for QueryServiceWireFormat<Ver> {
    type Error = Error;
    type Version = Ver;

    fn status(err: &Error) -> HttpStatusCode {
        HttpStatusCode::from_u16(u16::from(disco_types::error::Error::status(err)))
            .unwrap_or(HttpStatusCode::INTERNAL_SERVER_ERROR)
    }

    fn serialize_failure(message: String) -> Error {
        Error::internal(message)
    }
}

pub(crate) fn respond<Ver: StaticVersionType + 'static, T: Serialize>(
    headers: &HeaderMap,
    result: Result<T, Error>,
) -> Response {
    wire::respond::<QueryServiceWireFormat<Ver>, _>(headers, result)
}

fn fetch_timeout() -> Duration {
    availability::Options::default().fetch_timeout
}

fn payload_hash_param(value: &str) -> Result<VidCommitment, Error> {
    let err = || Error::Custom {
        message: format!("invalid payload hash {value}"),
        status: StatusCode::BAD_REQUEST,
    };
    let tb64: TaggedBase64 = value.parse().map_err(|_| err())?;
    VidCommitment::try_from(&tb64).map_err(|_| err())
}

async fn get_leaf<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let fetch = ds.get_leaf(LeafId::<MockTypes>::Number(height)).await;
    let result = fetch
        .with_timeout(fetch_timeout())
        .await
        .context(FetchLeafSnafu {
            resource: height.to_string(),
        })
        .map_err(Error::from);
    respond::<Ver, _>(&headers, result)
}

async fn get_leaf_range<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let leaves = ds.get_leaf_range(from..until).await;
    let result = leaves
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(fetch_timeout())
                .await
                .context(FetchLeafSnafu {
                    resource: (index + from).to_string(),
                })
        })
        .try_collect::<Vec<_>>()
        .await
        .map_err(Error::from);
    respond::<Ver, _>(&headers, result)
}

/// Decode a batch request body into the height ranges it asks for.
fn batch_ranges<Ver: StaticVersionType + 'static>(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<Vec<Range<u64>>, Error> {
    let ranges =
        wire::decode_body::<Ver, Vec<(u64, u64)>>(headers, body).map_err(|err| Error::Custom {
            message: format!("invalid batch request: {err}"),
            status: StatusCode::BAD_REQUEST,
        })?;
    Ok(ranges
        .into_iter()
        .map(|(from, until)| from..until)
        .collect())
}

fn storage_error(err: impl Display) -> Error {
    Error::Custom {
        message: format!("batch query failed: {err}"),
        status: StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn get_leaf_batch<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: VersionedDataSource + Send + Sync + 'static,
    for<'a> D::ReadOnly<'a>: AvailabilityStorage<MockTypes>,
{
    let result = match batch_ranges::<Ver>(&headers, &body) {
        Ok(ranges) => match ds.read().await {
            Ok(mut tx) => tx.get_leaf_batch(&ranges).await.map_err(storage_error),
            Err(err) => Err(storage_error(err)),
        },
        Err(err) => Err(err),
    };
    respond::<Ver, _>(&headers, result)
}

async fn get_block_batch<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: VersionedDataSource + Send + Sync + 'static,
    for<'a> D::ReadOnly<'a>: AvailabilityStorage<MockTypes>,
{
    let result = match batch_ranges::<Ver>(&headers, &body) {
        Ok(ranges) => match ds.read().await {
            Ok(mut tx) => tx.get_block_batch(&ranges).await.map_err(storage_error),
            Err(err) => Err(storage_error(err)),
        },
        Err(err) => Err(err),
    };
    respond::<Ver, _>(&headers, result)
}

async fn get_vid_common_batch<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: VersionedDataSource + Send + Sync + 'static,
    for<'a> D::ReadOnly<'a>: AvailabilityStorage<MockTypes>,
{
    let result = match batch_ranges::<Ver>(&headers, &body) {
        Ok(ranges) => match ds.read().await {
            Ok(mut tx) => tx
                .get_vid_common_batch(&ranges)
                .await
                .map_err(storage_error),
            Err(err) => Err(storage_error(err)),
        },
        Err(err) => Err(err),
    };
    respond::<Ver, _>(&headers, result)
}

async fn get_block_range<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let blocks = ds.get_block_range(from..until).await;
    let result = blocks
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(fetch_timeout())
                .await
                .context(FetchBlockSnafu {
                    resource: (index + from).to_string(),
                })
        })
        .try_collect::<Vec<_>>()
        .await
        .map_err(Error::from);
    respond::<Ver, _>(&headers, result)
}

async fn get_block_by_payload_hash<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let result = async {
        let id = BlockId::<MockTypes>::PayloadHash(payload_hash_param(&hash)?);
        let fetch = ds.get_block(id).await;
        Ok(fetch
            .with_timeout(fetch_timeout())
            .await
            .context(FetchBlockSnafu {
                resource: id.to_string(),
            })?)
    }
    .await;
    respond::<Ver, _>(&headers, result)
}

async fn get_cert2<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path(height): Path<u64>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let result = ds
        .get_cert2(height)
        .await
        .with_timeout(fetch_timeout())
        .await
        .ok_or_else(|| {
            availability::Error::Custom {
                message: format!("no cert2 available for height {height}"),
                status: StatusCode::NOT_FOUND,
            }
            .into()
        });
    respond::<Ver, _>(&headers, result)
}

async fn get_vid_common<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path(height): Path<usize>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let fetch = ds
        .get_vid_common(BlockId::<MockTypes>::Number(height))
        .await;
    let result = fetch
        .with_timeout(fetch_timeout())
        .await
        .context(FetchBlockSnafu {
            resource: height.to_string(),
        })
        .map_err(Error::from);
    respond::<Ver, _>(&headers, result)
}

async fn get_vid_common_range<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path((from, until)): Path<(usize, usize)>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let vid = ds.get_vid_common_range(from..until).await;
    let result = vid
        .enumerate()
        .then(|(index, fetch)| async move {
            fetch
                .with_timeout(fetch_timeout())
                .await
                .context(FetchBlockSnafu {
                    resource: (index + from).to_string(),
                })
        })
        .try_collect::<Vec<_>>()
        .await
        .map_err(Error::from);
    respond::<Ver, _>(&headers, result)
}

async fn get_vid_common_by_payload_hash<Ver, D>(
    State(ds): State<Arc<D>>,
    headers: HeaderMap,
    Path(hash): Path<String>,
) -> Response
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + Send + Sync + 'static,
{
    let result = async {
        let id = BlockId::<MockTypes>::PayloadHash(payload_hash_param(&hash)?);
        let fetch = ds.get_vid_common(id).await;
        Ok(fetch
            .with_timeout(fetch_timeout())
            .await
            .context(FetchBlockSnafu {
                resource: id.to_string(),
            })?)
    }
    .await;
    respond::<Ver, _>(&headers, result)
}

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

/// Unknown routes are reported the way tide-disco reported them: the provider's ranged-VID
/// fallback keys off the "No route matches" message to detect old peers.
async fn no_route<Ver: StaticVersionType + 'static>(headers: HeaderMap, uri: Uri) -> Response {
    let err = Error::Custom {
        message: format!("No route matches {}", uri.path()),
        status: StatusCode::NOT_FOUND,
    };
    wire::encode_err::<QueryServiceWireFormat<Ver>>(&headers, err)
}

/// The availability routes the fetch provider client requests.
fn availability_routes<Ver, D>(data_source: Arc<D>) -> Router
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + VersionedDataSource + Send + Sync + 'static,
    for<'a> D::ReadOnly<'a>: AvailabilityStorage<MockTypes>,
{
    Router::new()
        .route("/leaf/{height}", get(get_leaf::<Ver, D>))
        .route("/leaf/{from}/{until}", get(get_leaf_range::<Ver, D>))
        .route(
            "/block/payload-hash/{hash}",
            get(get_block_by_payload_hash::<Ver, D>),
        )
        .route("/block/{from}/{until}", get(get_block_range::<Ver, D>))
        .route("/cert2/{height}", get(get_cert2::<Ver, D>))
        .route(
            "/vid/common/payload-hash/{hash}",
            get(get_vid_common_by_payload_hash::<Ver, D>),
        )
        .route("/vid/common/{height}", get(get_vid_common::<Ver, D>))
        .route(
            "/vid/common/{from}/{until}",
            get(get_vid_common_range::<Ver, D>),
        )
        .with_state(data_source)
}

/// Just the batch routes, for tests that must reach a peer through them and no other way.
pub(crate) fn batch_routes<Ver, D>(data_source: Arc<D>) -> Router
where
    Ver: StaticVersionType + 'static,
    D: VersionedDataSource + Send + Sync + 'static,
    for<'a> D::ReadOnly<'a>: AvailabilityStorage<MockTypes>,
{
    Router::new()
        .route("/leaf/batch", post(get_leaf_batch::<Ver, D>))
        .route("/block/batch", post(get_block_batch::<Ver, D>))
        .route("/vid/common/batch", post(get_vid_common_batch::<Ver, D>))
        .with_state(data_source)
}

/// Mounts `api` under `/availability` (the module prefix the old tide app registered) with the
/// app-level healthcheck and the tide-style unknown-route error.
pub(crate) fn app<Ver: StaticVersionType + 'static>(api: Router) -> Router {
    Router::new()
        .route("/healthcheck", get(healthcheck))
        .nest("/availability", api)
        .fallback(no_route::<Ver>)
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

/// Serve the availability fixture for `data_source` on a fresh port. Returns the port and the
/// server task.
pub(crate) async fn serve_availability<Ver, D>(_: Ver, data_source: D) -> (u16, JoinHandle<()>)
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + VersionedDataSource + Send + Sync + 'static,
    for<'a> D::ReadOnly<'a>: AvailabilityStorage<MockTypes>,
{
    let data_source = Arc::new(data_source);
    let api = availability_routes::<Ver, _>(data_source.clone())
        .merge(batch_routes::<Ver, _>(data_source));
    serve(app::<Ver>(api)).await
}

/// Serve a peer that predates the batch routes, to exercise the fallback to per-range fetches.
pub(crate) async fn serve_availability_without_batch<Ver, D>(
    _: Ver,
    data_source: D,
) -> (u16, JoinHandle<()>)
where
    Ver: StaticVersionType + 'static,
    D: AvailabilityDataSource<MockTypes> + VersionedDataSource + Send + Sync + 'static,
    for<'a> D::ReadOnly<'a>: AvailabilityStorage<MockTypes>,
{
    serve(app::<Ver>(availability_routes::<Ver, _>(Arc::new(
        data_source,
    ))))
    .await
}
