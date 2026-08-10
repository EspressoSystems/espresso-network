// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

// `RequestError` is re-exported because it is embedded in this module's wire error type
// (`Error::Request`/`Error::TxnUnpack`); servers implementing this API need to construct it.
pub use disco_types::request::RequestError;
use disco_types::status::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

impl disco_types::error::Error for Error {
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
            Error::Custom { status, .. } => *status,
            Error::BuilderAddress { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::TxnStat { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl http_client::ClientError for Error {
    fn catch_all(status: http_client::StatusCode, msg: String) -> Self {
        Error::Custom {
            message: msg,
            status: status.into(),
        }
    }

    fn status(&self) -> http_client::StatusCode {
        disco_types::error::Error::status(self).into()
    }
}
