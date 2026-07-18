use std::{
    fmt,
    marker::PhantomData,
    time::{Duration, Instant},
};

use serde::de::DeserializeOwned;
use tokio::time::sleep;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use url::Url;
use vbs::version::StaticVersionType;

use crate::{
    error::ClientError,
    healthcheck::HealthCheck,
    request::{Request, reqwest_error_msg},
    socket::SocketRequest,
};

/// Content types supported by a `tide_disco`-shaped API.
#[derive(Clone, Copy, Debug)]
pub enum ContentType {
    Json,
    Binary,
}

impl ContentType {
    fn mime(self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Binary => "application/octet-stream",
        }
    }
}

/// A client of an HTTP/WebSocket application.
pub struct Client<E, VER: StaticVersionType> {
    inner: reqwest::Client,
    base_url: Url,
    accept: ContentType,
    _marker: PhantomData<fn(E, VER)>,
}

impl<E, VER: StaticVersionType> Clone for Client<E, VER> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            base_url: self.base_url.clone(),
            accept: self.accept,
            _marker: PhantomData,
        }
    }
}

impl<E, VER: StaticVersionType> fmt::Debug for Client<E, VER> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("base_url", &self.base_url)
            .field("accept", &self.accept)
            .finish()
    }
}

impl<E: ClientError, VER: StaticVersionType> Client<E, VER> {
    /// Create a client for the application at `base_url`.
    pub fn new(base_url: Url) -> Self {
        Self::builder(base_url).build()
    }

    /// Create a client with customization.
    pub fn builder(base_url: Url) -> ClientBuilder<E, VER> {
        ClientBuilder::<E, VER>::new(base_url)
    }

    /// Connect to the server, retrying if the server is not running.
    ///
    /// It is not necessary to call this before making requests: the client connects lazily.
    /// This is useful to wait for the server to come up if it may be offline when the client is
    /// created.
    ///
    /// Polls the server's `/healthcheck` endpoint every 10 seconds until it returns
    /// [`StatusCode::OK`](reqwest::StatusCode::OK), or until `timeout` elapses (or forever, if
    /// `timeout` is `None`).
    pub async fn connect(&self, timeout: Option<Duration>) -> bool {
        let deadline = timeout.map(|d| Instant::now() + d);
        while deadline.map(|t| Instant::now() < t).unwrap_or(true) {
            match self
                .inner
                // Absolute path: always probes the application's top-level healthcheck,
                // regardless of any submodule prefix in `base_url`. Matches surf-disco.
                .get(self.base_url.join("/healthcheck").unwrap())
                .send()
                .await
            {
                Ok(res) if res.status() == reqwest::StatusCode::OK => return true,
                Ok(res) => {
                    tracing::info!(
                        url = %self.base_url,
                        status = %res.status(),
                        "waiting for server to become ready",
                    );
                },
                Err(err) => {
                    tracing::info!(
                        url = %self.base_url,
                        err = reqwest_error_msg(err),
                        "waiting for server to become ready",
                    );
                },
            }
            sleep(Duration::from_secs(10)).await;
        }
        false
    }

    /// Connect to the server, retrying until the server is `healthy`.
    ///
    /// Similar to [`connect`](Self::connect), but continues retrying after the first successful
    /// `/healthcheck` response until it satisfies the `healthy` predicate.
    ///
    /// Returns the response from `/healthcheck` on success, or `None` on timeout.
    pub async fn wait_for_health<H: DeserializeOwned + HealthCheck>(
        &self,
        healthy: impl Fn(&H) -> bool,
        timeout: Option<Duration>,
    ) -> Option<H> {
        let deadline = timeout.map(|d| Instant::now() + d);
        while deadline.map(|t| Instant::now() < t).unwrap_or(true) {
            match self.healthcheck::<H>().await {
                Ok(health) if healthy(&health) => return Some(health),
                _ => sleep(Duration::from_secs(10)).await,
            }
        }
        None
    }

    /// Build an HTTP `GET` request.
    pub fn get<T: DeserializeOwned>(&self, route: &str) -> Request<T, E, VER> {
        self.request(reqwest::Method::GET, route)
    }

    /// Build an HTTP `POST` request.
    pub fn post<T: DeserializeOwned>(&self, route: &str) -> Request<T, E, VER> {
        self.request(reqwest::Method::POST, route)
    }

    /// Query the server's healthcheck endpoint.
    pub async fn healthcheck<H: DeserializeOwned + HealthCheck>(&self) -> Result<H, E> {
        self.get("healthcheck").send().await
    }

