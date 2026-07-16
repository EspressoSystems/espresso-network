use std::{collections::HashMap, pin::Pin};

use futures::{
    Sink, Stream,
    task::{Context, Poll},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async_with_config,
    tungstenite::{
        Error as WsError, Message,
        client::IntoClientRequest,
        http::{self, HeaderName, HeaderValue},
        protocol::WebSocketConfig,
    },
};
use url::Url;
use vbs::{BinarySerializer, Serializer, version::StaticVersionType};

use crate::{client::ContentType, error::ClientError};

type ConnectStream = MaybeTlsStream<TcpStream>;

const MAX_REDIRECTS: usize = 10;

#[must_use]
#[derive(Debug)]
pub struct SocketRequest<E, VER: StaticVersionType> {
    url: Url,
    content_type: ContentType,
    headers: HashMap<String, Vec<String>>,
    config: Option<WebSocketConfig>,
    marker: std::marker::PhantomData<fn(E, VER)>,
}

impl<E: ClientError, VER: StaticVersionType> SocketRequest<E, VER> {
    pub(crate) fn new(
        mut url: Url,
        content_type: ContentType,
        config: Option<WebSocketConfig>,
    ) -> Self {
        url.set_scheme(&socket_scheme(url.scheme())).unwrap();
        Self {
            url,
            content_type,
            headers: Default::default(),
            config,
            marker: Default::default(),
        }
    }

    /// Set a header on the request.
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers
            .entry(key.to_owned())
            .or_default()
            .push(value.to_owned());
        self
    }

    /// Start the WebSocket handshake and initiate a connection to the server.
    pub async fn connect<FromServer: DeserializeOwned, ToServer: Serialize + ?Sized>(
        mut self,
    ) -> Result<Connection<FromServer, ToServer, E, VER>, E> {
        // Follow redirects: tungstenite does not do this on its own.
        for _ in 0..MAX_REDIRECTS {
            // Build from the URI so the standard handshake headers (Host, Connection, Upgrade,
            // Sec-WebSocket-Version/Key) are filled in, then layer our own headers on top.
            let uri: http::Uri =
                self.url
                    .to_string()
                    .parse()
                    .map_err(|err: http::uri::InvalidUri| {
                        E::catch_all(reqwest::StatusCode::BAD_REQUEST, err.to_string())
                    })?;
            let mut req = uri
                .into_client_request()
                .map_err(|err| E::catch_all(reqwest::StatusCode::BAD_REQUEST, err.to_string()))?;
            for (key, values) in &self.headers {
                let name = HeaderName::from_bytes(key.as_bytes()).map_err(|err| {
                    E::catch_all(reqwest::StatusCode::BAD_REQUEST, err.to_string())
                })?;
                for value in values {
                    let value = HeaderValue::from_str(value).map_err(|err| {
                        E::catch_all(reqwest::StatusCode::BAD_REQUEST, err.to_string())
                    })?;
                    // `append` keeps the mandatory handshake headers intact; custom headers must
                    // not reuse their names (Host, Connection, Upgrade, Sec-WebSocket-Version/Key)
                    // or the handshake will send duplicates and fail.
                    req.headers_mut().append(name.clone(), value);
                }
            }

            let err = match connect_async_with_config(req, self.config, false).await {
                Ok((conn, _)) => return Ok(Connection::new(conn, self.content_type)),
                Err(err) => err,
            };
            if let WsError::Http(res) = &err
                && (301..=308).contains(&res.status().as_u16())
                && let Some(location) = res
                    .headers()
                    .get("location")
                    .and_then(|header| header.to_str().ok())
            {
                tracing::info!(from = %self.url, to = %location, "WS handshake following redirect");
                self.url.set_path(location);
                continue;
            }
            return Err(E::catch_all(
                reqwest::StatusCode::BAD_REQUEST,
                err.to_string(),
            ));
        }
        Err(E::catch_all(
            reqwest::StatusCode::BAD_REQUEST,
            format!("WS handshake exceeded {MAX_REDIRECTS} redirects"),
        ))
    }

    /// Initiate a unidirectional connection to the server.
    ///
    /// Equivalent to `self.connect()` with the `ToServer` message type replaced by
    /// [`Unsupported`], so callers don't need to specify a type parameter that isn't used.
    pub async fn subscribe<FromServer: DeserializeOwned>(
        self,
    ) -> Result<Connection<FromServer, Unsupported, E, VER>, E> {
        self.connect().await
    }
}

