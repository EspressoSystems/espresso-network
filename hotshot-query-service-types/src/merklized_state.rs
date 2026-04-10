use derive_more::From;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tide_disco::StatusCode;

use crate::QueryError;

/// Errors surfaced to clients from a Merklized state API.
#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: tide_disco::RequestError,
    },
    #[snafu(display("{source}"))]
    Query {
        source: QueryError,
    },
    #[snafu(display("error {status}: {message}"))]
    Custom {
        message: String,
        status: StatusCode,
    },
}

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Request { .. } => StatusCode::BAD_REQUEST,
            Self::Query { source, .. } => source.status(),
            Self::Custom { status, .. } => *status,
        }
    }
}
