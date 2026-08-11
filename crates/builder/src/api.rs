//! Axum port of the builder's `block_info` and `txn_submit` tide-disco modules
//! (`hotshot_builder_api::v0_1::builder::{define_api, submit_api}`).
//!
//! Route paths, status codes and the wire error type (`BuilderApiError`) are taken directly from
//! `hotshot-builder-api`'s handler bodies, so the node's `BuilderClient`
//! (`crates/hotshot/task-impls/src/builder.rs`), which is unmodified, keeps working against this
//! server unchanged.

use std::sync::Arc;

use axum::{
    Router,
    body::Bytes,
    extract::{Path, State},
    http::HeaderMap,
    response::Response,
    routing::{get, post},
};
use committable::Committable;
use disco_types::status::StatusCode;
use espresso_types::SeqTypes;
use hotshot_builder_api::v0_1::{
    block_info::{
        AvailableBlockData, AvailableBlockHeaderInputV1, AvailableBlockHeaderInputV2,
        AvailableBlockInfo,
    },
    builder::{Error as BuilderApiError, RequestError, TransactionStatus},
    data_source::{AcceptsTxnSubmits, BuilderDataSource},
};
use hotshot_builder_legacy::service::ProxyGlobalState;
use hotshot_types::{
    data::VidCommitment,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    utils::BuilderCommitment,
};
use http_wire::{self as wire, DecodeFailure, body_limit_layer, cors_layer, healthcheck_response};
use serde::de::DeserializeOwned;
use tagged_base64::TaggedBase64;

type SharedState = Arc<ProxyGlobalState<SeqTypes>>;

fn decode_body<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> Result<T, RequestError> {
    wire::decode_body(headers, body).map_err(|failure| match failure {
        DecodeFailure::Json(_) => RequestError::Json,
        DecodeFailure::Binary(_) => RequestError::Binary,
        DecodeFailure::UnsupportedContentType => RequestError::UnsupportedContentType,
    })
}

fn tb64_request_error(field: &str) -> BuilderApiError {
    BuilderApiError::Request(RequestError::TaggedBase64 {
        reason: format!("invalid tagged base64 for {field}"),
    })
}

/// Parses a hash-type path parameter (`parent_hash`, `block_hash`, `transaction_hash`). Any
/// failure is an `Error::Request`, which `BuilderApiError::status()` maps to 400, like tide's
/// `blob_param` path did for these params.
fn parse_hash_param<T>(value: &str, field: &str) -> Result<T, BuilderApiError>
where
    T: for<'a> TryFrom<&'a TaggedBase64>,
{
    let tb64: TaggedBase64 = value.parse().map_err(|_| tb64_request_error(field))?;
    T::try_from(&tb64).map_err(|_| tb64_request_error(field))
}

/// Parses a key/signature path parameter, mirroring `try_extract_param`: a wrong-type value is a
/// `Custom` error carrying 422.
fn parse_key_param<T>(value: &str, field: &str) -> Result<T, BuilderApiError>
where
    T: for<'a> TryFrom<&'a TaggedBase64>,
{
    let tb64: TaggedBase64 = value.parse().map_err(|_| tb64_request_error(field))?;
    T::try_from(&tb64).map_err(|_| BuilderApiError::Custom {
        message: format!("Invalid {field}"),
        status: StatusCode::UNPROCESSABLE_ENTITY,
    })
}

type Sender = <SeqTypes as NodeType>::SignatureKey;
type Signature = <Sender as SignatureKey>::PureAssembledSignatureType;

fn parse_sender_signature(
    sender: &str,
    signature: &str,
) -> Result<(Sender, Signature), BuilderApiError> {
    let sender = parse_key_param::<Sender>(sender, "sender")?;
    let signature = parse_key_param::<Signature>(signature, "signature")?;
    Ok((sender, signature))
}

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

