//! Snapshot harness for the HTTP APIs served by the native demo.
//!
//! Each [`Probe`] names one endpoint. The runner fetches it, normalizes the response body and
//! compares it against a committed insta snapshot. Only the body is snapshotted: a probe whose
//! response is not a success status is a hard failure, not a recorded snapshot, so the suite also
//! asserts that every endpoint it knows about actually works.
//!
//! Version aliases (`/v0/...` and the unversioned form) are not snapshotted separately. They are
//! fetched and required to normalize to the same body as the canonical `/v1/...` route, which is
//! what makes an alias an alias.

use std::{
    collections::{BTreeMap, BTreeSet},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail};
use futures::{SinkExt, StreamExt};
use serde::Serialize;
use tokio::time::{Instant, sleep, timeout};
use tokio_tungstenite::tungstenite::Message;

/// A service in the native demo that serves an HTTP API.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Service {
    Node0,
    Node1,
    Node2,
    Node3,
    Node4,
    Orchestrator,
    StateRelay,
    Builder,
    SubmitPublic,
    SubmitPrivate,
    NodeMetrics,
}

impl Service {
    /// The `.env` variable holding this service's port.
    pub fn port_var(&self) -> &'static str {
        match self {
            Self::Node0 => "ESPRESSO_NODE_0_API_PORT",
            Self::Node1 => "ESPRESSO_NODE_1_API_PORT",
            Self::Node2 => "ESPRESSO_NODE_2_API_PORT",
            Self::Node3 => "ESPRESSO_NODE_3_API_PORT",
            Self::Node4 => "ESPRESSO_NODE_4_API_PORT",
            Self::Orchestrator => "ESPRESSO_ORCHESTRATOR_PORT",
            Self::StateRelay => "ESPRESSO_STATE_RELAY_SERVER_PORT",
            Self::Builder => "ESPRESSO_BUILDER_SERVER_PORT",
            Self::SubmitPublic => "ESPRESSO_SUBMIT_TRANSACTIONS_PUBLIC_PORT",
            Self::SubmitPrivate => "ESPRESSO_SUBMIT_TRANSACTIONS_PRIVATE_PORT",
            Self::NodeMetrics => "ESPRESSO_NODE_VALIDATOR_PORT",
        }
    }

    /// Short name used as the first segment of snapshot file names.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Node0 => "node0",
            Self::Node1 => "node1",
            Self::Node2 => "node2",
            Self::Node3 => "node3",
            Self::Node4 => "node4",
            Self::Orchestrator => "orchestrator",
            Self::StateRelay => "state-relay",
            Self::Builder => "builder",
            Self::SubmitPublic => "submit-public",
            Self::SubmitPrivate => "submit-private",
            Self::NodeMetrics => "node-metrics",
        }
    }

    pub fn base_url(&self) -> Result<String> {
        let port = dotenvy::var(self.port_var())
            .with_context(|| format!("{} is not set", self.port_var()))?;
        Ok(format!(
            "{}://{}:{}",
            dotenvy::var("INTEGRATION_TEST_PROTO")?,
            dotenvy::var("INTEGRATION_TEST_HOST")?,
            port
        ))
    }
}

/// How much of the response body to pin.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Norm {
    /// The body verbatim, with object keys sorted. For responses that are fully determined by
    /// genesis.
    Exact,
    /// Keys, nesting and types, with scalars replaced by type markers and arrays collapsed to
    /// their distinct element shapes. For responses that depend on how far the chain has run.
    Shape,
    /// Prometheus exposition reduced to its sorted metric and label names.
    MetricNames,
    /// A non-JSON body kept as text. Only for responses that are byte-stable.
    Text,
}

/// Which version prefixes must serve a probe's canonical `/v1` route.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Aliases {
    /// Check `/v0/...` and the unversioned form return the same body.
    Check,
    /// Route has no alias forms (`/v2`, top-level routes, non-node services).
    None,
}

