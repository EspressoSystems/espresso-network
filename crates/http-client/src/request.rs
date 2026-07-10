use std::error::Error as _;

use serde::{Serialize, de::DeserializeOwned};
use vbs::{BinarySerializer, Serializer, version::StaticVersionType};

use crate::error::ClientError;

#[must_use]
#[derive(Debug)]
pub struct Request<T, E, VER: StaticVersionType> {
    inner: reqwest::RequestBuilder,
    marker: std::marker::PhantomData<fn(T, E, VER)>,
}

impl<T, E, VER: StaticVersionType> From<reqwest::RequestBuilder> for Request<T, E, VER> {
    fn from(inner: reqwest::RequestBuilder) -> Self {
        Self {
            inner,
            marker: Default::default(),
        }
    }
}

impl<T: DeserializeOwned, E: ClientError, VER: StaticVersionType> Request<T, E, VER> {
    /// Set a header on the request.
    pub fn header(self, key: &str, value: &str) -> Self {
        self.inner.header(key, value).into()
    }

    /// Set the request body using JSON.
    ///
    /// Body is serialized using [`serde_json`] and the `Content-Type` header is set to
    /// `application/json`.
    pub fn body_json<B: Serialize>(self, body: &B) -> Result<Self, E> {
        Ok(self
            .header("Content-Type", "application/json")
            .inner
            .body(serde_json::to_string(body).map_err(request_error)?)
            .into())
    }

    /// Set the request body using [`vbs`].
    ///
    /// Body is serialized using [`vbs::Serializer`] and the `Content-Type` header is set to
    /// `application/octet-stream`.
    ///
    /// # Errors
    ///
    /// Fails if `body` does not serialize successfully.
    pub fn body_binary<B: Serialize>(self, body: &B) -> Result<Self, E> {
        Ok(self
            .header("Content-Type", "application/octet-stream")
            .inner
            .body(Serializer::<VER>::serialize(body).map_err(request_error)?)
            .into())
    }

    /// Send the request and await a response from the server.
    ///
    /// If the request succeeds (receives a response with [`StatusCode::OK`](reqwest::StatusCode::OK))
    /// the response body is converted to a `T`, using a format determined by the `Content-Type`
    /// header of the response.
    ///
    /// # Errors
    ///
    /// If the client is unable to reach the server, or if the response body cannot be interpreted
    /// as a `T`, an error is synthesized with [`E::catch_all`](ClientError::catch_all).
    ///
    /// If the request completes but the response status code is not `OK`, an error is constructed
    /// from the body of the response: if the body deserializes to an `E` (using the content type
    /// specified in the response), that `E` is returned directly; otherwise a `catch_all` error is
    /// synthesized that includes human-readable information about the response.
    pub async fn send(self) -> Result<T, E> {
        let res = self.inner.send().await.map_err(reqwest_error)?;
        let status = res.status();
        let content_type = res.headers().get("Content-Type").cloned();
        if status == reqwest::StatusCode::OK {
            match &content_type {
                Some(content_type) => match content_type.to_str() {
                    Ok("application/json") => res.json().await.map_err(reqwest_error),
                    Ok("application/octet-stream") => {
                        Serializer::<VER>::deserialize(&res.bytes().await.map_err(reqwest_error)?)
                            .map_err(request_error)
                    },
                    to_str_result => {
                        let msg = body_debug_string(res).await;
                        Err(E::catch_all(
                            reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                            format!("unsupported content type {to_str_result:?} {msg}"),
                        ))
                    },
                },
                None => Err(E::catch_all(
                    reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "unspecified content type in response".into(),
                )),
            }
        } else {
            let bytes = match res.bytes().await {
                Ok(bytes) => bytes,
                Err(err) => {
                    return Err(E::catch_all(
                        status,
                        format!(
                            "Request terminated with error {status}. Failed to read request body \
                             due to {err}",
                        ),
                    ));
                },
            };
            if let Some(content_type) = content_type.as_ref().and_then(|c| c.to_str().ok()) {
                match content_type {
                    "application/json" => {
                        if let Ok(err) = serde_json::from_slice(&bytes) {
                            return Err(err);
                        }
                    },
                    "application/octet-stream" => {
                        if let Ok(err) = Serializer::<VER>::deserialize(&bytes) {
                            return Err(err);
                        }
                    },
                    _ => {},
                }
            }
            if let Ok(msg) = std::str::from_utf8(&bytes) {
                return Err(E::catch_all(status, msg.to_string()));
            }
            Err(E::catch_all(
                status,
                format!(
                    "Request terminated with error {status}. Content-Type: {}. Body: 0x{}",
                    content_type
                        .as_ref()
                        .and_then(|c| c.to_str().ok())
                        .unwrap_or("unspecified"),
                    hex::encode(&bytes),
                ),
            ))
        }
    }

