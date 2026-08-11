// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Axum routers serving the v0.1 builder API wire protocol.
//!
//! Route paths, status codes and the wire error type ([`Error`]) match the legacy `block_info`
//! and `txn_submit` modules, so existing builder clients keep working unchanged, with one
//! deliberate divergence: `txn_submit/status/{transaction_hash}` parses the path parameter its
//! API definition always declared, where the legacy handler ignored it and decoded the request
//! body as a transaction instead.

use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode as HttpStatusCode},
    response::Response,
    routing::{get, post},
};
use committable::Committable;
use disco_types::{error::Error as _, request::RequestParamType, status::StatusCode};
use hotshot_types::{
    constants::LEGACY_BUILDER_MODULE,
    data::VidCommitment,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    utils::BuilderCommitment,
};
use http_wire::{
    self as wire, DecodeFailure, WireFormat, body_limit_layer, cors_layer, healthcheck_response,
    module_healthcheck_response,
};
use serde::{Serialize, de::DeserializeOwned};
use tagged_base64::TaggedBase64;
use url::Url;

use super::{
    Version,
    block_info::{
        AvailableBlockData, AvailableBlockHeaderInputV1, AvailableBlockHeaderInputV2,
        AvailableBlockInfo,
    },
    builder::{Error, RequestError, TransactionStatus},
    data_source::{AcceptsTxnSubmits, BuilderDataSource},
};

/// Wire format of the builder API: [`Version`] VBS framing and the [`Error`] envelope.
struct BuilderWireFormat;

impl WireFormat for BuilderWireFormat {
    type Error = Error;
    type Version = Version;

    fn status(err: &Error) -> HttpStatusCode {
        HttpStatusCode::from_u16(u16::from(err.status()))
            .unwrap_or(HttpStatusCode::INTERNAL_SERVER_ERROR)
    }

