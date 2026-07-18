//! Axum port of the builder's `block_info` and `txn_submit` tide-disco modules
//! (`hotshot_builder_api::v0_1::builder::{define_api, submit_api}`).
//!
//! Route paths, status codes and the wire error type (`BuilderApiError`) are taken directly from
//! `hotshot-builder-api`'s handler bodies, so the node's `BuilderClient`
//! (`crates/hotshot/task-impls/src/builder.rs`), which is unmodified, keeps working against this
//! server unchanged.

use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode as AxumStatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use committable::Committable;
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
use http_client::healthcheck::HealthStatus;
use serde::{Serialize, de::DeserializeOwned};
use tagged_base64::TaggedBase64;
use tide_disco::{Error as _, StatusCode};
use vbs::{BinarySerializer, Serializer, version::StaticVersion};

/// Binary framing version for VBS-negotiated responses, matching `hotshot_builder_api::v0_1`'s
/// framing, which is what `BuilderClient` sends/expects.
type WireVersion = StaticVersion<0, 1>;

type SharedState = Arc<ProxyGlobalState<espresso_types::SeqTypes>>;

fn wants_binary(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/octet-stream"))
}

/// Maps tide-disco's `StatusCode` (what `BuilderApiError::status()` returns) onto axum's.
fn wire_status(status: StatusCode) -> AxumStatusCode {
    AxumStatusCode::from_u16(u16::from(status)).unwrap_or(AxumStatusCode::INTERNAL_SERVER_ERROR)
}

fn encode_ok<T: Serialize>(headers: &HeaderMap, value: T) -> Response {
    if wants_binary(headers) {
        match Serializer::<WireVersion>::serialize(&value) {
            Ok(bytes) => {
                ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
            },
            Err(err) => encode_err(
                headers,
                BuilderApiError::Custom {
                    message: err.to_string(),
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                },
            ),
        }
    } else {
        Json(value).into_response()
    }
}

fn encode_err(headers: &HeaderMap, err: BuilderApiError) -> Response {
    let status = wire_status(err.status());
    if wants_binary(headers) {
        match Serializer::<WireVersion>::serialize(&err) {
            Ok(bytes) => (
                status,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response(),
            Err(_) => (status, Json(err)).into_response(),
        }
    } else {
        (status, Json(err)).into_response()
    }
}

fn respond<T: Serialize>(headers: &HeaderMap, result: Result<T, BuilderApiError>) -> Response {
    match result {
        Ok(v) => encode_ok(headers, v),
        Err(e) => encode_err(headers, e),
    }
}

/// Decodes a request body the way tide-disco's `body_auto` did: VBS for
/// `application/octet-stream`, JSON for `application/json`.
fn decode_body<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> Result<T, RequestError> {
    match headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        Some("application/json") => serde_json::from_slice(body).map_err(|_| RequestError::Json),
        Some("application/octet-stream") => {
            Serializer::<WireVersion>::deserialize(body).map_err(|_| RequestError::Binary)
        },
        _ => Err(RequestError::UnsupportedContentType),
    }
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
/// `Custom` error carrying 422. Note `BuilderApiError::status()` returns 500 for every `Custom`,
/// so the wire status is 500, as it was under tide.
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

type Sender = <espresso_types::SeqTypes as NodeType>::SignatureKey;
type Signature = <Sender as SignatureKey>::PureAssembledSignatureType;

fn parse_sender_signature(
    sender: &str,
    signature: &str,
) -> Result<(Sender, Signature), BuilderApiError> {
    let sender = parse_key_param::<Sender>(sender, "sender")?;
    let signature = parse_key_param::<Signature>(signature, "signature")?;
    Ok((sender, signature))
}

/// Tide-disco-compatible singleton-app healthcheck: a bare [`HealthStatus`], so
/// `BuilderClient::connect` can decode it in both JSON and binary form.
async fn healthcheck(headers: HeaderMap) -> Response {
    encode_ok(&headers, HealthStatus::Available)
}

// --- block_info -----------------------------------------------------------------------------

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
    respond(&headers, result)
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
    respond(&headers, result)
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
    respond(&headers, result)
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
    respond(&headers, result)
}

async fn claim_header_input_v2(
    State(state): State<SharedState>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<AvailableBlockHeaderInputV2<espresso_types::SeqTypes>, BuilderApiError> =
        async {
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
    respond(&headers, result)
}

async fn builder_address(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    let result = state.builder_address().await.map_err(BuilderApiError::from);
    respond(&headers, result)
}

// --- txn_submit -------------------------------------------------------------------------------

async fn submit_txn(State(state): State<SharedState>, headers: HeaderMap, body: Bytes) -> Response {
    let result = async {
        let tx: <espresso_types::SeqTypes as NodeType>::Transaction =
            decode_body(&headers, &body).map_err(BuilderApiError::TxnUnpack)?;
        let hash = tx.commit();
        state
            .submit_txns(vec![tx])
            .await
            .map_err(BuilderApiError::TxnSubmit)?;
        Ok(hash)
    }
    .await;
    respond(&headers, result)
}

async fn submit_batch(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let result = async {
        let txns: Vec<<espresso_types::SeqTypes as NodeType>::Transaction> =
            decode_body(&headers, &body).map_err(BuilderApiError::TxnUnpack)?;
        let hashes = txns.iter().map(|tx| tx.commit()).collect::<Vec<_>>();
        state
            .submit_txns(txns)
            .await
            .map_err(BuilderApiError::TxnSubmit)?;
        Ok(hashes)
    }
    .await;
    respond(&headers, result)
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
    respond(&headers, result)
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
/// convention for the module's only registered major version).
pub fn router(state: ProxyGlobalState<espresso_types::SeqTypes>) -> Router {
    let state: SharedState = Arc::new(state);
    let block_info = block_info_router(state.clone());
    let txn_submit = txn_submit_router(state);
    Router::new()
        .route("/healthcheck", get(healthcheck))
        .nest("/block_info", block_info.clone())
        .nest("/block_info/v0", block_info)
        .nest("/txn_submit", txn_submit.clone())
        .nest("/txn_submit/v0", txn_submit)
}
