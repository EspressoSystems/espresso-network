//! HTTP to the watched nodes and the errors it returns.

use std::{error::Error, fmt, time::Duration};

use anyhow::{anyhow, ensure};
use reqwest::StatusCode;
use serde::{Deserialize, de::DeserializeOwned};
use url::Url;

use crate::{
    gaps::parse_missing_height,
    kind::{DataKind, Height},
    outcome::{FailedStatus, Failure, NotServed},
};

/// Total time of a request that may wait server-side: a range HEAD over up to 100 blocks waits up
/// to 100 x 500 ms for blocks the node does not have yet (`state.rs:85`).
const RANGE_TIMEOUT: Duration = Duration::from_secs(60);

/// Total time of a HEAD of one height, point or epoch: no range wait, so a stuck node costs 10 s
/// per probe and not a minute.
const SINGLE_TIMEOUT: Duration = Duration::from_secs(10);

/// Total time of a GET that is decoded (tip, state height, config).
const JSON_TIMEOUT: Duration = Duration::from_secs(5);

/// A query node's base URL and label.
#[derive(Clone, Debug)]
pub struct WatchedNode {
    pub label: String,
    pub base_url: Url,
}

impl WatchedNode {
    /// Labelled `host:port`; node URLs are internal and carry no credentials. Errors on a base
    /// path without a trailing slash, which `Url::join` would truncate.
    pub fn new(base_url: Url) -> anyhow::Result<Self> {
        let host = base_url
            .host_str()
            .ok_or_else(|| anyhow!("{} node URL has no host", base_url.scheme()))?;
        let port = base_url
            .port_or_known_default()
            .ok_or_else(|| anyhow!("node URL of {host} has no port"))?;
        ensure!(
            base_url.username().is_empty() && base_url.password().is_none(),
            "node URL of {host} must not carry credentials"
        );
        ensure!(
            base_url.path().ends_with('/'),
            "node URL path must end with '/': {}",
            base_url.path()
        );
        Ok(Self {
            label: format!("{host}:{port}"),
            base_url,
        })
    }

    /// `route` below the base URL.
    pub(crate) fn route_url(&self, route: &str) -> Url {
        self.base_url
            .join(route)
            .expect("a base URL can be extended by a relative route")
    }
}

/// Why a request returned no 200.
#[derive(Debug)]
pub enum FetchError {
    /// A non-2xx answer; `not_served` is built once from the status.
    Status {
        status: StatusCode,
        not_served: NotServed,
    },
    Transport(reqwest::Error),
}

impl FetchError {
    /// The `status` label: `StatusCode::as_str()` (`"404"`, `"500"`, ...) or `"transport"`.
    pub fn status_label(&self) -> &str {
        match self {
            Self::Status { status, .. } => status.as_str(),
            Self::Transport(_) => "transport",
        }
    }

    /// 404 or the failure; replaces `is_not_found`.
    pub fn not_served(&self) -> NotServed {
        match self {
            Self::Status { not_served, .. } => *not_served,
            Self::Transport(_) => NotServed::Failed(Failure::Transport),
        }
    }

    /// The error of a non-2xx `status`.
    fn status(status: StatusCode) -> Self {
        let not_served = FailedStatus::new(status).map_or(NotServed::NotFound, |failed| {
            NotServed::Failed(Failure::Status(failed))
        });
        Self::Status { status, not_served }
    }

    /// `Ok` for a 2xx status.
    fn from_status(status: StatusCode) -> Result<(), Self> {
        if status.is_success() {
            Ok(())
        } else {
            Err(Self::status(status))
        }
    }
}

impl fmt::Display for FetchError {
    /// Status code, or the transport error with its cause.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Status { status, .. } => write!(f, "status {status}"),
            Self::Transport(error) => {
                write!(f, "transport error: {error}")?;
                let mut cause = error.source();
                while let Some(source) = cause {
                    write!(f, ": {source}")?;
                    cause = source.source();
                }
                Ok(())
            },
        }
    }
}

