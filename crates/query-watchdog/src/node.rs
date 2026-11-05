//! HTTP to the watched nodes and the errors it returns.

use std::{error::Error, fmt, time::Duration};

use anyhow::{anyhow, ensure};
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use url::Url;

use crate::{
    kind::{DataKind, Probe, height_count},
    outcome::{FailedStatus, NotServed, RequestFailure},
};

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

/// Which timeout a request gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequestKind {
    /// HEAD of a range over several heights; the node may wait 500 ms per height
    /// (`state.rs:85`).
    RangeHead,
    /// HEAD of one height, a point or an epoch.
    SingleHead,
    /// GET decoded as JSON: tip, state height, config.
    Json,
    /// HEAD of a route that fetches the object from peers when the node lacks it.
    PeerFetchHead,
}

impl RequestKind {
    /// `RangeHead` for a range probe over more than one height, else `SingleHead`.
    pub fn head_of(probe: &Probe) -> RequestKind {
        match probe {
            Probe::Range { heights, .. } if height_count(heights) > 1 => Self::RangeHead,
            Probe::Range { .. } | Probe::Point { .. } | Probe::Epoch { .. } => Self::SingleHead,
        }
    }
}

/// Total time per request kind, connect to end of body (`RequestBuilder::timeout`); the only
/// timeout policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProbeTimeouts {
    pub range_head: Duration,
    pub single_head: Duration,
    pub json: Duration,
    /// Long enough for a fetch from a peer that has the object to finish and be stored: a client
    /// timeout cancels the node's fetch.
    pub peer_fetch_head: Duration,
}

impl ProbeTimeouts {
    /// 60 s, 10 s, 5 s, 5 s.
    pub const DEFAULT: ProbeTimeouts = ProbeTimeouts {
        range_head: Duration::from_secs(60),
        single_head: Duration::from_secs(10),
        json: Duration::from_secs(5),
        peer_fetch_head: Duration::from_secs(5),
    };

    /// The timeout of `request`.
    pub fn of(&self, request: RequestKind) -> Duration {
        match request {
            RequestKind::RangeHead => self.range_head,
            RequestKind::SingleHead => self.single_head,
            RequestKind::Json => self.json,
            RequestKind::PeerFetchHead => self.peer_fetch_head,
        }
    }
}

/// A request without 2xx.
#[derive(Debug)]
pub enum FetchError {
    NotFound,
    Status(FailedStatus),
    Transport(reqwest::Error),
}

impl FetchError {
    /// `NotFound` or the failure without its cause.
    pub fn not_served(&self) -> NotServed {
        match self {
            Self::NotFound => NotServed::NotFound,
            Self::Status(status) => NotServed::Failed(RequestFailure::Status(*status)),
            Self::Transport(_) => NotServed::Failed(RequestFailure::Transport),
        }
    }

    /// Whether a peer-fetch route answered that no peer could serve the object: 404, 500 or a
    /// timeout.
    pub fn is_peer_miss(&self) -> bool {
        match self {
            Self::NotFound => true,
            Self::Status(status) => status.is_internal_server_error(),
            Self::Transport(error) => error.is_timeout(),
        }
    }

    /// `Ok` for a 2xx status.
    fn from_status(status: StatusCode) -> Result<(), Self> {
        if status.is_success() {
            Ok(())
        } else {
            Err(FailedStatus::new(status).map_or(Self::NotFound, Self::Status))
        }
    }
}

