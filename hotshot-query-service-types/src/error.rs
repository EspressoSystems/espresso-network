use std::fmt::Display;

use derive_more::From;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use surf_disco::StatusCode;
use tide_disco::Error as _;

use crate::{availability, explorer, merklized_state, node, status};

/// External error type surfaced to clients of the API.
#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
pub enum Error {
    #[snafu(display("{source}"))]
    Availability { source: availability::Error },
    #[snafu(display("{source}"))]
    Node { source: node::Error },
    #[snafu(display("{source}"))]
    Status { source: status::Error },
    #[snafu(display("{source}"))]
    MerklizedState { source: merklized_state::Error },
    #[snafu(display("{source}"))]
    Explorer {
        #[serde(rename = "error")]
        source: explorer::Error,
    },
    #[snafu(display("error {status}: {message}"))]
    Custom { message: String, status: StatusCode },
}

impl Error {
    pub fn internal<M: Display>(message: M) -> Self {
        Self::Custom {
            message: message.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl surf_disco::Error for Error {
    fn catch_all(status: StatusCode, message: String) -> Self {
        Self::Custom { status, message }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Availability { source } => source.status(),
            Self::Node { source } => source.status(),
            Self::Status { source } => source.status(),
            Self::MerklizedState { source } => source.status(),
            Self::Explorer { source } => source.status(),
            Self::Custom { status, .. } => *status,
        }
    }
}

/// Here we converge the events service error type into the `tide-disco` error type
impl From<hotshot_events_service::events::Error> for Error {
    fn from(err: hotshot_events_service::events::Error) -> Self {
        Self::Custom {
            message: err.to_string(),
            status: err.status(),
        }
    }
}

/// An internal error that arises when querying a database.
#[derive(Clone, Debug, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum QueryError {
    /// The requested resource does not exist or is not known to this query service.
    NotFound,
    /// The requested resource exists but is not currently available.
    ///
    /// In most cases a missing resource can be recovered from DA.
    Missing,
    /// There was an error while trying to fetch the requested resource.
    #[snafu(display("Failed to fetch requested resource: {message}"))]
    #[snafu(context(suffix(ErrorSnafu)))]
    Error { message: String },
}

impl QueryError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::NotFound | Self::Missing => StatusCode::NOT_FOUND,
            Self::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type QueryResult<T> = Result<T, QueryError>;