/// What a failed request asked for.
#[derive(Clone, Copy, Debug)]
pub enum FetchTarget {
    Data(DataKind),
    BlockHeight,
    MerklizedStateHeight,
    Config,
}

impl FetchTarget {
    /// The `kind` label of `fetch_errors_total`: the `DataKind` label, `"block_height"`,
    /// `"merklized_state_height"` or `"config"`.
    pub fn label(self) -> &'static str {
        match self {
            Self::Data(kind) => kind.into(),
            Self::BlockHeight => "block_height",
            Self::MerklizedStateHeight => "merklized_state_height",
            Self::Config => "config",
        }
    }
}

/// What the GET of a range that failed with 404 says.
pub enum HeightLookup {
    /// The first missing height.
    Named(Height),
    /// A 404 whose body does not name a height.
    NoHeight,
    /// The range is served after all.
    Present,
}

/// The 404 body of the availability routes.
#[derive(Deserialize)]
struct ErrorBody {
    #[serde(rename = "Custom")]
    custom: ErrorMessage,
}

/// The message of an `ErrorBody`.
#[derive(Deserialize)]
struct ErrorMessage {
    message: String,
}

/// HTTP to the watched nodes: one reqwest client with `RANGE_TIMEOUT`.
#[derive(Clone, Debug)]
pub struct NodeHttp {
    client: reqwest::Client,
}

impl NodeHttp {
    /// Builds the client.
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder().timeout(RANGE_TIMEOUT).build()?,
        })
    }

    /// HEAD `route`; the status is the answer. `RANGE_TIMEOUT` when the node may wait for blocks
    /// (a range over several heights), else `SINGLE_TIMEOUT`.
    pub async fn head(
        &self,
        node: &WatchedNode,
        route: &str,
        waits_server_side: bool,
    ) -> Result<(), FetchError> {
        let timeout = if waits_server_side {
            RANGE_TIMEOUT
        } else {
            SINGLE_TIMEOUT
        };
        let response = self
            .client
            .head(node.route_url(route))
            .timeout(timeout)
            .send()
            .await
            .map_err(FetchError::Transport)?;
        FetchError::from_status(response.status())
    }

    /// GET `route` with `JSON_TIMEOUT`, decoding a 2xx body.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        node: &WatchedNode,
        route: &str,
    ) -> Result<T, FetchError> {
        let response = self
            .client
            .get(node.route_url(route))
            .timeout(JSON_TIMEOUT)
            .send()
            .await
            .map_err(FetchError::Transport)?;
        FetchError::from_status(response.status())?;
        response.json().await.map_err(FetchError::Transport)
    }

    /// GET `route` after its HEAD said 404: the named height, `NoHeight`, or `Present`.
    pub async fn lookup_missing_height(
        &self,
        node: &WatchedNode,
        route: &str,
    ) -> Result<HeightLookup, FetchError> {
        let response = self
            .client
            .get(node.route_url(route))
            .send()
            .await
            .map_err(FetchError::Transport)?;
        let status = response.status();
        if status.is_success() {
            return Ok(HeightLookup::Present);
        }
        if status != StatusCode::NOT_FOUND {
            return Err(FetchError::status(status));
        }
        let body = response.bytes().await.map_err(FetchError::Transport)?;
        let height = serde_json::from_slice::<ErrorBody>(&body)
            .ok()
            .and_then(|body| parse_missing_height(&body.custom.message));
        Ok(height.map_or(HeightLookup::NoHeight, HeightLookup::Named))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_with_credentials_are_rejected() {
        let url = |s: &str| Url::parse(s).unwrap();
        assert!(WatchedNode::new(url("http://host:1/")).is_ok());
        assert!(WatchedNode::new(url("http://user:token@host:1/")).is_err());
        assert!(WatchedNode::new(url("http://user@host:1/")).is_err());
    }
}
