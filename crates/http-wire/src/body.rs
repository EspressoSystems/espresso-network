use http::StatusCode;
use serde::{Serialize, de::DeserializeOwned};
use vbs::{
    BinarySerializer, Serializer,
    version::{StaticVersion, StaticVersionType, Version},
};

use crate::{content_type::ContentType, error::WireError};

/// The VBS framing version every encode and decode helper here speaks, on both the server and
/// the client side.
///
/// The API's own versioning lives in the URL (`/v1`, `/v2`); the framing version has always
/// been 0.1 and is shared rather than re-declared per service.
pub type WireVersion = StaticVersion<0, 1>;

/// Why a body failed to decode.
///
/// Variants carry the bare codec error message and `Display` adds no prefix, unlike
/// [`EncodeFailure`]: decode call sites compose their own context, and several of those
/// messages are historic client-visible text that must not gain a prefix.
#[derive(Debug, thiserror::Error)]
pub enum DecodeFailure {
    /// The body was `application/json` but did not parse.
    #[error("{0}")]
    Json(String),
    /// The body was `application/octet-stream` but did not VBS-deserialize.
    #[error("{0}")]
    Binary(String),
    /// The `Content-Type` was missing or not one of the two supported types.
    #[error("unsupported content type")]
    UnsupportedContentType,
}

/// Why a body failed to encode.
#[derive(Debug, thiserror::Error)]
pub enum EncodeFailure {
    #[error("invalid JSON serialization: {0}")]
    Json(String),
    #[error("invalid binary serialization: {0}")]
    Binary(String),
}

/// Encode a body in the given format: VBS for [`Binary`](ContentType::Binary), JSON for
/// [`Json`](ContentType::Json).
pub fn encode_body<Ver: StaticVersionType, T: Serialize + ?Sized>(
    content_type: ContentType,
    value: &T,
) -> Result<Vec<u8>, EncodeFailure> {
    match content_type {
        ContentType::Binary => Serializer::<Ver>::serialize(value)
            .map_err(|err| EncodeFailure::Binary(err.to_string())),
        ContentType::Json => {
            serde_json::to_vec(value).map_err(|err| EncodeFailure::Json(err.to_string()))
        },
    }
}

/// Decode a body in the format named by a `Content-Type` header value: VBS for
/// `application/octet-stream`, JSON for `application/json`.
///
/// The header is matched by media-type essence (see [`ContentType::parse`]), so parameters are
/// tolerated.
pub(crate) fn decode_body<Ver: StaticVersionType, T: DeserializeOwned>(
    content_type: Option<&str>,
    body: &[u8],
) -> Result<T, DecodeFailure> {
    match content_type.and_then(ContentType::parse) {
        Some(ContentType::Binary) => decode_binary::<Ver, T>(body),
        Some(ContentType::Json) => {
            serde_json::from_slice(body).map_err(|err| DecodeFailure::Json(err.to_string()))
        },
        None => Err(DecodeFailure::UnsupportedContentType),
    }
}

/// Decode a VBS payload like [`vbs::Serializer::deserialize`], but through
/// [`versions::deserialize_limited`] since the bytes come from a peer.
pub(crate) fn decode_binary<Ver, T>(bytes: &[u8]) -> Result<T, DecodeFailure>
where
    Ver: StaticVersionType,
    T: DeserializeOwned,
{
    let (version, rest) =
        Version::deserialize(bytes).map_err(|err| DecodeFailure::Binary(err.to_string()))?;
    if version != Ver::version() {
        // The text, swapped arguments included, is the one `vbs` returns, which clients see.
        return Err(DecodeFailure::Binary(format!(
            "Version Mismatch! Expected {version}, got {}",
            Ver::version()
        )));
    }
    versions::deserialize_limited(rest).map_err(|err| DecodeFailure::Binary(err.to_string()))
}

/// Decode the client-side view of a complete response.
///
/// A [`StatusCode::OK`] body decodes to a `T` in the format named by the response's
/// `Content-Type`. Any other status decodes the body to the typed error envelope `E` when it
/// carries one, falling back to [`catch_all`](WireError::catch_all) with a human-readable
/// rendering of the body.
pub fn decode_response<Ver: StaticVersionType, T: DeserializeOwned, E: WireError>(
    status: StatusCode,
    content_type: Option<&str>,
    body: &[u8],
) -> Result<T, E> {
    if status == StatusCode::OK {
        return decode_body::<Ver, T>(content_type, body).map_err(|failure| match failure {
            DecodeFailure::UnsupportedContentType => match content_type {
                None => E::catch_all(
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "unspecified content type in response".into(),
                ),
                Some(content_type) => E::catch_all(
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    format!(
                        "unsupported content type {content_type:?} {}",
                        body_debug(body)
                    ),
                ),
            },
            // The 500/400 split keeps the statuses clients have always seen for a `200 OK`
            // whose body doesn't decode.
            DecodeFailure::Json(msg) => E::catch_all(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("invalid JSON body: {msg}"),
            ),
            DecodeFailure::Binary(msg) => E::catch_all(
                StatusCode::BAD_REQUEST,
                format!("invalid binary body: {msg}"),
            ),
        });
    }
    if let Ok(err) = decode_body::<Ver, E>(content_type, body) {
        return Err(err);
    }
    if let Ok(msg) = std::str::from_utf8(body) {
        return Err(E::catch_all(status, msg.to_string()));
    }
    Err(E::catch_all(
        status,
        format!(
            "Request terminated with error {status}. Content-Type: {}. Body: 0x{}",
            content_type.unwrap_or("unspecified"),
            hex::encode(body),
        ),
    ))
}

/// Render a response body for inclusion in an error message about an unexpected content type.
fn body_debug(body: &[u8]) -> String {
    match std::str::from_utf8(body) {
        Ok(s) => format!("body: {s}"),
        Err(_) => format!("body: {}", hex::encode(body)),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    type Ver = StaticVersion<0, 1>;

    #[test]
    fn binary_rejects_deeply_nested_input() {
        #[derive(Debug, Deserialize)]
        #[expect(dead_code, reason = "only ever decoded")]
        enum Nest {
            End,
            Inner(Box<Nest>),
        }

        let end = 0u32.to_le_bytes();
        let inner = 1u32.to_le_bytes();
        let mut bytes = Version::serialize(&Ver::version());
        bytes.extend(inner.repeat(1_000));
        bytes.extend(end);

        match decode_binary::<Ver, Nest>(&bytes) {
            Err(DecodeFailure::Binary(err)) => assert!(err.contains("nested more than"), "{err}"),
            other => panic!("expected a binary decode failure, got {other:?}"),
        }
    }

    #[test]
    fn binary_version_mismatch_reads_like_vbs() {
        let bytes = Serializer::<StaticVersion<0, 2>>::serialize(&0u8).unwrap();
        let vbs_err = Serializer::<Ver>::deserialize::<u8>(&bytes).unwrap_err();

        match decode_binary::<Ver, u8>(&bytes) {
            Err(DecodeFailure::Binary(err)) => assert_eq!(err, vbs_err.to_string()),
            other => panic!("expected a binary decode failure, got {other:?}"),
        }
    }
}
