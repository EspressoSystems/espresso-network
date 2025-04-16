// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::path::PathBuf;

use clap::Args;
use committable::Committable;
use derive_more::From;
use futures::FutureExt;
use hotshot_types::{traits::node_implementation::NodeType, utils::BuilderCommitment};
use serde::{Deserialize, Serialize};
use tagged_base64::TaggedBase64;
use thiserror::Error;
use tide_disco::{api::ApiError, method::ReadState, Api, RequestError, RequestParams, StatusCode};
use vbs::version::StaticVersionType;

use super::{
    block_info::AvailableBlockHeaderInputV2,
    data_source::{AcceptsTxnSubmits, BuilderDataSource},
    Version,
};
use crate::api::load_api;

#[derive(Args, Default)]
pub struct Options {
    #[arg(long = "builder-api-path", env = "HOTSHOT_BUILDER_API_PATH")]
    pub api_path: Option<PathBuf>,

    /// Additional API specification files to merge with `builder-api-path`.
    ///
    /// These optional files may contain route definitions for application-specific routes that have
    /// been added as extensions to the basic builder API.
    #[arg(
        long = "builder-extension",
        env = "HOTSHOT_BUILDER_EXTENSIONS",
        value_delimiter = ','
    )]
    pub extensions: Vec<toml::Value>,
}

#[derive(Clone, Debug, Error, Deserialize, Serialize)]
pub enum BuildError {
    #[error("The requested resource does not exist or is not known to this builder service")]
    NotFound,
    #[error("The requested resource exists but is not currently available")]
    Missing,
    #[error("Error trying to fetch the requested resource: {0}")]
    Error(String),
}

/// Enum to keep track on status of a transaction
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Sequenced { leaf: u64 },
    Rejected { reason: String }, // Rejection reason is in the String format
    Unknown,
}

