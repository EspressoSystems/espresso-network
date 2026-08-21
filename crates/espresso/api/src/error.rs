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
<<<<<<< HEAD
            ApiError::Internal(err) => write!(f, "{}", err),
||||||| parent of 355c72eab8f (fix(telemetry): stop logging L1 provider credentials (#4783))
            ApiError::BadRequest(err) | ApiError::NotFound(err) | ApiError::Internal(err) => {
                write!(f, "{}", err)
            },
=======
            ApiError::BadRequest(err) | ApiError::NotFound(err) | ApiError::Internal(err) => {
                // Both the axum and tonic adapters render an error into a response body through
                // this impl, so a provider credential in the message is removed once, here.
                f.write_str(&espresso_utils::redact::scrub(&err.to_string()))
            },
>>>>>>> 355c72eab8f (fix(telemetry): stop logging L1 provider credentials (#4783))
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
