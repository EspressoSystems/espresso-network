//! A reqwest-based HTTP/WebSocket client for the network's HTTP APIs.
//!
//! This crate is only the transport shell (reqwest for HTTP, tokio-tungstenite for WebSockets)
//! and the surf-disco-shaped API surface; the wire protocol itself (content negotiation, body
//! and frame codecs, error envelope, health types) lives in [`http_wire`], shared with the
//! server side in `espresso-api`.
//!
//! This crate is a path dependency only, never published: the unrelated crates.io package
//! `http-client` 6.5.3 remains in the dependency graph transitively via `tide`/`surf` until
//! their removal.
//!
//! ```no_run
//! # use http_client::{error::ClientErr, Client};
//! # use vbs::version::StaticVersion;
//! # async fn ex() {
//! let url = "http://localhost:50000".parse().unwrap();
//! let client: Client<ClientErr, StaticVersion<0, 1>> = Client::new(url);
//! let res: String = client.get("/app/route").send().await.unwrap();
//! # }
//! ```

pub mod client;
pub mod error;
pub mod healthcheck;
pub mod request;
pub mod socket;

pub use client::{Client, ClientBuilder, ContentType};
pub use error::ClientError;
pub use request::Request;
pub use reqwest::StatusCode;
pub use socket::SocketRequest;
/// Re-exported so [`Client::socket_with_config`] callers don't link `tokio-tungstenite` directly.
pub use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
pub use url::Url;