impl fmt::Display for FetchError {
    /// Status code, or the transport error with its cause.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "status {}", StatusCode::NOT_FOUND),
            Self::Status(status) => write!(f, "status {status}"),
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

/// HTTP to the watched nodes; every request sets its own timeout from `timeouts`.
#[derive(Clone, Debug)]
pub struct NodeHttp {
    client: reqwest::Client,
    timeouts: ProbeTimeouts,
}

impl NodeHttp {
    /// A client without a default timeout.
    pub fn new(timeouts: ProbeTimeouts) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::Client::builder().build()?,
            timeouts,
        })
    }

    /// HEAD of `probe` with `RequestKind::head_of(probe)`'s timeout.
    pub async fn head(&self, node: &WatchedNode, probe: &Probe) -> Result<(), FetchError> {
        self.head_route(node, &probe.route(), RequestKind::head_of(probe))
            .await
    }

    /// HEAD of `probe.peer_fetch_route()` with the `PeerFetchHead` timeout; `None` when the probe
    /// has no such route.
    pub async fn head_peer_fetch(
        &self,
        node: &WatchedNode,
        probe: &Probe,
    ) -> Option<Result<(), FetchError>> {
        let route = probe.peer_fetch_route()?;
        Some(
            self.head_route(node, &route, RequestKind::PeerFetchHead)
                .await,
        )
    }

    /// HEAD of `route` with the timeout of `request`.
    async fn head_route(
        &self,
        node: &WatchedNode,
        route: &str,
        request: RequestKind,
    ) -> Result<(), FetchError> {
        let response = self
            .client
            .head(node.route_url(route))
            .timeout(self.timeouts.of(request))
            .send()
            .await
            .map_err(FetchError::Transport)?;
        FetchError::from_status(response.status())
    }

    /// GET `route` with the `Json` timeout, decoding a 2xx body.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        node: &WatchedNode,
        route: &str,
    ) -> Result<T, FetchError> {
        let response = self
            .client
            .get(node.route_url(route))
            .timeout(self.timeouts.of(RequestKind::Json))
            .send()
            .await
            .map_err(FetchError::Transport)?;
        FetchError::from_status(response.status())?;
        response.json().await.map_err(FetchError::Transport)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::{Router, routing::any};
    use tokio::net::TcpListener;

    use super::*;
    use crate::kind::{Epoch, EpochKind, Height, PointKind, RangeKind};

    /// A node that answers every request after `delay`.
    async fn slow_node(delay: Duration) -> WatchedNode {
        let app = Router::new().fallback(any(move || async move {
            tokio::time::sleep(delay).await;
            "{}"
        }));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });
        WatchedNode::new(url).unwrap()
    }

    fn range(heights: std::ops::Range<u64>) -> Probe {
        Probe::Range {
            kind: RangeKind::Leaf,
            heights: Height(heights.start)..Height(heights.end),
        }
    }

    #[tokio::test]
    async fn each_request_kind_gets_its_own_timeout() {
        let node = slow_node(Duration::from_millis(250)).await;
        let http = NodeHttp::new(ProbeTimeouts {
            range_head: Duration::from_millis(500),
            single_head: Duration::from_millis(100),
            json: Duration::from_millis(100),
            peer_fetch_head: Duration::from_millis(100),
        })
        .unwrap();
        http.head(&node, &range(0..100)).await.unwrap();
        let timed_out =
            |error: FetchError| matches!(error, FetchError::Transport(e) if e.is_timeout());
        let point = Probe::Point {
            kind: PointKind::LcLeafProof,
            height: Height(5),
        };
        assert!(timed_out(http.head(&node, &range(5..6)).await.unwrap_err()));
        assert!(timed_out(http.head(&node, &point).await.unwrap_err()));
        let json = http.get_json::<serde_json::Value>(&node, "v1/x").await;
        assert!(timed_out(json.unwrap_err()));
    }

    #[test]
    fn default_timeouts_and_request_kinds() {
        let secs = |kind| ProbeTimeouts::DEFAULT.of(kind).as_secs();
        assert_eq!(secs(RequestKind::RangeHead), 60);
        assert_eq!(secs(RequestKind::SingleHead), 10);
        assert_eq!(secs(RequestKind::Json), 5);
        assert_eq!(secs(RequestKind::PeerFetchHead), 5);
        let epoch = Probe::Epoch {
            kind: EpochKind::StakeTable,
            epoch: Epoch(3),
        };
        let point = Probe::Point {
            kind: PointKind::LcLeafProof,
            height: Height(5),
        };
        assert_eq!(RequestKind::head_of(&range(0..2)), RequestKind::RangeHead);
        assert_eq!(RequestKind::head_of(&range(0..1)), RequestKind::SingleHead);
        assert_eq!(RequestKind::head_of(&point), RequestKind::SingleHead);
        assert_eq!(RequestKind::head_of(&epoch), RequestKind::SingleHead);
    }

    #[test]
    fn statuses_map_to_fetch_errors() {
        let error = |code| FetchError::from_status(StatusCode::from_u16(code).unwrap());
        assert!(error(200).is_ok());
        assert_eq!(error(404).unwrap_err().not_served(), NotServed::NotFound);
        assert_eq!(error(500).unwrap_err().not_served().label(), "500");
    }

    #[test]
    fn urls_with_credentials_are_rejected() {
        let url = |s: &str| Url::parse(s).unwrap();
        assert!(WatchedNode::new(url("http://host:1/")).is_ok());
        assert!(WatchedNode::new(url("http://user:token@host:1/")).is_err());
        assert!(WatchedNode::new(url("http://user@host:1/")).is_err());
    }
}