    fn serialize_failure(message: String) -> Error {
        Error::Custom {
            message,
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, Error>) -> Response {
    wire::respond::<BuilderWireFormat, _>(headers, result)
}

fn decode_body<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> Result<T, RequestError> {
    wire::decode_body::<Version, T>(headers, body).map_err(|failure| match failure {
        DecodeFailure::Json(_) => RequestError::Json,
        DecodeFailure::Binary(_) => RequestError::Binary,
        DecodeFailure::UnsupportedContentType => RequestError::UnsupportedContentType,
    })
}

fn tb64_request_error(field: &str) -> Error {
    Error::Request(RequestError::TaggedBase64 {
        reason: format!("invalid tagged base64 for {field}"),
    })
}

/// Parses a hash-type path parameter (`parent_hash`, `block_hash`, `transaction_hash`). Any
/// failure is an [`Error::Request`], which maps to 400, like the legacy `blob_param` path did
/// for these params.
fn parse_hash_param<T>(value: &str, field: &str) -> Result<T, Error>
where
    T: for<'a> TryFrom<&'a TaggedBase64>,
{
    let tb64: TaggedBase64 = value.parse().map_err(|_| tb64_request_error(field))?;
    T::try_from(&tb64).map_err(|_| tb64_request_error(field))
}

/// Parses a key/signature path parameter: a wrong-type value is a `Custom` error carrying 422.
fn parse_key_param<T>(value: &str, field: &str) -> Result<T, Error>
where
    T: for<'a> TryFrom<&'a TaggedBase64>,
{
    let tb64: TaggedBase64 = value.parse().map_err(|_| tb64_request_error(field))?;
    T::try_from(&tb64).map_err(|_| Error::Custom {
        message: format!("Invalid {field}"),
        status: StatusCode::UNPROCESSABLE_ENTITY,
    })
}

/// Parses an integer path parameter (`view_number`, `num_nodes`). Reproduces the error the
/// legacy `Integer` params produced for a bad value: axum's own `Path` rejection would be a
/// plain-text 400, which clients expecting the [`Error`] envelope cannot decode.
fn parse_int_param<T: std::str::FromStr>(value: &str) -> Result<T, Error> {
    value.parse().map_err(|_| {
        Error::Request(RequestError::IncorrectParamType {
            actual: RequestParamType::Literal,
            expected: RequestParamType::Integer,
        })
    })
}

type Sender<Types> = <Types as NodeType>::SignatureKey;
type Signature<Types> = <Sender<Types> as SignatureKey>::PureAssembledSignatureType;

fn parse_sender_signature<Types: NodeType>(
    sender: &str,
    signature: &str,
) -> Result<(Sender<Types>, Signature<Types>), Error> {
    let sender = parse_key_param::<Sender<Types>>(sender, "sender")?;
    let signature = parse_key_param::<Signature<Types>>(signature, "signature")?;
    Ok((sender, signature))
}

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

/// The legacy server answered `/<module>/healthcheck` for every registered module (a bare
/// "Available", since neither module defined its own handler); each module router keeps that
/// route so it survives version mounting.
async fn module_healthcheck(headers: HeaderMap) -> Response {
    module_healthcheck_response(&headers)
}

async fn available_blocks<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((parent_hash, view_number, sender, signature)): Path<(String, String, String, String)>,
) -> Response {
    let result: Result<Vec<AvailableBlockInfo<Types>>, Error> = async {
        let hash = parse_hash_param::<VidCommitment>(&parent_hash, "parent_hash")?;
        let view_number = parse_int_param(&view_number)?;
        let (sender, signature) = parse_sender_signature::<Types>(&sender, &signature)?;
        state
            .available_blocks(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| Error::BlockAvailable {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    respond(&headers, result)
}

async fn claim_block<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, String, String, String)>,
) -> Response {
    let result: Result<AvailableBlockData<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let view_number = parse_int_param(&view_number)?;
        let (sender, signature) = parse_sender_signature::<Types>(&sender, &signature)?;
        state
            .claim_block(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| Error::BlockClaim {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    respond(&headers, result)
}

async fn claim_block_with_num_nodes<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature, num_nodes)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
) -> Response {
    let result: Result<AvailableBlockData<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let view_number = parse_int_param(&view_number)?;
        let (sender, signature) = parse_sender_signature::<Types>(&sender, &signature)?;
        let num_nodes = parse_int_param(&num_nodes)?;
        state
            .claim_block_with_num_nodes(&hash, view_number, sender, &signature, num_nodes)
            .await
            .map_err(|source| Error::BlockClaim {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    respond(&headers, result)
}

async fn claim_header_input<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, String, String, String)>,
) -> Response {
    let result: Result<AvailableBlockHeaderInputV1<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let view_number = parse_int_param(&view_number)?;
        let (sender, signature) = parse_sender_signature::<Types>(&sender, &signature)?;
        state
            .claim_block_header_input(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| Error::BlockClaim {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    respond(&headers, result)
}

async fn claim_header_input_v2<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, String, String, String)>,
) -> Response {
    let result: Result<AvailableBlockHeaderInputV2<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let view_number = parse_int_param(&view_number)?;
        let (sender, signature) = parse_sender_signature::<Types>(&sender, &signature)?;
        let input = state
            .claim_block_header_input(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| Error::BlockClaim {
                source,
                resource: hash.to_string(),
            })?;
        Ok(AvailableBlockHeaderInputV2 {
            fee_signature: input.fee_signature,
            sender: input.sender,
        })
    }
    .await;
    respond(&headers, result)
}

async fn builder_address<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Response {
    let result = state.builder_address().await.map_err(Error::from);
    respond(&headers, result)
}

async fn submit_txn<Types: NodeType, S: AcceptsTxnSubmits<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let result = async {
        let tx: Types::Transaction = decode_body(&headers, &body).map_err(Error::TxnUnpack)?;
        let hash = tx.commit();
        state
            .submit_txns(vec![tx])
            .await
            .map_err(Error::TxnSubmit)?;
        Ok(hash)
    }
    .await;
    respond(&headers, result)
}

async fn submit_batch<Types: NodeType, S: AcceptsTxnSubmits<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let result = async {
        let txns: Vec<Types::Transaction> =
            decode_body(&headers, &body).map_err(Error::TxnUnpack)?;
        let hashes = txns.iter().map(|tx| tx.commit()).collect::<Vec<_>>();
        state.submit_txns(txns).await.map_err(Error::TxnSubmit)?;
        Ok(hashes)
    }
    .await;
    respond(&headers, result)
}

async fn get_status<Types: NodeType, S: AcceptsTxnSubmits<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path(transaction_hash): Path<String>,
) -> Response {
    let result: Result<TransactionStatus, Error> = async {
        let hash = parse_hash_param(&transaction_hash, "transaction_hash")?;
        state.txn_status(hash).await.map_err(Error::TxnStat)
    }
    .await;
    respond(&headers, result)
}

