//! Typed API errors for proper HTTP/gRPC status code mapping

use std::fmt;

use thiserror::Error;

/// Marker errors for availability endpoint failures. These are wrapped in `anyhow::Error` by the
/// state implementation and downcasted in the Axum handlers to select the right HTTP status code.
#[derive(Debug, Error)]
pub enum AvailabilityError {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    RangeExceeded(String),
    #[error("{0}")]
    BadRequest(String),
}

/// API error types that can be downcast at the HTTP/gRPC boundary
#[derive(Debug)]
pub enum ApiError {
    /// Client provided invalid input (maps to 400 Bad Request / INVALID_ARGUMENT)
    BadRequest(anyhow::Error),
    /// Requested resource does not exist (maps to 404 Not Found)
    NotFound(anyhow::Error),
    /// Handler failed for any reason (maps to 500 Internal Server Error / INTERNAL)
    Internal(anyhow::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::BadRequest(err) | ApiError::NotFound(err) | ApiError::Internal(err) => {
                write!(f, "{}", err)
            },
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApiError::BadRequest(err) | ApiError::NotFound(err) | ApiError::Internal(err) => {
                err.source()
            },
        }
    }
}