    /// Build an HTTP request with the specified method.
    pub fn request<T: DeserializeOwned>(
        &self,
        method: reqwest::Method,
        route: &str,
    ) -> Request<T, E, VER> {
        Request::from(
            self.inner
                .request(method, self.base_url.join(route).unwrap()),
        )
        .header("Accept", self.accept.mime())
    }

    /// Build a streaming connection request.
    ///
    /// # Panics
    ///
    /// Panics if `route`, joined with the base URL, is not a valid URL.
    pub fn socket(&self, route: &str) -> SocketRequest<E, VER> {
        SocketRequest::new(self.base_url.join(route).unwrap(), self.accept, None)
            .header("Accept", self.accept.mime())
    }

    /// Build a streaming connection request using a custom [`WebSocketConfig`].
    ///
    /// # Panics
    ///
    /// Panics if `route`, joined with the base URL, is not a valid URL.
    pub fn socket_with_config(
        &self,
        route: &str,
        config: WebSocketConfig,
    ) -> SocketRequest<E, VER> {
        SocketRequest::new(
            self.base_url.join(route).unwrap(),
            self.accept,
            Some(config),
        )
        .header("Accept", self.accept.mime())
    }

    /// Create a client for a sub-module of the connected application.
    pub fn module<E2: ClientError>(
        &self,
        prefix: &str,
    ) -> Result<Client<E2, VER>, url::ParseError> {
        Ok(Client {
            inner: self.inner.clone(),
            base_url: self.base_url.join(prefix)?,
            accept: self.accept,
            _marker: PhantomData,
        })
    }

    pub fn base_url(&self) -> Url {
        self.base_url.clone()
    }
}

/// Interface to specify optional configuration values before creating a [`Client`].
pub struct ClientBuilder<E: ClientError, VER: StaticVersionType> {
    inner: reqwest::ClientBuilder,
    accept: ContentType,
    base_url: Url,
    timeout: Option<Duration>,
    _marker: PhantomData<fn(E, VER)>,
}

impl<E: ClientError, VER: StaticVersionType> ClientBuilder<E, VER> {
    fn new(mut base_url: Url) -> Self {
        // If the path part of `base_url` does not end in `/`, `join` will treat it as a filename
        // and remove it, which is never what we want: `base_url` is always a directory-like path.
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Self {
            inner: reqwest::Client::builder(),
            accept: ContentType::Binary,
            base_url,
            timeout: Some(Duration::from_secs(60)),
            _marker: PhantomData,
        }
    }

    /// Set connection timeout duration.
    ///
    /// Passing `None` removes the timeout.
    ///
    /// Default: `Some(Duration::from_secs(60))`.
    pub fn set_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the content type used for responses.
    pub fn content_type(mut self, content_type: ContentType) -> Self {
        self.accept = content_type;
        self
    }

    /// Create a [`Client`] with the settings specified in this builder.
    pub fn build(self) -> Client<E, VER> {
        let mut builder = self.inner;
        if let Some(timeout) = self.timeout {
            builder = builder.timeout(timeout);
        }
        Client {
            inner: builder.build().unwrap(),
            base_url: self.base_url,
            accept: self.accept,
            _marker: PhantomData,
        }
    }
}

impl<E: ClientError, VER: StaticVersionType> From<ClientBuilder<E, VER>> for Client<E, VER> {
    fn from(builder: ClientBuilder<E, VER>) -> Self {
        builder.build()
    }
}

#[cfg(test)]
mod test {
    use vbs::version::StaticVersion;

    use super::*;
    use crate::error::ClientErr;

    type Ver01 = StaticVersion<0, 1>;

    #[test]
    fn adds_trailing_slash_to_base_url_path() {
        let client =
            Client::<ClientErr, Ver01>::builder("http://example.com/api".parse().unwrap()).build();
        assert_eq!(
            client.base_url(),
            "http://example.com/api/".parse().unwrap()
        );
    }

    #[test]
    fn leaves_existing_trailing_slash_alone() {
        let client =
            Client::<ClientErr, Ver01>::builder("http://example.com/api/".parse().unwrap()).build();
        assert_eq!(
            client.base_url(),
            "http://example.com/api/".parse().unwrap()
        );
    }

    #[test]
    fn leaves_root_path_alone() {
        let client =
            Client::<ClientErr, Ver01>::builder("http://example.com".parse().unwrap()).build();
        assert_eq!(client.base_url(), "http://example.com/".parse().unwrap());
    }
}