/// The `block_info` module's routes, to be nested at `/block_info`.
pub fn block_info_router<Types, S>(state: S) -> Router
where
    Types: NodeType,
    S: BuilderDataSource<Types> + Clone + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/availableblocks/{parent_hash}/{view_number}/{sender}/{signature}",
            get(available_blocks::<Types, S>),
        )
        .route(
            "/claimblock/{block_hash}/{view_number}/{sender}/{signature}",
            get(claim_block::<Types, S>),
        )
        .route(
            "/claimblockwithnumnodes/{block_hash}/{view_number}/{sender}/{signature}/{num_nodes}",
            get(claim_block_with_num_nodes::<Types, S>),
        )
        .route(
            "/claimheaderinput/{block_hash}/{view_number}/{sender}/{signature}",
            get(claim_header_input::<Types, S>),
        )
        .route(
            "/claimheaderinput/v2/{block_hash}/{view_number}/{sender}/{signature}",
            get(claim_header_input_v2::<Types, S>),
        )
        .route("/builderaddress", get(builder_address::<Types, S>))
        .route("/healthcheck", get(module_healthcheck))
        .with_state(state)
}

/// The `txn_submit` module's routes, to be nested at `/txn_submit`.
pub fn txn_submit_router<Types, S>(state: S) -> Router
where
    Types: NodeType,
    S: AcceptsTxnSubmits<Types> + Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/submit", post(submit_txn::<Types, S>))
        .route("/batch", post(submit_batch::<Types, S>))
        .route("/status/{transaction_hash}", get(get_status::<Types, S>))
        .route("/healthcheck", get(module_healthcheck))
        .with_state(state)
}

