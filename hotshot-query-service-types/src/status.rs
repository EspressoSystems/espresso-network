use std::fmt::Display;

use derive_more::From;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tide_disco::{RequestError, StatusCode};

/// Error exposed to clients of the status API.
#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
pub enum Error {
    Request { source: RequestError },
    Internal { reason: String },
}

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Request { .. } => StatusCode::BAD_REQUEST,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn internal<M: Display>(msg: M) -> Self {
        Error::Internal {
            reason: msg.to_string(),
        }
    }
}
