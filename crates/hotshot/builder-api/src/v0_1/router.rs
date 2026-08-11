// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Axum routers serving the v0.1 builder API wire protocol.
//!
//! Route paths, status codes and the wire error type ([`Error`]) match the legacy `block_info`
//! and `txn_submit` modules, so existing builder clients keep working unchanged.

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
use hotshot_types::{
    data::VidCommitment,
    traits::{node_implementation::NodeType, signature_key::SignatureKey},
    utils::BuilderCommitment,
};
use http_wire::{self as wire, DecodeFailure, body_limit_layer, cors_layer, healthcheck_response};
use serde::de::DeserializeOwned;
use tagged_base64::TaggedBase64;

use super::{
    block_info::{
        AvailableBlockData, AvailableBlockHeaderInputV1, AvailableBlockHeaderInputV2,
        AvailableBlockInfo,
    },
    builder::{Error, RequestError, TransactionStatus},
    data_source::{AcceptsTxnSubmits, BuilderDataSource},
};

fn decode_body<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> Result<T, RequestError> {
    wire::decode_body(headers, body).map_err(|failure| match failure {
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

async fn available_blocks<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((parent_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<Vec<AvailableBlockInfo<Types>>, Error> = async {
        let hash = parse_hash_param::<VidCommitment>(&parent_hash, "parent_hash")?;
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
    wire::respond::<Error, _>(&headers, result)
}

async fn claim_block<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<AvailableBlockData<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
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
    wire::respond::<Error, _>(&headers, result)
}

async fn claim_block_with_num_nodes<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature, num_nodes)): Path<(
        String,
        u64,
        String,
        String,
        usize,
    )>,
) -> Response {
    let result: Result<AvailableBlockData<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
        let (sender, signature) = parse_sender_signature::<Types>(&sender, &signature)?;
        state
            .claim_block_with_num_nodes(&hash, view_number, sender, &signature, num_nodes)
            .await
            .map_err(|source| Error::BlockClaim {
                source,
                resource: hash.to_string(),
            })
    }
    .await;
    wire::respond::<Error, _>(&headers, result)
}

async fn claim_header_input<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<AvailableBlockHeaderInputV1<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
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
    wire::respond::<Error, _>(&headers, result)
}

async fn claim_header_input_v2<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
    Path((block_hash, view_number, sender, signature)): Path<(String, u64, String, String)>,
) -> Response {
    let result: Result<AvailableBlockHeaderInputV2<Types>, Error> = async {
        let hash = parse_hash_param::<BuilderCommitment>(&block_hash, "block_hash")?;
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
    wire::respond::<Error, _>(&headers, result)
}

async fn builder_address<Types: NodeType, S: BuilderDataSource<Types>>(
    State(state): State<S>,
    headers: HeaderMap,
) -> Response {
    let result = state.builder_address().await.map_err(Error::from);
    wire::respond::<Error, _>(&headers, result)
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
    wire::respond::<Error, _>(&headers, result)
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
    wire::respond::<Error, _>(&headers, result)
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
    wire::respond::<Error, _>(&headers, result)
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
        .with_state(state)
}

/// Wraps module routers with the app-level `healthcheck`, a `/v0` mirror of every module (the
/// legacy server served each module both unversioned and under its major version), a request
/// body limit, and permissive CORS headers.
pub fn app(api: Router) -> Router {
    Router::new()
        .route(
            "/healthcheck",
            get(|headers: HeaderMap| async move { healthcheck_response(&headers) }),
        )
        .merge(api.clone())
        .nest("/v0", api)
        .layer(body_limit_layer())
        .layer(cors_layer())
}