/// Wraps module routers with the app-level `healthcheck`, a `/v0` mirror of every module (the
/// legacy server served each module both unversioned and under its major version), a request
/// body limit, and permissive CORS headers.
pub fn app(api: Router) -> Router {
    Router::new()
        .route("/healthcheck", get(healthcheck))
        .merge(api.clone())
        .nest("/v0", api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}

/// The full builder server: both modules mounted at their canonical paths, wrapped by [`app`].
/// The single place the mount paths are spelled, so servers and tests cannot drift.
pub fn builder_app<Types, S>(state: S) -> Router
where
    Types: NodeType,
    S: BuilderDataSource<Types> + AcceptsTxnSubmits<Types> + Clone + Send + Sync + 'static,
{
    app(Router::new()
        .nest(
            &format!("/{LEGACY_BUILDER_MODULE}"),
            block_info_router::<Types, S>(state.clone()),
        )
        .nest("/txn_submit", txn_submit_router::<Types, S>(state)))
}

/// Binds `url`'s host and port and serves `router` until the returned handle is aborted.
///
/// # Panics
/// If `url` has no port or the port cannot be bound.
pub fn serve(url: &Url, router: Router) -> tokio::task::JoinHandle<()> {
    wire::spawn_serve(url, router)
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use axum::http::{Method, Request, header};
    use committable::Commitment;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::traits::signature_key::BuilderSignatureKey;

    use super::*;
    use crate::v0_1::builder::BuildError;

    /// The tests below pin routing shape and response headers, not handler bodies, so a stub
    /// source is all they need.
    #[derive(Clone)]
    struct StubSource;

    #[async_trait]
    impl BuilderDataSource<TestTypes> for StubSource {
        async fn available_blocks(
            &self,
            _for_parent: &VidCommitment,
            _view_number: u64,
            _sender: Sender<TestTypes>,
            _signature: &Signature<TestTypes>,
        ) -> Result<Vec<AvailableBlockInfo<TestTypes>>, BuildError> {
            Ok(vec![])
        }

        async fn claim_block(
            &self,
            _block_hash: &BuilderCommitment,
            _view_number: u64,
            _sender: Sender<TestTypes>,
            _signature: &Signature<TestTypes>,
        ) -> Result<AvailableBlockData<TestTypes>, BuildError> {
            Err(BuildError::NotFound)
        }

        async fn claim_block_with_num_nodes(
            &self,
            _block_hash: &BuilderCommitment,
            _view_number: u64,
            _sender: Sender<TestTypes>,
            _signature: &Signature<TestTypes>,
            _num_nodes: usize,
        ) -> Result<AvailableBlockData<TestTypes>, BuildError> {
            Err(BuildError::NotFound)
        }

        async fn claim_block_header_input(
            &self,
            _block_hash: &BuilderCommitment,
            _view_number: u64,
            _sender: Sender<TestTypes>,
            _signature: &Signature<TestTypes>,
        ) -> Result<AvailableBlockHeaderInputV1<TestTypes>, BuildError> {
            Err(BuildError::NotFound)
        }

        async fn builder_address(
            &self,
        ) -> Result<<TestTypes as NodeType>::BuilderSignatureKey, BuildError> {
            type Key = <TestTypes as NodeType>::BuilderSignatureKey;
            Ok(<Key as BuilderSignatureKey>::generated_from_seed_indexed([0; 32], 0).0)
        }
    }

    #[async_trait]
    impl AcceptsTxnSubmits<TestTypes> for StubSource {
        async fn submit_txns(
            &self,
            txns: Vec<<TestTypes as NodeType>::Transaction>,
        ) -> Result<Vec<Commitment<<TestTypes as NodeType>::Transaction>>, BuildError> {
            Ok(txns.iter().map(Committable::commit).collect())
        }

        async fn txn_status(
            &self,
            _txn_hash: Commitment<<TestTypes as NodeType>::Transaction>,
        ) -> Result<TransactionStatus, BuildError> {
            Ok(TransactionStatus::Unknown)
        }
    }

    fn test_router() -> Router {
        builder_app::<TestTypes, _>(StubSource)
    }

    async fn request(method: Method, uri: &str) -> axum::http::Response<axum::body::Body> {
        let req = Request::builder()
            .method(method)
            .uri(uri)
            .header(header::ORIGIN, "https://example.com")
            .body(axum::body::Body::empty())
            .unwrap();
        tower::ServiceExt::oneshot(test_router(), req)
            .await
            .unwrap()
    }

    /// The legacy server served both modules at `/v0/<module>/...` and redirected the unversioned
    /// paths there, so both forms must route. Nesting the version after the module path instead
    /// (`/block_info/v0/...`) silently 404s the canonical versioned URLs.
    #[tokio::test]
    async fn versioned_and_unversioned_module_paths_route() {
        for uri in [
            "/block_info/builderaddress",
            "/v0/block_info/builderaddress",
            "/txn_submit/status/abc",
            "/v0/txn_submit/status/abc",
            "/block_info/healthcheck",
            "/txn_submit/healthcheck",
            "/v0/block_info/healthcheck",
            "/v0/txn_submit/healthcheck",
        ] {
            assert_ne!(
                request(Method::GET, uri).await.status(),
                HttpStatusCode::NOT_FOUND,
                "{uri} did not route"
            );
        }
        assert_eq!(
            request(Method::GET, "/block_info/v0/builderaddress")
                .await
                .status(),
            HttpStatusCode::NOT_FOUND,
            "the version belongs before the module path, not after"
        );
    }

    /// A non-numeric integer path parameter must produce a 400 carrying the [`Error`] envelope,
    /// like the legacy `Integer` params did. Left to axum's own `Path` rejection, the body would
    /// be plain text and clients decoding the envelope on error would fail to parse.
    #[tokio::test]
    async fn bad_integer_params_are_enveloped() {
        let hash = TaggedBase64::new("HASH", &[0; 32]).unwrap();
        let uri = format!("/block_info/availableblocks/{hash}/not-a-number/snd/sig");
        let resp = request(Method::GET, &uri).await;
        assert_eq!(resp.status(), HttpStatusCode::BAD_REQUEST, "{uri}");
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let err: Error = serde_json::from_slice(&body)
            .unwrap_or_else(|_| panic!("error body is not the wire envelope: {body:?}"));
        assert!(matches!(
            err,
            Error::Request(RequestError::IncorrectParamType { .. })
        ));
    }

    /// Like the legacy server, every response carries permissive CORS headers; builder clients
    /// are not browsers, but demo tooling and dashboards are.
    #[tokio::test]
    async fn responses_carry_cors_headers() {
        for (method, uri) in [
            (Method::GET, "/healthcheck"),
            (Method::GET, "/v0/block_info/builderaddress"),
            (Method::GET, "/no/such/route"),
        ] {
            let resp = request(method.clone(), uri).await;
            assert_eq!(
                resp.headers()
                    .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                    .unwrap_or_else(|| panic!("no CORS header on {uri}")),
                "*",
                "{uri}"
            );
        }
    }
}