/// One endpoint to snapshot.
#[derive(Clone, Debug)]
pub struct Probe {
    pub service: Service,
    /// Snapshot name, minus the service prefix. Stable across runs; `/` and `{}` are not allowed.
    pub name: &'static str,
    /// Path, with `{placeholder}` params resolved from [`Params`].
    pub path: &'static str,
    pub norm: Norm,
    pub aliases: Aliases,
    /// Fields whose value is replaced by a marker, for the per-run values carried inside a body
    /// that is otherwise fixed. `"key"` matches that key at any depth; `"parent.key"` and
    /// `"parent.*"` match only under a parent of that name, so a volatile `l1_finalized.timestamp`
    /// can be masked without losing the header's own deterministic `timestamp`.
    pub mask_fields: &'static [&'static str],
    /// Fields holding an object whose set of keys depends on the run, such as a map of signatures
    /// by signer. The object becomes a deduplicated list of its values, which pins what the entries
    /// look like without pinning which ones have arrived yet.
    pub collapse_fields: &'static [&'static str],
}

impl Probe {
    pub const fn new(service: Service, name: &'static str, path: &'static str) -> Self {
        Self {
            service,
            name,
            path,
            norm: Norm::Exact,
            aliases: Aliases::Check,
            mask_fields: &[],
            collapse_fields: &[],
        }
    }

    /// A `/v1` node route whose body depends on chain progress.
    pub const fn shape(service: Service, name: &'static str, path: &'static str) -> Self {
        Self {
            norm: Norm::Shape,
            ..Self::new(service, name, path)
        }
    }

    pub const fn norm(mut self, norm: Norm) -> Self {
        self.norm = norm;
        self
    }

    /// Route has no `/v0` or unversioned alias.
    pub const fn no_aliases(mut self) -> Self {
        self.aliases = Aliases::None;
        self
    }

    pub const fn mask_fields(mut self, mask_fields: &'static [&'static str]) -> Self {
        self.mask_fields = mask_fields;
        self
    }

    pub const fn collapse_fields(mut self, collapse_fields: &'static [&'static str]) -> Self {
        self.collapse_fields = collapse_fields;
        self
    }

    fn snapshot_name(&self) -> String {
        format!("{}_{}", self.service.slug(), self.name)
    }
}

/// A WebSocket endpoint to snapshot. The first [`WsProbe::take`] messages are normalized and
/// snapshotted together.
#[derive(Clone, Debug)]
pub struct WsProbe {
    pub service: Service,
    pub name: &'static str,
    pub path: &'static str,
    pub take: usize,
    pub norm: Norm,
    pub mask_fields: &'static [&'static str],
    pub collapse_fields: &'static [&'static str],
}

impl WsProbe {
    pub const fn new(service: Service, name: &'static str, path: &'static str) -> Self {
        Self {
            service,
            name,
            path,
            take: 1,
            norm: Norm::Exact,
            mask_fields: &[],
            collapse_fields: &[],
        }
    }

    pub const fn shape(service: Service, name: &'static str, path: &'static str) -> Self {
        Self {
            norm: Norm::Shape,
            ..Self::new(service, name, path)
        }
    }

    pub const fn take(mut self, take: usize) -> Self {
        self.take = take;
        self
    }

    pub const fn mask_fields(mut self, mask_fields: &'static [&'static str]) -> Self {
        self.mask_fields = mask_fields;
        self
    }

    fn snapshot_name(&self) -> String {
        format!("{}_ws_{}", self.service.slug(), self.name)
    }
}

/// Per-run values substituted into probe paths, so probe declarations never hard-code a hash.
#[derive(Clone, Debug)]
pub struct Params {
    pub values: BTreeMap<&'static str, String>,
}

impl Params {
    fn resolve(&self, path: &str) -> Result<String> {
        let mut out = path.to_string();
        for (key, value) in &self.values {
            out = out.replace(&format!("{{{key}}}"), value);
        }
        if let Some(start) = out.find('{') {
            bail!("unresolved path placeholder in {path} at byte {start}");
        }
        Ok(out)
    }
}

/// A body normalized for snapshotting. Ordering is fully determined by this type rather than by
/// `serde_json`'s map implementation, so snapshots do not depend on crate features.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Canon {
    Null,
    Bool(bool),
    Num(serde_json::Number),
    Str(String),
    Arr(Vec<Canon>),
    Obj(BTreeMap<String, Canon>),
}

const MASKED: &str = "<masked>";