    /// Send the request and return the full response body as raw bytes.
    pub async fn bytes(self) -> Result<Vec<u8>, E> {
        let res = self.inner.send().await.map_err(reqwest_error)?;
        let status = res.status();
        let content_type = res.headers().get("Content-Type").cloned();

        let bytes = res.bytes().await.map_err(|err| {
            E::catch_all(
                status,
                format!(
                    "Request terminated with error {status}. Failed to read request body due to \
                     {err}",
                ),
            )
        })?;

        if status.is_success() {
            return Ok(bytes.to_vec());
        }

        if let Ok(msg) = std::str::from_utf8(&bytes) {
            return Err(E::catch_all(status, msg.to_string()));
        }

        Err(E::catch_all(
            status,
            format!(
                "Request failed with status {status}. Content-Type: {}. Body: 0x{}",
                content_type
                    .as_ref()
                    .and_then(|c| c.to_str().ok())
                    .unwrap_or("unspecified"),
                hex::encode(&bytes),
            ),
        ))
    }
}

/// Read the response body for inclusion in an error message about an unexpected content type.
async fn body_debug_string(res: reqwest::Response) -> String {
    match res.bytes().await {
        Ok(bytes) => match std::str::from_utf8(&bytes) {
            Ok(s) => format!("body: {s}"),
            Err(_) => format!("body: {}", hex::encode(&bytes)),
        },
        Err(_) => String::default(),
    }
}

fn request_error<E: ClientError>(source: impl std::fmt::Display) -> E {
    E::catch_all(reqwest::StatusCode::BAD_REQUEST, source.to_string())
}

fn reqwest_error<E: ClientError>(source: reqwest::Error) -> E {
    E::catch_all(
        source
            .status()
            .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR),
        reqwest_error_msg(source),
    )
}

pub(crate) fn reqwest_error_msg(err: reqwest::Error) -> String {
    match err.source() {
        Some(inner) => format!("{err}: {inner}"),
        None => err.to_string(),
    }
}

#[cfg(test)]
mod test {
    use vbs::version::StaticVersion;

    use super::*;
    use crate::{client::Client, error::ClientErr};

    type Ver01 = StaticVersion<0, 1>;

    #[test]
    fn joins_relative_route_onto_base_url() {
        let client = Client::<ClientErr, Ver01>::new("http://example.com/api/".parse().unwrap());
        let built = client.get::<()>("foo/bar").inner.build().unwrap();
        assert_eq!(built.url().as_str(), "http://example.com/api/foo/bar");
    }

    #[test]
    fn absolute_route_ignores_base_url_path() {
        let client = Client::<ClientErr, Ver01>::new("http://example.com/api/".parse().unwrap());
        let built = client.get::<()>("/absolute/path").inner.build().unwrap();
        assert_eq!(built.url().as_str(), "http://example.com/absolute/path");
    }

    #[test]
    fn body_binary_double_serializes_a_pre_serialized_buffer_stably() {
        // `body_binary` must serialize its argument as-is: callers double-encode a
        // pre-serialized `Vec<u8>` (e.g. a VID share) and rely on the outer envelope being
        // exactly [4-byte version][8-byte bincode length][inner bytes].
        let inner = Serializer::<Ver01>::serialize(&"hello".to_string()).unwrap();

        let client = Client::<ClientErr, Ver01>::new("http://example.com/".parse().unwrap());
        let built = client
            .post::<()>("route")
            .body_binary(&inner)
            .unwrap()
            .inner
            .build()
            .unwrap();
        let body = built.body().unwrap().as_bytes().unwrap();

        assert_eq!(&body[0..2], 0u16.to_le_bytes().as_slice(), "major version");
        assert_eq!(&body[2..4], 1u16.to_le_bytes().as_slice(), "minor version");
        assert_eq!(
            &body[4..12],
            (inner.len() as u64).to_le_bytes().as_slice(),
            "bincode length prefix for the inner Vec<u8>",
        );
        assert_eq!(&body[12..], inner.as_slice());
    }
}
