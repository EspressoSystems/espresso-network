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
                // Both transports render handler errors through this impl (v1 via the axum
                // handlers, v2 via `to_status`), so a provider credential in the message is
                // removed once, here.
                f.write_str(&espresso_utils::redact::scrub(&err.to_string()))
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

/// Errors raised as [`AvailabilityError`] by a state implementation carry semantic meaning;
/// anything else is a failure the client cannot act on. Both transports classify through here,
/// so v1 and v2 cannot drift on what counts as a 404.
pub fn classify(err: anyhow::Error) -> ApiError {
    match err.downcast_ref::<AvailabilityError>() {
        Some(AvailabilityError::NotFound(_)) => ApiError::NotFound(err),
        Some(_) => ApiError::BadRequest(err),
        None => ApiError::Internal(err),
    }
}

/// Render a handler error for the v2 transports: the gRPC `grpc-message` trailer and, via
/// `tonic_rest::RestError`, the REST error body. Classification and the credential scrub happen
/// in [`classify`] and [`ApiError`]'s `Display`, shared with the v1 path.
pub fn to_status(err: anyhow::Error) -> tonic::Status {
    let err = classify(err);
    let message = err.to_string();
    match err {
        ApiError::NotFound(_) => tonic::Status::not_found(message),
        ApiError::BadRequest(_) => tonic::Status::invalid_argument(message),
        ApiError::Internal(_) => tonic::Status::internal(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ported from the deleted `api/src/tonic.rs` when the v2 adapter stopped rendering through
    /// [`ApiError`]. The token endpoints inline the L1 provider's error, URL and API key included,
    /// and both the `grpc-message` trailer and the REST body reach unauthenticated callers.
    #[test]
    fn to_status_scrubs_provider_credentials() {
        let err = anyhow::anyhow!(
            r#"failed to get total supply: reqwest::Error {{ url: "https://u:p@rpc.invalid/v1/FAKEKEY" }}"#
        );

        let status = to_status(err);

        assert_eq!(status.code(), tonic::Code::Internal);
        assert!(
            !status.message().contains("FAKEKEY"),
            "{}",
            status.message()
        );
        assert!(!status.message().contains("u:p"), "{}", status.message());
        assert!(
            status.message().contains("rpc.invalid"),
            "{}",
            status.message()
        );
    }

    /// A 404 has to stay a 404 through the classifier, since the scrub sits on the same path.
    #[test]
    fn availability_not_found_maps_to_not_found() {
        let status = to_status(anyhow::Error::new(AvailabilityError::NotFound(
            "leaf 7".to_string(),
        )));
        assert_eq!(status.code(), tonic::Code::NotFound);
    }
}