fn canon_exact(value: &serde_json::Value) -> Canon {
    match value {
        serde_json::Value::Null => Canon::Null,
        serde_json::Value::Bool(b) => Canon::Bool(*b),
        serde_json::Value::Number(n) => Canon::Num(n.clone()),
        serde_json::Value::String(s) => Canon::Str(s.clone()),
        serde_json::Value::Array(items) => Canon::Arr(items.iter().map(canon_exact).collect()),
        serde_json::Value::Object(map) => Canon::Obj(
            map.iter()
                .map(|(k, v)| (k.clone(), canon_exact(v)))
                .collect(),
        ),
    }
}

/// Replace scalars with type markers and collapse arrays to their distinct element shapes, so the
/// result records structure without recording how far the chain has run.
fn canon_shape(value: &serde_json::Value) -> Canon {
    match value {
        serde_json::Value::Null => Canon::Null,
        serde_json::Value::Bool(_) => Canon::Str("<bool>".into()),
        serde_json::Value::Number(_) => Canon::Str("<number>".into()),
        serde_json::Value::String(_) => Canon::Str("<string>".into()),
        serde_json::Value::Array(items) => Canon::Arr(dedup(items.iter().map(canon_shape))),
        serde_json::Value::Object(map) => Canon::Obj(
            map.iter()
                .map(|(k, v)| (k.clone(), canon_shape(v)))
                .collect(),
        ),
    }
}

fn dedup(items: impl Iterator<Item = Canon>) -> Vec<Canon> {
    let mut out: Vec<Canon> = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

/// Apply a mutation at a JSON Pointer. A pointer that does not resolve is ignored: probes share
/// mask lists across endpoints whose bodies differ in which optional fields are present.
/// Does `key`, found directly under `parent`, match one of the patterns? A pattern without a dot
/// matches the key at any depth; `parent.key` and `parent.*` are restricted to that parent.
fn field_matches(patterns: &[&str], parent: Option<&str>, key: &str) -> bool {
    patterns
        .iter()
        .any(|pattern| match pattern.split_once('.') {
            None => *pattern == key,
            Some((want_parent, want_key)) => {
                parent == Some(want_parent) && (want_key == "*" || want_key == key)
            },
        })
}

fn rewrite_fields(
    value: &mut serde_json::Value,
    mask: &[&str],
    collapse: &[&str],
    parent: Option<&str>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if field_matches(mask, parent, key) {
                    *child = serde_json::Value::String(MASKED.into());
                    continue;
                }
                if let serde_json::Value::Object(entries) = child
                    && field_matches(collapse, parent, key)
                {
                    let mut values: Vec<serde_json::Value> = Vec::new();
                    for entry in entries.values() {
                        if !values.contains(entry) {
                            values.push(entry.clone());
                        }
                    }
                    *child = serde_json::Value::Array(values);
                }
                rewrite_fields(child, mask, collapse, Some(key));
            }
        },
        serde_json::Value::Array(items) => {
            for item in items {
                rewrite_fields(item, mask, collapse, parent);
            }
        },
        _ => {},
    }
}

fn prometheus_names(body: &str) -> Canon {
    let mut names = BTreeSet::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // "# HELP <name> ..." and "# TYPE <name> ..." carry the name in the same position.
        let decl = line
            .strip_prefix("# HELP ")
            .or_else(|| line.strip_prefix("# TYPE "));
        if let Some(rest) = decl {
            if let Some(name) = rest.split_whitespace().next() {
                names.insert(name.to_string());
            }
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let Some(sample) = line.split_whitespace().next() else {
            continue;
        };
        // Keep label names, drop label values: `foo{a="1",b="2"} 3` -> `foo{a,b}`.
        match sample.split_once('{') {
            None => {
                names.insert(sample.to_string());
            },
            Some((name, labels)) => {
                let labels = labels.trim_end_matches('}');
                let keys: Vec<&str> = labels
                    .split(',')
                    .filter_map(|kv| kv.split('=').next())
                    .filter(|k| !k.is_empty())
                    .collect();
                names.insert(format!("{name}{{{}}}", keys.join(",")));
            },
        }
    }
    Canon::Arr(names.into_iter().map(Canon::Str).collect())
}