async fn available_blocks(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((parent_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<Vec<AvailableBlockInfo<_>>, BuilderApiError> = async {
        let hash = parse_hash_param::<VidCommitment>(&parent_hash, "parent_hash")?;
        let (sender, signature) = parse_sender_signature(&sender, &signature)?;
        state
            .available_blocks(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| BuilderApiError::BlockAvailable {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn claim_block(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<AvailableBlockData<_>, BuilderApiError> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let (sender, signature) = parse_sender_signature(&sender, &signature)?;
        state
            .claim_block(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| BuilderApiError::BlockClaim {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn claim_block_with_num_nodes(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature, num_nodes)): Path<(
        String,
        u64,
        String,
        String,
        usize,
    )>,
) -> Response {
    let result: Result<AvailableBlockData<_>, BuilderApiError> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let (sender, signature) = parse_sender_signature(&sender, &signature)?;
        state
            .claim_block_with_num_nodes(&hash, view_number, sender, &signature, num_nodes)
            .await
            .map_err(|source| BuilderApiError::BlockClaim {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn claim_header_input(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<AvailableBlockHeaderInputV1<_>, BuilderApiError> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let (sender, signature) = parse_sender_signature(&sender, &signature)?;
        state
            .claim_block_header_input(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| BuilderApiError::BlockClaim {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn claim_header_input_v2(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<AvailableBlockHeaderInputV2<SeqTypes>, BuilderApiError> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let (sender, signature) = parse_sender_signature(&sender, &signature)?;
        let input = state
            .claim_block_header_input(&hash, view_number, sender, &signature)
            .await
            .map_err(|source| BuilderApiError::BlockClaim {
                source,
                resource: hash.to_string(),
            })?;
        Ok(AvailableBlockHeaderInputV2 {
            fee_signature: input.fee_signature,
            sender: input.sender,
        })
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn builder_address(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    let result = state.builder_address().await.map_err(BuilderApiError::from);
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn submit_txn(State(state): State<SharedState>, headers: HeaderMap, body: Bytes) -> Response {
    let result = async {
        let tx: <SeqTypes as NodeType>::Transaction =
            decode_body(&headers, &body).map_err(BuilderApiError::TxnUnpack)?;
        let hash = tx.commit();
        state
            .submit_txns(vec![tx])
            .await
            .map_err(BuilderApiError::TxnSubmit)?;
        Ok(hash)
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn submit_batch(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let result = async {
        let txns: Vec<<SeqTypes as NodeType>::Transaction> =
            decode_body(&headers, &body).map_err(BuilderApiError::TxnUnpack)?;
        let hashes = txns.iter().map(|tx| tx.commit()).collect::<Vec<_>>();
        state
            .submit_txns(txns)
            .await
            .map_err(BuilderApiError::TxnSubmit)?;
        Ok(hashes)
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

async fn get_status(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path(transaction_hash): Path<String>,
) -> Response {
    let result: Result<TransactionStatus, BuilderApiError> = async {
        let hash = parse_hash_param(&transaction_hash, "transaction_hash")?;
        state
            .txn_status(hash)
            .await
            .map_err(BuilderApiError::TxnStat)
    }
    .await;
    wire::respond::<BuilderApiError, _>(&headers, result)
}

fn block_info_router(state: SharedState) -> Router {
    Router::new()
        .route(
            "/availableblocks/{parent_hash}/{view_number}/{sender}/{signature}",
            get(available_blocks),
        )
        .route(
            "/claimblock/{block_hash}/{view_number}/{sender}/{signature}",
            get(claim_block),
        )
        .route(
            "/claimblockwithnumnodes/{block_hash}/{view_number}/{sender}/{signature}/{num_nodes}",
            get(claim_block_with_num_nodes),
        )
        .route(
            "/claimheaderinput/{block_hash}/{view_number}/{sender}/{signature}",
            get(claim_header_input),
        )
        .route(
            "/claimheaderinput/v2/{block_hash}/{view_number}/{sender}/{signature}",
            get(claim_header_input_v2),
        )
        .route("/builderaddress", get(builder_address))
        .with_state(state)
}

fn txn_submit_router(state: SharedState) -> Router {
    Router::new()
        .route("/submit", post(submit_txn))
        .route("/batch", post(submit_batch))
        .route("/status/{transaction_hash}", get(get_status))
        .with_state(state)
}

/// Builds the full router: `healthcheck`, plus `block_info` and `txn_submit`, served both
/// unversioned and under `/v0` (both modules were registered with API version major `0`, tide's
/// convention for the module's only registered major version). Like tide-disco, every response
/// carries permissive CORS headers.
pub fn router(state: ProxyGlobalState<SeqTypes>) -> Router {
    let state: SharedState = Arc::new(state);
    let api = Router::new()
        .nest("/block_info", block_info_router(state.clone()))
        .nest("/txn_submit", txn_submit_router(state));
    Router::new()
        .route("/healthcheck", get(healthcheck))
        .merge(api.clone())
        .nest("/v0", api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use async_lock::RwLock;
    use axum::http::{Method, Request, StatusCode as AxumStatusCode, header};
    use hotshot_builder_legacy::service::GlobalState;
    use hotshot_types::{data::ViewNumber, traits::signature_key::BuilderSignatureKey as _};

    use super::*;

    fn test_router() -> Router {
        let (bootstrap_sender, _bootstrap_receiver) = async_broadcast::broadcast(10);
        let (tx_sender, _tx_receiver) = async_broadcast::broadcast(10);
        let keys =
            <SeqTypes as NodeType>::BuilderSignatureKey::generated_from_seed_indexed([0; 32], 0);
        router(ProxyGlobalState::new(
            Arc::new(RwLock::new(GlobalState::new(
                bootstrap_sender,
                tx_sender,
                VidCommitment::default(),
                ViewNumber::new(0),
                ViewNumber::new(0),
                Duration::from_secs(60),
                1024,
                1,
                1,
            ))),
            keys,
            Duration::from_millis(100),
        ))
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

    /// tide-disco served both modules at `/v0/<module>/...` and redirected the unversioned paths
    /// there, so both forms must route. Nesting the version after the module path instead
    /// (`/block_info/v0/...`) silently 404s the canonical versioned URLs.
    #[tokio::test]
    async fn versioned_and_unversioned_module_paths_route() {
        for uri in [
            "/block_info/builderaddress",
            "/v0/block_info/builderaddress",
            "/txn_submit/status/abc",
            "/v0/txn_submit/status/abc",
        ] {
            assert_ne!(
                request(Method::GET, uri).await.status(),
                AxumStatusCode::NOT_FOUND,
                "{uri} did not route"
            );
        }
        assert_eq!(
            request(Method::GET, "/block_info/v0/builderaddress")
                .await
                .status(),
            AxumStatusCode::NOT_FOUND,
            "the version belongs before the module path, not after"
        );
    }

    /// Like tide-disco, every response carries permissive CORS headers; the node's builder client
    /// is not a browser, but the demo tooling and dashboards are.
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
