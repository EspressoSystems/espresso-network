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
                // The v1 axum handlers render an error into a response body through this impl, so
                // a provider credential in the message is removed once, here. The v2 adapters do
                // not go through `ApiError`; they scrub in `to_status`.
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

/// What a handler error means, independent of transport. v1 maps this onto [`ApiError`] and an
/// HTTP status; v2 maps it onto a gRPC code in [`to_status`]. Both go through [`classify`] so the
/// two transports cannot drift on what counts as a 404.
pub enum ErrorKind {
    NotFound,
    BadRequest,
    Internal,
}

/// Errors raised as [`AvailabilityError`] by a state implementation carry semantic meaning;
/// anything else is a failure the client cannot act on.
pub fn classify(err: &anyhow::Error) -> ErrorKind {
    match err.downcast_ref::<AvailabilityError>() {
        Some(AvailabilityError::NotFound(_)) => ErrorKind::NotFound,
        Some(_) => ErrorKind::BadRequest,
        None => ErrorKind::Internal,
    }
}

/// Render a handler error for the v2 transports: the gRPC `grpc-message` trailer and, via
/// `tonic_rest::RestError`, the REST error body.
///
/// Both reach unauthenticated callers, and the v2 services hand up `anyhow::Error` directly
/// rather than through [`ApiError`], so the credential scrub that `ApiError`'s `Display` applies
/// to the v1 path has to happen here instead.
pub fn to_status(err: anyhow::Error) -> tonic::Status {
    let message = espresso_utils::redact::scrub(&err.to_string());
    match classify(&err) {
        ErrorKind::NotFound => tonic::Status::not_found(message),
        ErrorKind::BadRequest => tonic::Status::invalid_argument(message),
        ErrorKind::Internal => tonic::Status::internal(message),
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
            r#"failed to get total supply. err=reqwest::Error {{ url: "https://u:p@rpc.invalid/v1/FAKEKEY" }}"#
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