#[derive(Clone, Debug, Error, Deserialize, Serialize)]
pub enum Error {
    #[error("Error processing request: {0}")]
    Request(#[from] RequestError),
    #[error("Error building block from {resource}: {source}")]
    BlockAvailable {
        source: BuildError,
        resource: String,
    },
    #[error("Error claiming block {resource}: {source}")]
    BlockClaim {
        source: BuildError,
        resource: String,
    },
    #[error("Error unpacking transactions: {0}")]
    TxnUnpack(RequestError),
    #[error("Error submitting transaction: {0}")]
    TxnSubmit(BuildError),
    #[error("Error getting builder address: {0}")]
    BuilderAddress(#[from] BuildError),
    #[error("Error getting transaction status: {0}")]
    TxnStat(BuildError),
    #[error("Custom error {status}: {message}")]
    Custom { message: String, status: StatusCode },
}

impl tide_disco::error::Error for Error {
    fn catch_all(status: StatusCode, msg: String) -> Self {
        Error::Custom {
            message: msg,
            status,
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Error::Request { .. } => StatusCode::BAD_REQUEST,
            Error::BlockAvailable { source, .. } | Error::BlockClaim { source, .. } => match source
            {
                BuildError::NotFound => StatusCode::NOT_FOUND,
                BuildError::Missing => StatusCode::NOT_FOUND,
                BuildError::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Error::TxnUnpack { .. } => StatusCode::BAD_REQUEST,
            Error::TxnSubmit { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Custom { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::BuilderAddress { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::TxnStat { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub(crate) fn try_extract_param<T: for<'a> TryFrom<&'a TaggedBase64>>(
    params: &RequestParams,
    param_name: &str,
) -> Result<T, Error> {
    params
        .param(param_name)?
        .as_tagged_base64()?
        .try_into()
        .map_err(|_| Error::Custom {
            message: format!("Invalid {param_name}"),
            status: StatusCode::UNPROCESSABLE_ENTITY,
        })
}

pub fn define_api<State, Types: NodeType>(
    options: &Options,
) -> Result<Api<State, Error, Version>, ApiError>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + BuilderDataSource<Types>,
{
    let mut api = load_api::<State, Error, Version>(
        options.api_path.as_ref(),
        include_str!("../../api/v0_1/builder.toml"),
        options.extensions.clone(),
    )?;
    api.with_version("0.1.0".parse().unwrap())
        .get("available_blocks", |req, state| {
            async move {
                let hash = req.blob_param("parent_hash")?;
                let view_number = req.integer_param("view_number")?;
                let signature = try_extract_param(&req, "signature")?;
                let sender = try_extract_param(&req, "sender")?;
                state
                    .available_blocks(&hash, view_number, sender, &signature)
                    .await
                    .map_err(|source| Error::BlockAvailable {
                        source,
                        resource: hash.to_string(),
                    })
            }
            .boxed()
        })?
        .get("claim_block", |req, state| {
            async move {
                let block_hash: BuilderCommitment = req.blob_param("block_hash")?;
                let view_number = req.integer_param("view_number")?;
                let signature = try_extract_param(&req, "signature")?;
                let sender = try_extract_param(&req, "sender")?;
                state
                    .claim_block(&block_hash, view_number, sender, &signature)
                    .await
                    .map_err(|source| Error::BlockClaim {
                        source,
                        resource: block_hash.to_string(),
                    })
            }
            .boxed()
        })?
        .get("claim_block_with_num_nodes", |req, state| {
            async move {
                let block_hash: BuilderCommitment = req.blob_param("block_hash")?;
                let view_number = req.integer_param("view_number")?;
                let signature = try_extract_param(&req, "signature")?;
                let sender = try_extract_param(&req, "sender")?;
                let num_nodes = req.integer_param("num_nodes")?;
                state
                    .claim_block_with_num_nodes(
                        &block_hash,
                        view_number,
                        sender,
                        &signature,
                        num_nodes,
                    )
                    .await
                    .map_err(|source| Error::BlockClaim {
                        source,
                        resource: block_hash.to_string(),
                    })
            }
            .boxed()
        })?
        .get("claim_header_input", |req, state| {
            async move {
                let block_hash: BuilderCommitment = req.blob_param("block_hash")?;
                let view_number = req.integer_param("view_number")?;
                let signature = try_extract_param(&req, "signature")?;
                let sender = try_extract_param(&req, "sender")?;
                state
                    .claim_block_header_input(&block_hash, view_number, sender, &signature)
                    .await
                    .map_err(|source| Error::BlockClaim {
                        source,
                        resource: block_hash.to_string(),
                    })
            }
            .boxed()
        })?
        .get("claim_header_input_v2", |req, state| {
            async move {
                let block_hash: BuilderCommitment = req.blob_param("block_hash")?;
                let view_number = req.integer_param("view_number")?;
                let signature = try_extract_param(&req, "signature")?;
                let sender = try_extract_param(&req, "sender")?;
                let out = state
                    .claim_block_header_input(&block_hash, view_number, sender, &signature)
                    .await
                    .map_err(|source| Error::BlockClaim {
                        source,
                        resource: block_hash.to_string(),
                    });

                out.map(|input| AvailableBlockHeaderInputV2::<Types> {
                    fee_signature: input.fee_signature,
                    sender: input.sender,
                })
            }
            .boxed()
        })?
        .get("builder_address", |_req, state| {
            async move { state.builder_address().await.map_err(|e| e.into()) }.boxed()
        })?;
    Ok(api)
}

pub fn submit_api<State, Types: NodeType, Ver: StaticVersionType + 'static>(
    options: &Options,
) -> Result<Api<State, Error, Ver>, ApiError>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + AcceptsTxnSubmits<Types>,
{
    let mut api = load_api::<State, Error, Ver>(
        options.api_path.as_ref(),
        include_str!("../../api/v0_1/submit.toml"),
        options.extensions.clone(),
    )?;
    api.with_version("0.0.1".parse().unwrap())
        .at("submit_txn", |req: RequestParams, state| {
            async move {
                let tx = req
                    .body_auto::<<Types as NodeType>::Transaction, Ver>(Ver::instance())
                    .map_err(Error::TxnUnpack)?;
                let hash = tx.commit();
                state
                    .read(|state| state.submit_txns(vec![tx]))
                    .await
                    .map_err(Error::TxnSubmit)?;
                Ok(hash)
            }
            .boxed()
        })?
        .at("submit_batch", |req: RequestParams, state| {
            async move {
                let txns = req
                    .body_auto::<Vec<<Types as NodeType>::Transaction>, Ver>(Ver::instance())
                    .map_err(Error::TxnUnpack)?;
                let hashes = txns.iter().map(|tx| tx.commit()).collect::<Vec<_>>();
                state
                    .read(|state| state.submit_txns(txns))
                    .await
                    .map_err(Error::TxnSubmit)?;
                Ok(hashes)
            }
            .boxed()
        })?
        .get("get_status", |req: RequestParams, state| {
            async move {
                let tx = req
                    .body_auto::<<Types as NodeType>::Transaction, Ver>(Ver::instance())
                    .map_err(Error::TxnUnpack)?;
                let hash = tx.commit();
                state.txn_status(hash).await.map_err(Error::TxnStat)
            }
            .boxed()
        })?;
    Ok(api)
}
