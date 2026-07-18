use std::fmt::Display;

use derive_more::From;
use disco_types::status::StatusCode;
#[cfg(feature = "web")]
use http_client::ClientError;
use serde::{Deserialize, Serialize};
use snafu::Snafu;

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

impl disco_types::error::Error for Error {
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

/// Mirrors the `disco_types::error::Error` impl above, converting between
/// `disco_types::status::StatusCode` and `reqwest::StatusCode` (the wire status carried by
/// `http_client`).
#[cfg(feature = "web")]
impl ClientError for Error {
    fn status(&self) -> http_client::StatusCode {
        let status = match self {
            Self::Availability { source } => source.status(),
            Self::Node { source } => source.status(),
            Self::Status { source } => source.status(),
            Self::MerklizedState { source } => source.status(),
            Self::Explorer { source } => source.status(),
            Self::Custom { status, .. } => *status,
        };
        status.into()
    }

    fn catch_all(status: http_client::StatusCode, message: String) -> Self {
        Self::Custom {
            message,
            status: status.into(),
        }
    }
}

/// Here we converge the events service error type into the API error type
#[cfg(feature = "web")]
impl From<hotshot_events_service::events::Error> for Error {
    fn from(err: hotshot_events_service::events::Error) -> Self {
        Self::Custom {
            message: err.to_string(),
            status: disco_types::error::Error::status(&err),
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
