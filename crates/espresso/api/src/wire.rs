//! Tide-disco-compatible content negotiation, shared by the axum ports of the tide-disco
//! services (builder, state relay server, orchestrator, light client query service).
//!
//! Each service keeps its own wire error envelope (the exact `Serialize` shape its existing
//! clients decode) and describes it to these helpers with a [`WireFormat`] impl; the negotiation
//! logic itself (`Accept`/`Content-Type` handling, VBS vs JSON, serialization fallbacks) lives
//! only here.

use axum::{
    Json,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::{Serialize, de::DeserializeOwned};
use vbs::{BinarySerializer, Serializer, version::StaticVersionType};

/// Describes a service's wire protocol: its VBS framing version and its error envelope.
pub trait WireFormat {
    /// The service's wire error envelope, serialized in the negotiated format like any body.
    type Error: Serialize;
    /// VBS framing version for binary bodies.
    type Version: StaticVersionType;

    /// The HTTP status an error response is sent with.
    fn status(err: &Self::Error) -> StatusCode;

    /// The error reported when a response body fails to VBS-serialize.
    fn serialize_failure(message: String) -> Self::Error;
}

/// Whether the request negotiates VBS binary responses, matching tide-disco: surf-disco clients
/// default to `Accept: application/octet-stream`; everything else (browsers, curl) gets JSON.
pub fn wants_binary(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/octet-stream"))
}

/// Encode a successful response body, negotiating VBS binary vs JSON from the `Accept` header.
pub fn encode_ok<C: WireFormat, T: Serialize>(headers: &HeaderMap, value: T) -> Response {
    if wants_binary(headers) {
        match Serializer::<C::Version>::serialize(&value) {
            Ok(bytes) => {
                ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
            },
            Err(err) => encode_err::<C>(headers, C::serialize_failure(err.to_string())),
        }
    } else {
        Json(value).into_response()
    }
}

/// Encode an error response using the same content negotiation as [`encode_ok`].
pub fn encode_err<C: WireFormat>(headers: &HeaderMap, err: C::Error) -> Response {
    let status = C::status(&err);
    if wants_binary(headers) {
        match Serializer::<C::Version>::serialize(&err) {
            Ok(bytes) => (
                status,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response(),
            Err(_) => (status, Json(err)).into_response(),
        }
    } else {
        (status, Json(err)).into_response()
    }
}

pub fn respond<C: WireFormat, T: Serialize>(
    headers: &HeaderMap,
    result: Result<T, C::Error>,
) -> Response {
    match result {
        Ok(value) => encode_ok::<C, _>(headers, value),
        Err(err) => encode_err::<C>(headers, err),
    }
}

/// Why a request body failed to decode; each service maps this into its own wire error type.
#[derive(Debug)]
pub enum DecodeFailure {
    /// The body was `application/json` but did not parse.
    Json(String),
    /// The body was `application/octet-stream` but did not VBS-deserialize.
    Binary(String),
    /// The `Content-Type` was missing or not one of the two supported types.
    UnsupportedContentType,
}

/// Decode a request body, matching tide-disco's `body_auto`: VBS for `application/octet-stream`,
/// JSON for `application/json`. Content types are prefix-matched so parameters (e.g. a charset)
/// are tolerated.
pub fn decode_body<Ver: StaticVersionType, T: DeserializeOwned>(
    headers: &HeaderMap,
    body: &[u8],
) -> Result<T, DecodeFailure> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if content_type.starts_with("application/octet-stream") {
        Serializer::<Ver>::deserialize(body).map_err(|e| DecodeFailure::Binary(e.to_string()))
    } else if content_type.starts_with("application/json") {
        serde_json::from_slice(body).map_err(|e| DecodeFailure::Json(e.to_string()))
    } else {
        Err(DecodeFailure::UnsupportedContentType)
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use serde::Deserialize;
    use vbs::version::StaticVersion;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestError {
        status: u16,
        message: String,
    }

    struct TestFormat;

    impl WireFormat for TestFormat {
        type Error = TestError;
        type Version = StaticVersion<0, 1>;

        fn status(err: &Self::Error) -> StatusCode {
            StatusCode::from_u16(err.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
        }

        fn serialize_failure(message: String) -> Self::Error {
            TestError {
                status: 500,
                message,
            }
        }
    }

    fn headers(name: header::HeaderName, value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_str(value).unwrap());
        headers
    }

    async fn body_bytes(resp: Response) -> Vec<u8> {
        axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    #[tokio::test]
    async fn encode_negotiates_binary_and_json() {
        let binary = headers(header::ACCEPT, "application/octet-stream");
        let resp = encode_ok::<TestFormat, _>(&binary, 42u64);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/octet-stream"
        );
        let bytes = body_bytes(resp).await;
        let decoded: u64 = Serializer::<StaticVersion<0, 1>>::deserialize(&bytes).unwrap();
        assert_eq!(decoded, 42);

        let resp = encode_ok::<TestFormat, _>(&HeaderMap::new(), 42u64);
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(String::from_utf8(body_bytes(resp).await).unwrap(), "42");
    }

    #[tokio::test]
    async fn errors_carry_status_and_envelope() {
        let err = TestError {
            status: 404,
            message: "no such thing".into(),
        };
        let resp = encode_err::<TestFormat>(&HeaderMap::new(), err);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let decoded: TestError = serde_json::from_slice(&body_bytes(resp).await).unwrap();
        assert_eq!(decoded.message, "no such thing");

        let binary = headers(header::ACCEPT, "application/octet-stream");
        let err = TestError {
            status: 404,
            message: "no such thing".into(),
        };
        let resp = encode_err::<TestFormat>(&binary, err);
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let decoded: TestError =
            Serializer::<StaticVersion<0, 1>>::deserialize(&body_bytes(resp).await).unwrap();
        assert_eq!(decoded.status, 404);
    }

    #[test]
    fn decode_matches_content_type_by_prefix() {
        let json = headers(header::CONTENT_TYPE, "application/json; charset=utf-8");
        let decoded: u64 = decode_body::<StaticVersion<0, 1>, _>(&json, b"42").unwrap();
        assert_eq!(decoded, 42);

        let binary = headers(header::CONTENT_TYPE, "application/octet-stream");
        let bytes = Serializer::<StaticVersion<0, 1>>::serialize(&42u64).unwrap();
        let decoded: u64 = decode_body::<StaticVersion<0, 1>, _>(&binary, &bytes).unwrap();
        assert_eq!(decoded, 42);

        assert!(matches!(
            decode_body::<StaticVersion<0, 1>, u64>(&HeaderMap::new(), b"42"),
            Err(DecodeFailure::UnsupportedContentType)
        ));
        assert!(matches!(
            decode_body::<StaticVersion<0, 1>, u64>(&json, b"not json"),
            Err(DecodeFailure::Json(_))
        ));
        assert!(matches!(
            decode_body::<StaticVersion<0, 1>, u64>(&binary, b""),
            Err(DecodeFailure::Binary(_))
        ));
    }
}
