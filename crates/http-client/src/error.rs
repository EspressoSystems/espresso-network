use std::fmt;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Errors that can be serialized into a response body.
///
/// Application error types implement this trait so that failures returned by an API can be
/// carried across the wire and reconstructed by the client. Errors that don't downcast to `Self`
/// (for example, errors from infrastructure in front of the server) are represented using
/// [`catch_all`](ClientError::catch_all).
pub trait ClientError:
    std::error::Error + DeserializeOwned + Serialize + Send + Sync + 'static
{
    /// The status code this error should be reported with.
    fn status(&self) -> StatusCode;

    /// Construct an error from an arbitrary status code and message, for errors that don't
    /// otherwise have a structured representation.
    fn catch_all(status: StatusCode, message: String) -> Self;
}

/// Serialize a [`StatusCode`] as its numeric value, matching the wire format used by
/// `tide_disco::error::ServerError`.
mod status_as_u16 {
    use reqwest::StatusCode;
    use serde::{Deserialize, Deserializer, Serializer, de::Error as _};

    pub fn serialize<S: Serializer>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u16(status.as_u16())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<StatusCode, D::Error> {
        let code = u16::deserialize(deserializer)?;
        StatusCode::from_u16(code).map_err(D::Error::custom)
    }
}

/// The simplest implementation of [`ClientError`].
///
/// Its serialized shape (`{"status": <u16>, "message": <string>}`) matches
/// `tide_disco::error::ServerError`, so it can deserialize error bodies produced by servers built
/// on `tide_disco`.
///
/// [`Display`](fmt::Display) renders as `"Error {status}: {message}"` with a numeric status.
/// This is a deliberate simplification of tide's format, which spells out the canonical reason
/// (`"Error 404: Not Found: not found"`); nothing parses the `Display` output.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub struct ClientErr {
    #[serde(with = "status_as_u16")]
    pub status: StatusCode,
    pub message: String,
}

impl fmt::Display for ClientErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error {}: {}", self.status.as_u16(), self.message)
    }
}

impl ClientError for ClientErr {
    fn status(&self) -> StatusCode {
        self.status
    }

    fn catch_all(status: StatusCode, message: String) -> Self {
        Self { status, message }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn serializes_like_tide_disco_server_error() {
        let err = ClientErr::catch_all(StatusCode::NOT_FOUND, "not found".to_owned());
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"status": 404, "message": "not found"})
        );
    }

    #[test]
    fn round_trips_through_the_literal_wire_shape() {
        let body = r#"{"status":404,"message":"not found"}"#;
        let err: ClientErr = serde_json::from_str(body).unwrap();
        assert_eq!(
            err,
            ClientErr::catch_all(StatusCode::NOT_FOUND, "not found".to_owned())
        );
        assert_eq!(serde_json::to_string(&err).unwrap(), body);
    }

    #[test]
    fn display_uses_simplified_numeric_status_format() {
        // Intentionally simpler than tide's "Error 400: Bad Request: invalid body".
        let err = ClientErr::catch_all(StatusCode::BAD_REQUEST, "invalid body".to_owned());
        assert_eq!(err.to_string(), "Error 400: invalid body");
    }
}
