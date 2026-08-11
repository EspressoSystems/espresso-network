use derive_more::From;
use disco_types::{request::RequestError, status::StatusCode};
use serde::{Deserialize, Serialize};
use snafu::Snafu;

/// The axum server for this API; client-only consumers skip the server stack by leaving the
/// `server` feature off.
#[cfg(feature = "server")]
mod server;
#[cfg(feature = "server")]
pub use server::*;

#[derive(Clone, Debug, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum EventError {
    /// The requested resource does not exist or is not known to this hotshot node.
    NotFound,
    /// The requested resource exists but is not currently available.
    Missing,
    /// There was an error while trying to fetch the requested resource.
    #[snafu(display("Failed to fetch requested resource: {message}"))]
    Error { message: String },
}

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: RequestError,
    },
    #[snafu(display("error receiving events {resource}: {source}"))]
    #[from(ignore)]
    EventAvailable {
        source: EventError,
        resource: String,
    },
    Custom {
        message: String,
        status: StatusCode,
    },
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
            Error::EventAvailable { source, .. } => match source {
                EventError::NotFound => StatusCode::NOT_FOUND,
                EventError::Missing => StatusCode::NOT_FOUND,
                EventError::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Error::Custom { status, .. } => *status,
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
