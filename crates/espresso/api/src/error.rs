//! Typed API errors for proper HTTP/gRPC status code mapping

use std::fmt;

/// API error types that can be downcast at the HTTP/gRPC boundary
#[derive(Debug)]
pub enum ApiError {
    /// Client provided invalid input (maps to 400 Bad Request / INVALID_ARGUMENT)
    BadRequest(anyhow::Error),
    /// Handler failed for any reason (maps to 500 Internal Server Error / INTERNAL)
    Internal(anyhow::Error),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::BadRequest(err) => write!(f, "{}", err),
            ApiError::Internal(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for ApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ApiError::BadRequest(err) => err.source(),
            ApiError::Internal(err) => err.source(),
        }
    }
}