fn normalize(body: &str, norm: Norm, mask: &[&str], collapse: &[&str]) -> Result<Canon> {
    if norm == Norm::MetricNames {
        return Ok(prometheus_names(body));
    }
    if norm == Norm::Text {
        return Ok(Canon::Str(body.to_string()));
    }

    let mut value: serde_json::Value = serde_json::from_str(body)
        .with_context(|| format!("body is not JSON: {}", truncate(body, 200)))?;
    rewrite_fields(&mut value, mask, collapse, None);

    Ok(match norm {
        Norm::Exact => canon_exact(&value),
        Norm::Shape => canon_shape(&value),
        Norm::MetricNames | Norm::Text => unreachable!("handled above"),
    })
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let cut = s.char_indices().map(|(i, _)| i).nth(max).unwrap_or(s.len());
    format!("{}... ({} bytes total)", &s[..cut], s.len())
}

/// Runs probes and accumulates failures so one run reports every mismatch rather than the first.
pub struct SnapshotRunner {
    client: reqwest::Client,
    params: Params,
    settings: insta::Settings,
    failures: Vec<String>,
    passed: usize,
    filter: Option<String>,
}

impl SnapshotRunner {
    pub fn new(params: Params) -> Result<Self> {
        if std::env::var("CI").unwrap_or_default() == "true" {
            let update = std::env::var("INSTA_UPDATE").unwrap_or_default();
            if !update.is_empty() && update != "no" {
                bail!(
                    "refusing to run with INSTA_UPDATE={update} on CI: snapshots must be checked, \
                     not written"
                );
            }
        }

        // Because we use nextest with the archive feature on CI we need to use the **runtime**
        // value of CARGO_MANIFEST_DIR.
        let crate_dir =
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
        let mut settings = insta::Settings::clone_current();
        settings.set_snapshot_path(crate_dir.join("snapshots").join("api"));
        settings.set_prepend_module_to_snapshot(false);
        settings.set_omit_expression(true);

        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .no_proxy()
                .build()?,
            params,
            settings,
            failures: Vec::new(),
            passed: 0,
            filter: std::env::var("API_SNAPSHOT_FILTER")
                .ok()
                .filter(|f| !f.is_empty()),
        })
    }

    fn skip(&self, name: &str) -> bool {
        self.filter
            .as_ref()
            .is_some_and(|filter| !name.contains(filter.as_str()))
    }

    /// Fetch a URL, retrying while the request limiter is shedding load so a 429 is never
    /// mistaken for an endpoint's real response.
    async fn get(&self, url: &str) -> Result<(reqwest::StatusCode, String)> {
        let mut attempt = 0;
        loop {
            let response = self
                .client
                .get(url)
                .send()
                .await
                .with_context(|| format!("GET {url}"))?;
            let status = response.status();
            let body = response
                .text()
                .await
                .with_context(|| format!("reading body of {url}"))?;
            if status != reqwest::StatusCode::TOO_MANY_REQUESTS || attempt >= 3 {
                return Ok((status, body));
            }
            attempt += 1;
            sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn run(&mut self, probe: &Probe) {
        let name = probe.snapshot_name();
        if self.skip(&name) {
            return;
        }
        if let Err(err) = self.run_inner(probe, &name).await {
            self.failures.push(format!("{name}: {err:#}"));
        }
    }

    async fn run_inner(&mut self, probe: &Probe, name: &str) -> Result<()> {
        let base = probe.service.base_url()?;
        let path = self.params.resolve(probe.path)?;
        let url = format!("{base}{path}");

        let (status, body) = self.get(&url).await?;
        if !status.is_success() {
            bail!("GET {path} returned {status}: {}", truncate(&body, 300));
        }
        let canon = normalize(&body, probe.norm, probe.mask_fields, probe.collapse_fields)
            .with_context(|| format!("normalizing {path}"))?;

        if probe.aliases == Aliases::Check {
            self.check_aliases(probe, &base, &path, &canon).await?;
        }

        self.assert_snapshot(name, &canon)
    }

    /// `/v0/x` and `/x` are rewritten to `/v1/x`, so they must produce the same body.
    async fn check_aliases(
        &self,
        probe: &Probe,
        base: &str,
        path: &str,
        expected: &Canon,
    ) -> Result<()> {
        let suffix = path
            .strip_prefix("/v1")
            .ok_or_else(|| anyhow!("alias check needs a /v1 path, got {path}"))?;
        for alias in [format!("/v0{suffix}"), suffix.to_string()] {
            let (status, body) = self.get(&format!("{base}{alias}")).await?;
            if !status.is_success() {
                bail!(
                    "alias GET {alias} returned {status}: {}",
                    truncate(&body, 300)
                );
            }
            let canon = normalize(&body, probe.norm, probe.mask_fields, probe.collapse_fields)
                .with_context(|| format!("normalizing alias {alias}"))?;
            if canon != *expected {
                bail!("alias {alias} body differs from {path}");
            }
        }
        Ok(())
    }

    pub async fn run_ws(&mut self, probe: &WsProbe) {
        let name = probe.snapshot_name();
        if self.skip(&name) {
            return;
        }
        if let Err(err) = self.run_ws_inner(probe, &name).await {
            self.failures.push(format!("{name}: {err:#}"));
        }
    }

    async fn run_ws_inner(&mut self, probe: &WsProbe, name: &str) -> Result<()> {
        let base = probe.service.base_url()?;
        let path = self.params.resolve(probe.path)?;
        let url = format!("{}{}", base.replacen("http", "ws", 1), path);

        let messages = timeout(Duration::from_secs(60), async {
            let (mut socket, _response) = tokio_tungstenite::connect_async(&url)
                .await
                .with_context(|| format!("connecting to {url}"))?;
            let mut messages = Vec::new();
            while messages.len() < probe.take {
                let Some(message) = socket.next().await else {
                    bail!(
                        "stream closed after {} of {} messages",
                        messages.len(),
                        probe.take
                    );
                };
                match message.with_context(|| format!("reading from {url}"))? {
                    Message::Text(text) => messages.push(text.to_string()),
                    Message::Binary(bytes) => {
                        bail!("expected text frames, got {} binary bytes", bytes.len())
                    },
                    Message::Close(frame) => bail!("server closed early: {frame:?}"),
                    // Control frames carry no stream items.
                    Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => continue,
                }
            }
            socket.send(Message::Close(None)).await.ok();
            Ok(messages)
        })
        .await
        .with_context(|| format!("timed out reading {} messages from {path}", probe.take))??;

        let canon = Canon::Arr(
            messages
                .iter()
                .map(|message| {
                    normalize(
                        message,
                        probe.norm,
                        probe.mask_fields,
                        probe.collapse_fields,
                    )
                })
                .collect::<Result<Vec<_>>>()
                .with_context(|| format!("normalizing messages from {path}"))?,
        );

        self.assert_snapshot(name, &canon)
    }

    /// insta panics on a mismatch, but this suite reports every endpoint that changed, so the
    /// panic is caught and recorded. insta still writes the `.snap.new` file first, so
    /// `cargo insta review` works normally.
    fn assert_snapshot(&mut self, name: &str, canon: &Canon) -> Result<()> {
        let settings = self.settings.clone();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            settings.bind(|| insta::assert_yaml_snapshot!(name, canon));
        }));
        match outcome {
            Ok(()) => {
                self.passed += 1;
                Ok(())
            },
            Err(_) => Err(anyhow!("snapshot mismatch")),
        }
    }

    /// Fail with every mismatch found, or report how many endpoints were pinned.
    pub fn finish(self) -> Result<()> {
        if self.failures.is_empty() {
            println!("api snapshots: {} endpoints matched", self.passed);
            return Ok(());
        }
        let mut report = format!(
            "{} of {} api snapshots failed (see tests/api_snapshots/README.md to re-record):\n",
            self.failures.len(),
            self.failures.len() + self.passed
        );
        for failure in &self.failures {
            report.push_str("  - ");
            report.push_str(failure);
            report.push('\n');
        }
        Err(anyhow!(report))
    }
}

/// Wait until a URL answers with any HTTP response. Unlike `common::wait_for_service` this does
/// not require a success status, so it can gate on services whose healthcheck is not 2xx.
pub async fn wait_for_http(url: &str, timeout_after: Duration) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .no_proxy()
        .build()?;
    let deadline = Instant::now() + timeout_after;
    let mut last_err = None;
    while Instant::now() < deadline {
        match client.get(url).send().await {
            Ok(response) => {
                tracing::debug!(%url, status = %response.status(), "service is up");
                return Ok(());
            },
            Err(err) => {
                last_err = Some(err);
                sleep(Duration::from_secs(1)).await;
            },
        }
    }
    Err(anyhow!(
        "timed out after {timeout_after:?} waiting for {url}: {last_err:?}"
    ))
}