/// A bi-directional connection to a WebSocket server.
pub struct Connection<FromServer, ToServer: ?Sized, E, VER: StaticVersionType> {
    inner: WebSocketStream<ConnectStream>,
    content_type: ContentType,
    #[allow(clippy::type_complexity)]
    marker: std::marker::PhantomData<fn(FromServer, ToServer, E, VER)>,
}

impl<FromServer, ToServer: ?Sized, E, VER: StaticVersionType> std::fmt::Debug
    for Connection<FromServer, ToServer, E, VER>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("content_type", &self.content_type)
            .finish_non_exhaustive()
    }
}

impl<FromServer, ToServer: ?Sized, E, VER: StaticVersionType>
    Connection<FromServer, ToServer, E, VER>
{
    fn new(inner: WebSocketStream<ConnectStream>, content_type: ContentType) -> Self {
        Self {
            inner,
            content_type,
            marker: Default::default(),
        }
    }

    /// Project a `Pin<&mut Self>` to a pinned reference to the underlying connection.
    ///
    /// # Soundness
    ///
    /// This implements structural pinning for [`Connection`]: `Connection` is only [`Unpin`] if
    /// `inner` is, and no operation on this type moves `inner` out of a pinned `Connection`.
    fn pinned_inner(self: Pin<&mut Self>) -> Pin<&mut WebSocketStream<ConnectStream>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.inner) }
    }
}

impl<FromServer: DeserializeOwned, ToServer: ?Sized, E: ClientError, VER: StaticVersionType> Stream
    for Connection<FromServer, ToServer, E, VER>
{
    type Item = Result<FromServer, E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.pinned_inner().poll_next(cx) {
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(Err(err))) => match err {
                WsError::ConnectionClosed | WsError::AlreadyClosed => Poll::Ready(None),
                err => Poll::Ready(Some(Err(E::catch_all(
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                    err.to_string(),
                )))),
            },
            Poll::Ready(Some(Ok(msg))) => Poll::Ready(match msg {
                Message::Binary(bytes) => {
                    Some(Serializer::<VER>::deserialize(&bytes).map_err(|err| {
                        E::catch_all(
                            reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                            format!("invalid binary: {err}\n{bytes:?}"),
                        )
                    }))
                },
                Message::Text(s) => Some(serde_json::from_str(&s).map_err(|err| {
                    E::catch_all(
                        reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                        format!("invalid JSON: {err}\n{s}"),
                    )
                })),
                Message::Close(_) => None,
                _ => Some(Err(E::catch_all(
                    reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "unsupported WebSocket message".into(),
                ))),
            }),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<FromServer, ToServer: Serialize + ?Sized, E: ClientError, VER: StaticVersionType>
    Sink<&ToServer> for Connection<FromServer, ToServer, E, VER>
{
    type Error = E;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.pinned_inner().poll_ready(cx).map_err(|err| {
            E::catch_all(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                format!("error in WebSocket connection: {err}"),
            )
        })
    }

    fn start_send(self: Pin<&mut Self>, item: &ToServer) -> Result<(), Self::Error> {
        let msg = match self.content_type {
            ContentType::Binary => Message::Binary(
                Serializer::<VER>::serialize(item)
                    .map_err(|err| {
                        E::catch_all(
                            reqwest::StatusCode::BAD_REQUEST,
                            format!("invalid binary serialization: {err}"),
                        )
                    })?
                    .into(),
            ),
            ContentType::Json => Message::Text(
                serde_json::to_string(item)
                    .map_err(|err| {
                        E::catch_all(
                            reqwest::StatusCode::BAD_REQUEST,
                            format!("invalid JSON serialization: {err}"),
                        )
                    })?
                    .into(),
            ),
        };
        self.pinned_inner().start_send(msg).map_err(|err| {
            E::catch_all(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                format!("error sending WebSocket message: {err}"),
            )
        })
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.pinned_inner().poll_flush(cx).map_err(|err| {
            E::catch_all(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                format!("error in WebSocket connection: {err}"),
            )
        })
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.pinned_inner().poll_close(cx).map_err(|err| {
            E::catch_all(
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                format!("error in WebSocket connection: {err}"),
            )
        })
    }
}

/// Unconstructable type used to disable the [`Sink`] side of a [`Connection`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Unsupported {}

/// Get the scheme for a WebSockets connection upgraded from an existing stateless connection.
///
/// `scheme` is the scheme of the stateless connection, e.g. HTTP or HTTPS. If it has a known
/// WebSockets counterpart, e.g. WS or WSS, that is returned. Otherwise `scheme` is returned
/// unmodified, trusting the caller to know what they're doing.
fn socket_scheme(scheme: &str) -> String {
    match scheme {
        "http" => "ws",
        "https" => "wss",
        _ => scheme,
    }
    .to_string()
}
