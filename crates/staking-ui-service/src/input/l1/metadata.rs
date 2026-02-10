//! Dealing with node metadata.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};

use crate::{
    Error, Result,
    error::{ResultExt, ensure},
    types::common::{ImageSet, NodeMetadata, NodeMetadataContent},
};
use hickory_resolver::{Resolver, TokioResolver};
use prometheus_parse::Scrape;
use reqwest::Url;
use tagged_base64::TaggedBase64;
use tide_disco::http::url::Host;
use tracing::instrument;

/// Metadata sourced from third party URIs should be automatically refreshed every fixed number of
/// L1 blocks.
///
/// 300 L1 blocks on Ethereum takes approximately 300 * 12 = 3600 seconds, or 1 hour.
///
/// # Determinism
///
/// It is important that this parameter is a fixed constant, not configurable, because it impacts
/// the derivation of [`FullNodeSetDiff`](crate::types::global::FullNodeSetDiff)s from
/// [`L1Event`](super::L1Event)s (by injecting a `NodeUpdate` diff for each node whose metadata has
/// changed every [`METADATA_REFRESH_BLOCKS`] blocks). This derivation is supposed to be
/// deterministic: two copies of this software processing the same L1 events should derive the same
/// diffs, even with different environment variables, so this parameter must be fixed.
pub const METADATA_REFRESH_BLOCKS: u64 = 300;

/// Fetching metadata from third-party URIs.
pub trait MetadataFetcher: Sync {
    /// Fetch node metadata from the given URI.
    ///
    /// If the URI is invalid or empty, the node is considered to have opted out of metadata, and
    /// the result is [`None`].
    ///
    /// If the resource is unretrievable or the returned resource cannot be parsed as valid
    /// [`NodeMetadataContent`], returns [`Some`] [`NodeMetadata`], but the
    /// [`NodeMetadata::content`] is [`None`] (the valid, parsed URI will still be populated in the
    /// returned [`NodeMetadata`], so that the content can be refetched at a later time).
    fn fetch_infallible(&self, uri: &str) -> impl Send + Future<Output = Option<NodeMetadata>> {
        async move {
            let uri = parse_metadata_uri(uri)?;
            let content = match self.fetch_content(&uri).await {
                Ok(content) => Some(content),
                Err(err) => {
                    tracing::warn!(%uri, "unable to fetch metadata, returning default: {err:#}");
                    None
                }
            };
            Some(NodeMetadata { uri, content })
        }
    }

    /// Fetch node metadata from the given URI, authenticating against the given public key.
    ///
    /// If `pub_key` does not match the public key contained in the metadata content, the content
    /// is treated as invalid (i.e. the result has [`NodeMetadata::content`]: [`None`]).
    fn fetch_infallible_authenticated(
        &self,
        uri: &str,
        pub_key: &TaggedBase64,
    ) -> impl Send + Future<Output = Option<NodeMetadata>> {
        async move {
            let mut metadata = self.fetch_infallible(uri).await?;
            if let Some(content) = &mut metadata.content
                && *pub_key != content.pub_key.into()
            {
                tracing::warn!(
                    %pub_key,
                    %content.pub_key,
                    "metadata content includes incorrect public key",
                );
                metadata.content = None;
            }
            Some(metadata)
        }
    }

    /// Download and parse node metadata content from a third-party URI.
    fn fetch_content(&self, url: &Url) -> impl Send + Future<Output = Result<NodeMetadataContent>>;

    /// Download and parse node metadata content from a third-party URI, authenticating against the
    /// given public key.
    ///
    /// If `pub_key` does not match the public key contained in the metadata content, an error is
    /// returned.
    fn fetch_content_authenticated(
        &self,
        uri: &Url,
        pub_key: &TaggedBase64,
    ) -> impl Send + Future<Output = Result<NodeMetadataContent>> {
        async move {
            let content = self.fetch_content(uri).await?;
            ensure!(
                *pub_key == content.pub_key.into(),
                Error::internal().context(format!(
                    "metadata content includes incorrect public key {} (expected {pub_key})",
                    content.pub_key
                ))
            );
            Ok(content)
        }
    }
}

/// Object that can download metadata from a given URI.
#[derive(Clone, Debug)]
pub struct HttpMetadataFetcher {
    client: reqwest::Client,
    resolver: TokioResolver,

    // Allow fetching from `localhost` in unit tests, where the staking service and metadata server
    // run on the same host.
    #[cfg(test)]
    allow_local: bool,
}

impl Default for HttpMetadataFetcher {
    fn default() -> Self {
        Self {
            client: reqwest::ClientBuilder::new()
                // Use a fairly short timeout to guard against malicious nonresponsive metadata
                // sites from stalling the entire service.
                .timeout(Duration::from_secs(3))
                .build()
                .unwrap(),
            resolver: Resolver::builder_tokio().unwrap().build(),

            #[cfg(test)]
            allow_local: false,
        }
    }
}

impl HttpMetadataFetcher {
    /// Check that a third-party metadata URI is safe to fetch from.
    ///
    /// Malicious URLs include reserved IP address ranges and loopback hosts (e.g. `localhost`).
    /// Attempting to fetch metadata from such URLs could cause this service to make a request to
    /// its own intranet, potentially leaking sensitive data.
    async fn check_metadata_uri_safety(&self, url: &Url) -> Result<()> {
        let Some(host) = url.host() else {
            return Err(Error::bad_request().context("URI does not have a host"));
        };

        // Allow fetching from `localhost` in unit tests, where the staking service and metadata
        // server run on the same host.
        #[cfg(test)]
        if self.allow_local
            && [Host::Domain("localhost"), Host::Ipv4(Ipv4Addr::LOCALHOST)].contains(&host)
        {
            return Ok(());
        }

        match host {
            Host::Domain(domain) => {
                let ips = self.resolver.lookup_ip(domain).await.context(|| {
                    Error::internal().context(format!("could not resolve metadata host {host}"))
                })?;
                for ip in ips {
                    let res = match ip {
                        IpAddr::V4(ipv4) => check_metadata_ipv4_safety(ipv4),
                        IpAddr::V6(ipv6) => check_metadata_ipv6_safety(ipv6),
                    };
                    res.map_err(|err| err.context(format!("for host {domain}")))?;
                }
            }
            Host::Ipv4(ipv4) => check_metadata_ipv4_safety(ipv4)?,
            Host::Ipv6(ipv6) => check_metadata_ipv6_safety(ipv6)?,
        }

        Ok(())
    }
}

impl MetadataFetcher for HttpMetadataFetcher {
    #[instrument(skip(self))]
    async fn fetch_content(&self, url: &Url) -> Result<NodeMetadataContent> {
        self.check_metadata_uri_safety(url)
            .await
            .map_err(|err| err.context("disallowed metadata URI"))?;
        let res =
            self.client.get(url.clone()).send().await.context(|| {
                Error::internal().context(format!("downloading metadata from {url}"))
            })?;

        // Ignore content type and always try JSON first (some hosts like GitHub raw serve
        // JSON files as text/plain), then fall back to prometheus metrics.
        let text = res.text().await.context(|| {
            Error::internal().context(format!("reading metadata response from {url}"))
        })?;
        match serde_json::from_str(&text) {
            Ok(metadata) => Ok(metadata),
            Err(json_err) => parse_prometheus(&text).map_err(|err| {
                err.context(format!(
                    "from {url}: failed to parse as JSON ({json_err}) or metrics"
                ))
            }),
        }
    }
}

/// Check that an IPv4 address is safe to make a metadata request to.
///
/// This function filters out reserved IP address ranges.
fn check_metadata_ipv4_safety(ip: Ipv4Addr) -> Result<()> {
    ensure!(
        !(ip.is_broadcast()
            || ip.is_link_local()
            || ip.is_loopback()
            || ip.is_multicast()
            || ip.is_private()
            || ip.is_unspecified()
            || ip.is_documentation()),
        Error::bad_request().context(format!("IP {ip} is in reserved range"))
    );
    Ok(())
}

/// Check that an IPv6 address is safe to make a metadata request to.
///
/// This function filters out reserved IPv6 address ranges, as well as unsafe IPv4 addresses encoded
/// in IPv6 format.
fn check_metadata_ipv6_safety(ip: Ipv6Addr) -> Result<()> {
    if let Some(ipv4) = ip.to_ipv4_mapped() {
        check_metadata_ipv4_safety(ipv4)?;
    }
    ensure!(
        !(ip.is_loopback()
            || ip.is_multicast()
            || ip.is_unicast_link_local()
            || ip.is_unique_local()
            || ip.is_unspecified()),
        Error::bad_request().context(format!("IP {ip} is in reserved range"))
    );
    Ok(())
}

/// Interpret prometheus labels as node metadata according to Espresso convention.
fn parse_prometheus(text: &str) -> Result<NodeMetadataContent> {
    let metrics = Scrape::parse(text.lines().map(String::from).map(Ok))
        .context(|| Error::internal().context("malformed prometheus metadata"))?;

    // Find the public key, which is required.
    let node = metrics
        .samples
        .iter()
        .find(|sample| sample.metric.as_str() == "consensus_node")
        .ok_or_else(|| Error::internal().context("missing public key"))?;
    let key = node
        .labels
        .get("key")
        .ok_or_else(|| Error::internal().context("consensus_node metric is missing key label"))?;
    let pub_key = key
        .parse()
        .context(|| Error::internal().context("invalid public key"))?;

    let mut metadata = NodeMetadataContent {
        pub_key,
        ..Default::default()
    };

    for sample in metrics.samples {
        match sample.metric.as_str() {
            "consensus_node_identity_general" => {
                for (label, value) in sample.labels.iter() {
                    match label.as_str() {
                        "name" => metadata.name = Some(value.clone()),
                        "description" => metadata.description = Some(value.clone()),
                        "company_name" => metadata.company_name = Some(value.clone()),
                        "company_website" if !value.is_empty() => match value.parse() {
                            Ok(url) => metadata.company_website = Some(url),
                            Err(err) => {
                                tracing::warn!("malformed company website {value}: {err:#}");
                            }
                        },
                        _ => continue,
                    }
                }
            }
            "consensus_version" => {
                if let Some(desc) = sample.labels.get("desc") {
                    metadata.client_version = Some(desc.into());
                }
            }
            "consensus_node_identity_icon" => {
                let mut icon = ImageSet::default();
                for (label, value) in sample.labels.iter() {
                    if value.is_empty() {
                        continue;
                    }
                    let url = match value.parse() {
                        Ok(url) => url,
                        Err(err) => {
                            tracing::warn!("malformed icon URL {value}: {err:#}");
                            continue;
                        }
                    };
                    match label.as_str() {
                        "small_1x" => icon.small.ratio1 = Some(url),
                        "small_2x" => icon.small.ratio2 = Some(url),
                        "small_3x" => icon.small.ratio3 = Some(url),
                        "large_1x" => icon.large.ratio1 = Some(url),
                        "large_2x" => icon.large.ratio2 = Some(url),
                        "large_3x" => icon.large.ratio3 = Some(url),
                        _ => {
                            tracing::warn!("unrecognized icon format {label}");
                        }
                    }
                }
                metadata.icon = Some(icon);
            }
            _ => continue,
        }
    }
    Ok(metadata)
}

/// Interpret a string as a [`Url`] from a contract event.
///
/// Empty and invalid URIs are treated as missing.
pub fn parse_metadata_uri(uri: &str) -> Option<Url> {
    if uri.is_empty() {
        return None;
    }
    match uri.parse() {
        Ok(uri) => Some(uri),
        Err(err) => {
            // The contract does not validate URIs; nodes can post whatever they want. If
            // what they posted is not a well-formed URI, we will warn but then just ignore
            // it, as if they had opted out of registering one at all.
            tracing::warn!("node registered invalid metadata URI {uri}: {err:#}");
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::{borrow::Cow, str::FromStr, sync::Arc};

    use crate::types::common::{ImageSet, RatioSet};

    use super::*;

    use async_lock::RwLock;
    use espresso_types::PubKey;
    use futures::FutureExt;
    use hotshot_query_service::metrics::PrometheusMetrics;
    use hotshot_types::traits::{metrics::Metrics, signature_key::SignatureKey};
    use portpicker::pick_unused_port;
    use pretty_assertions::assert_eq;
    use tide_disco::{Error as _, StatusCode};
    use tokio::spawn;
    use toml::toml;
    use vbs::version::{StaticVersion, StaticVersionType};
    use warp::Filter;

    #[test_log::test]
    fn test_parse_prometheus_all_fields() {
        let pub_key = PubKey::generated_from_seed_indexed(Default::default(), 42).0;
        let text = format!(
            r#"
        # HELP consensus_node node
        # TYPE consensus_node gauge
        consensus_node{{key="{pub_key}"}} 1
        # HELP consensus_version version
        # TYPE consensus_version gauge
        consensus_version{{desc="db0900d",rev="db0900dc7bc539479dfc58c5ace8f3aed736491a",timestamp="2025-11-21T14:40:40.000000000-05:00"}} 1
        # HELP consensus_node_identity_general node_identity_general
        # TYPE consensus_node_identity_general gauge
        consensus_node_identity_general{{company_name="Espresso Systems",company_website="https://www.espressosys.com/",name="test-node",description="a test node",network_type="AWS",node_type="espresso-sequencer 0.1.0",operating_system="Linux"}} 1
        # HELP consensus_node_identity_location node_identity_location
        # TYPE consensus_node_identity_location gauge
        consensus_node_identity_location{{country="US",latitude="39.96264474822646",longitude="-83.00332937380611"}} 1
        # HELP consensus_node_identity_icon node_identity_icon
        # TYPE consensus_node_identity_icon gauge
        consensus_node_identity_icon{{small_1x="https://www.espressosys.com/small-icon.png",large_2x="https://www.espressosys.com/large-icon.png","medium_1x"="https://www.espressosys.com/ignored-icon.png"}} 1
        "#
        );
        let metadata = parse_prometheus(&text).unwrap();
        assert_eq!(
            metadata,
            NodeMetadataContent {
                pub_key,
                name: Some("test-node".into()),
                description: Some("a test node".into()),
                company_name: Some("Espresso Systems".into()),
                company_website: Some("https://www.espressosys.com/".parse().unwrap()),
                client_version: Some("db0900d".into()),
                icon: Some(ImageSet {
                    small: RatioSet {
                        ratio1: Some(
                            "https://www.espressosys.com/small-icon.png"
                                .parse()
                                .unwrap()
                        ),
                        ratio2: None,
                        ratio3: None,
                    },
                    large: RatioSet {
                        ratio1: None,
                        ratio2: Some(
                            "https://www.espressosys.com/large-icon.png"
                                .parse()
                                .unwrap()
                        ),
                        ratio3: None,
                    }
                }),
            }
        );
    }

    #[test_log::test]
    fn test_parse_prometheus_no_fields() {
        let pub_key = PubKey::generated_from_seed_indexed(Default::default(), 42).0;
        let text = format!(
            r#"
        # HELP consensus_l1_failovers failovers
        # TYPE consensus_l1_failovers counter
        consensus_l1_failovers 0
        # HELP consensus_l1_finalized finalized
        # TYPE consensus_l1_finalized gauge
        consensus_l1_finalized 9711621
        # HELP consensus_l1_head head
        # TYPE consensus_l1_head gauge
        consensus_l1_head 9711696
        # HELP consensus_l1_stream_reconnects stream_reconnects
        # TYPE consensus_l1_stream_reconnects counter
        consensus_l1_stream_reconnects 0
        # HELP consensus_last_decided_time last_decided_time
        # TYPE consensus_last_decided_time gauge
        consensus_last_decided_time 1764171936
        # HELP consensus_last_decided_view last_decided_view
        # TYPE consensus_last_decided_view gauge
        consensus_last_decided_view 6805199
        # HELP consensus_last_synced_block_height last_synced_block_height
        # TYPE consensus_last_synced_block_height gauge
        consensus_last_synced_block_height 6140656
        # HELP consensus_last_voted_view last_voted_view
        # TYPE consensus_last_voted_view gauge
        consensus_last_voted_view 6805201
        # HELP consensus_libp2p_is_ready is_ready
        # TYPE consensus_libp2p_is_ready gauge
        consensus_libp2p_is_ready 1
        # HELP consensus_libp2p_num_connected_peers num_connected_peers
        # TYPE consensus_libp2p_num_connected_peers gauge
        consensus_libp2p_num_connected_peers 97
        # HELP consensus_libp2p_num_failed_messages num_failed_messages
        # TYPE consensus_libp2p_num_failed_messages counter
        consensus_libp2p_num_failed_messages 4098
        # HELP consensus_node node
        # TYPE consensus_node gauge
        consensus_node{{key="{pub_key}"}} 1
        "#
        );
        assert_eq!(
            parse_prometheus(&text).unwrap(),
            NodeMetadataContent {
                pub_key,
                ..Default::default()
            }
        );
    }

    #[test_log::test]
    fn test_parse_prometheus_invalid_urls() {
        let pub_key = PubKey::generated_from_seed_indexed(Default::default(), 42).0;
        let text = format!(
            r#"
        consensus_node{{key={pub_key}}} 1
        consensus_node_identity_general{{company_website="notaurl"}} 1
        consensus_node_identity_icon{{small_1x="notaurl"}} 1
        "#
        );
        assert_eq!(
            parse_prometheus(&text).unwrap(),
            NodeMetadataContent {
                icon: Some(ImageSet::default()),
                pub_key,
                ..Default::default()
            }
        );
    }

    #[test_log::test]
    fn test_parse_prometheus_missing_pub_key() {
        let text = r#"
        # HELP consensus_version version
        # TYPE consensus_version gauge
        consensus_version{{desc="db0900d",rev="db0900dc7bc539479dfc58c5ace8f3aed736491a",timestamp="2025-11-21T14:40:40.000000000-05:00"}} 1
        # HELP consensus_node_identity_general node_identity_general
        # TYPE consensus_node_identity_general gauge
        consensus_node_identity_general{{company_name="Espresso Systems",company_website="https://www.espressosys.com/",name="test-node",description="a test node",network_type="AWS",node_type="espresso-sequencer 0.1.0",operating_system="Linux"}} 1
        # HELP consensus_node_identity_location node_identity_location
        # TYPE consensus_node_identity_location gauge
        consensus_node_identity_location{{country="US",latitude="39.96264474822646",longitude="-83.00332937380611"}} 1
        # HELP consensus_node_identity_icon node_identity_icon
        # TYPE consensus_node_identity_icon gauge
        consensus_node_identity_icon{{small_1x="https://www.espressosys.com/small-icon.png",large_2x="https://www.espressosys.com/large-icon.png","medium_1x"="https://www.espressosys.com/ignored-icon.png"}} 1
        "#;
        parse_prometheus(text).unwrap_err();
    }

    #[test_log::test]
    fn test_parse_metadata_uri_valid() {
        let uri = "https://example.com";
        assert_eq!(parse_metadata_uri(uri), Some(uri.parse().unwrap()));
    }

    #[test_log::test]
    fn test_parse_metadata_uri_invalid() {
        let uri = "notarealuri";
        assert_eq!(parse_metadata_uri(uri), None);
    }

    #[test_log::test]
    fn test_parse_metadata_uri_empty() {
        let uri = "";
        assert_eq!(parse_metadata_uri(uri), None);
    }

    #[test_log::test(tokio::test)]
    async fn test_fetch_empty_url() {
        let fetcher = HttpMetadataFetcher::default();
        assert_eq!(fetcher.fetch_infallible("").await, None);
    }

    #[test_log::test(tokio::test)]
    async fn test_fetch_failure() {
        let fetcher = HttpMetadataFetcher::default();
        let uri = "http://thisisnotarealdomainnamesothisfetchwillfail.com";

        // It parses even though it's not a real website.
        let uri = parse_metadata_uri(uri).unwrap();

        assert_eq!(
            fetcher.fetch_infallible(uri.as_ref()).await,
            Some(NodeMetadata { uri, content: None })
        );
    }

    #[test_log::test(tokio::test)]
    async fn test_fetch_json() {
        let pub_key = PubKey::generated_from_seed_indexed(Default::default(), 42).0;
        let expected = NodeMetadataContent {
            pub_key,
            name: Some("test".into()),
            description: Some("longer description".into()),
            company_name: Some("Espresso Systems".into()),
            company_website: Some("https://www.espressosys.com/".parse().unwrap()),
            client_version: Some("release-main".into()),
            icon: Some(ImageSet {
                small: RatioSet {
                    ratio1: Some(
                        "https://www.espressosys.com/icon-14x14@1x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio2: Some(
                        "https://www.espressosys.com/icon-14x14@2x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio3: Some(
                        "https://www.espressosys.com/icon-14x14@3x.svg"
                            .parse()
                            .unwrap(),
                    ),
                },
                large: RatioSet {
                    ratio1: Some(
                        "https://www.espressosys.com/icon-24x24@1x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio2: Some(
                        "https://www.espressosys.com/icon-24x24@2x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio3: Some(
                        "https://www.espressosys.com/icon-24x24@3x.svg"
                            .parse()
                            .unwrap(),
                    ),
                },
            }),
        };

        let mut app = tide_disco::App::<_, Error>::with_state(expected.clone());
        let api = toml! {
            [route.metadata]
            PATH = ["/metadata"]
        };
        app.module::<Error, StaticVersion<0, 1>>("node", api)
            .unwrap()
            .at("metadata", |_, expected| {
                async move { Ok(expected.clone()) }.boxed()
            })
            .unwrap();

        let port = pick_unused_port().unwrap();
        let server = spawn(app.serve(format!("0.0.0.0:{port}"), StaticVersion::<0, 1>::instance()));

        let fetcher = HttpMetadataFetcher {
            allow_local: true,
            ..Default::default()
        };
        let uri: Url = format!("http://localhost:{port}/node/metadata")
            .parse()
            .unwrap();
        assert_eq!(
            fetcher.fetch_infallible(uri.as_ref()).await.unwrap(),
            NodeMetadata {
                uri: uri.clone(),
                content: Some(expected.clone())
            }
        );

        // Test authentication with valid public key.
        assert_eq!(
            fetcher
                .fetch_infallible_authenticated(uri.as_ref(), &pub_key.into())
                .await
                .unwrap(),
            NodeMetadata {
                uri: uri.clone(),
                content: Some(expected)
            }
        );

        // Test authentication with wrong public key.
        assert_eq!(
            fetcher
                .fetch_infallible_authenticated(
                    uri.as_ref(),
                    &PubKey::generated_from_seed_indexed(Default::default(), 43)
                        .0
                        .into()
                )
                .await
                .unwrap(),
            NodeMetadata { uri, content: None }
        );

        server.abort();
        server.await.ok();
    }

    #[test_log::test(tokio::test)]
    async fn test_fetch_prometheus() {
        let pub_key = PubKey::generated_from_seed_indexed(Default::default(), 42).0;
        let expected = NodeMetadataContent {
            pub_key,
            name: Some("test".into()),
            description: Some("longer description".into()),
            company_name: Some("Espresso Systems".into()),
            company_website: Some("https://www.espressosys.com/".parse().unwrap()),
            client_version: Some("release-main".into()),
            icon: Some(ImageSet {
                small: RatioSet {
                    ratio1: Some(
                        "https://www.espressosys.com/icon-14x14@1x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio2: Some(
                        "https://www.espressosys.com/icon-14x14@2x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio3: Some(
                        "https://www.espressosys.com/icon-14x14@3x.svg"
                            .parse()
                            .unwrap(),
                    ),
                },
                large: RatioSet {
                    ratio1: Some(
                        "https://www.espressosys.com/icon-24x24@1x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio2: Some(
                        "https://www.espressosys.com/icon-24x24@2x.svg"
                            .parse()
                            .unwrap(),
                    ),
                    ratio3: Some(
                        "https://www.espressosys.com/icon-24x24@3x.svg"
                            .parse()
                            .unwrap(),
                    ),
                },
            }),
        };
        let metrics = PrometheusMetrics::default();
        {
            let metrics: &dyn Metrics = &metrics;
            metrics
                .gauge_family("consensus_node".into(), vec!["key".into()])
                .create(vec![pub_key.to_string()])
                .set(1);
            metrics
                .gauge_family(
                    "consensus_node_identity_general".into(),
                    vec![
                        "name".into(),
                        "description".into(),
                        "company_name".into(),
                        "company_website".into(),
                    ],
                )
                .create(vec![
                    expected.name.clone().unwrap(),
                    expected.description.clone().unwrap(),
                    expected.company_name.clone().unwrap(),
                    expected.company_website.as_ref().unwrap().to_string(),
                ])
                .set(1);
            metrics
                .gauge_family("consensus_version".into(), vec!["desc".into()])
                .create(vec![expected.client_version.clone().unwrap()])
                .set(1);
            let icon = expected.icon.as_ref().unwrap();
            metrics
                .gauge_family(
                    "consensus_node_identity_icon".into(),
                    vec![
                        "small_1x".into(),
                        "small_2x".into(),
                        "small_3x".into(),
                        "large_1x".into(),
                        "large_2x".into(),
                        "large_3x".into(),
                    ],
                )
                .create(vec![
                    icon.small.ratio1.as_ref().unwrap().to_string(),
                    icon.small.ratio2.as_ref().unwrap().to_string(),
                    icon.small.ratio3.as_ref().unwrap().to_string(),
                    icon.large.ratio1.as_ref().unwrap().to_string(),
                    icon.large.ratio2.as_ref().unwrap().to_string(),
                    icon.large.ratio3.as_ref().unwrap().to_string(),
                ])
                .set(1);
        }

        let mut app = tide_disco::App::<_, Error>::with_state(Arc::new(RwLock::new(metrics)));
        let api = toml! {
            [route.metadata]
            PATH = ["/metadata"]
            METHOD = "METRICS"
        };
        app.module::<Error, StaticVersion<0, 1>>("node", api)
            .unwrap()
            .metrics("metadata", |_, metrics| {
                async move { Ok(Cow::Borrowed(metrics)) }.boxed()
            })
            .unwrap();

        let port = pick_unused_port().unwrap();
        let server = spawn(app.serve(format!("0.0.0.0:{port}"), StaticVersion::<0, 1>::instance()));

        let fetcher = HttpMetadataFetcher {
            allow_local: true,
            ..Default::default()
        };
        let uri: Url = format!("http://localhost:{port}/node/metadata")
            .parse()
            .unwrap();
        assert_eq!(
            fetcher.fetch_infallible(uri.as_ref()).await.unwrap(),
            NodeMetadata {
                uri,
                content: Some(expected)
            }
        );

        server.abort();
        server.await.ok();
    }

    #[test_log::test(tokio::test)]
    async fn test_metadata_parsing_different_content_type() {
        let pub_key = PubKey::generated_from_seed_indexed(Default::default(), 42).0;
        let expected = NodeMetadataContent {
            pub_key,
            name: Some("test-node".into()),
            description: Some("Test description".into()),
            company_name: Some("Test Company".into()),
            company_website: Some("https://example.com/".parse().unwrap()),
            client_version: Some("v1.0.0".into()),
            icon: None,
        };

        let json_content = serde_json::to_string(&expected).unwrap();

        let metrics = PrometheusMetrics::default();
        {
            let m: &dyn Metrics = &metrics;
            m.gauge_family("consensus_node".into(), vec!["key".into()])
                .create(vec![pub_key.to_string()])
                .set(1);
            m.gauge_family(
                "consensus_node_identity_general".into(),
                vec![
                    "name".into(),
                    "description".into(),
                    "company_name".into(),
                    "company_website".into(),
                ],
            )
            .create(vec![
                expected.name.clone().unwrap(),
                expected.description.clone().unwrap(),
                expected.company_name.clone().unwrap(),
                expected.company_website.as_ref().unwrap().to_string(),
            ])
            .set(1);
            m.gauge_family("consensus_version".into(), vec!["desc".into()])
                .create(vec![expected.client_version.clone().unwrap()])
                .set(1);
        }
        let metrics_text = tide_disco::metrics::Metrics::export(&metrics).unwrap();

        let json_route = {
            let json = json_content.clone();
            warp::path("json").map(move || {
                warp::reply::with_header(json.clone(), "content-type", "application/json")
            })
        };

        let plain_route = {
            let json = json_content.clone();
            warp::path("plain")
                .map(move || warp::reply::with_header(json.clone(), "content-type", "text/plain"))
        };

        let metrics_route = {
            let metrics = metrics_text.clone();
            warp::path("metrics").map(move || {
                warp::reply::with_header(metrics.clone(), "content-type", "text/plain")
            })
        };

        let routes = json_route.or(plain_route).or(metrics_route);

        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0u16))
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let server = warp::serve(routes).incoming(listener).run();
        println!("server listening on http://{addr}");
        let server_handle = spawn(server);

        tokio::time::sleep(Duration::from_secs(1)).await;

        let fetcher = HttpMetadataFetcher {
            allow_local: true,
            ..Default::default()
        };

        let uri: Url = format!("http://{addr}/json").parse().unwrap();
        let result = fetcher.fetch_content(&uri).await.unwrap();
        assert_eq!(result, expected,);

        let uri: Url = format!("http://{addr}/plain").parse().unwrap();
        let result = fetcher.fetch_content(&uri).await.unwrap();
        assert_eq!(result, expected,);

        let uri: Url = format!("http://{addr}/metrics").parse().unwrap();
        let result = fetcher.fetch_content(&uri).await.unwrap();
        assert_eq!(result, expected);

        server_handle.abort();
        server_handle.await.ok();
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_malicious_uri_file() {
        let url = Url::parse("file:///etc/hosts").unwrap();
        let err = HttpMetadataFetcher::default()
            .check_metadata_uri_safety(&url)
            .await
            .unwrap_err();
        tracing::info!("metadata sanitization failed as expected: {err:#}");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert!(err.to_string().contains("URI does not have a host"));
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_malicious_uri_loopback() {
        let url = Url::parse("http://localhost:8080").unwrap();
        let err = HttpMetadataFetcher::default()
            .check_metadata_uri_safety(&url)
            .await
            .unwrap_err();
        tracing::info!("metadata sanitization failed as expected: {err:#}");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert!(
            err.to_string()
                .contains("for host localhost: IP 127.0.0.1 is in reserved range")
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_malicious_uri_ipv4_loopback() {
        let url = Url::parse("http://127.0.0.1:8080").unwrap();
        let err = HttpMetadataFetcher::default()
            .check_metadata_uri_safety(&url)
            .await
            .unwrap_err();
        tracing::info!("metadata sanitization failed as expected: {err:#}");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert!(
            err.to_string()
                .contains("IP 127.0.0.1 is in reserved range")
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_malicious_uri_ipv4_link_local() {
        let url = Url::parse("http://169.254.169.254:8080").unwrap();
        let err = HttpMetadataFetcher::default()
            .check_metadata_uri_safety(&url)
            .await
            .unwrap_err();
        tracing::info!("metadata sanitization failed as expected: {err:#}");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert!(
            err.to_string()
                .contains("IP 169.254.169.254 is in reserved range")
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_malicious_uri_ipv6_embedded_ipv4() {
        // Try to launder a malicious IPv4 address through an IPv6 address.
        let ipv4 = Ipv4Addr::from_str("169.254.169.254").unwrap();
        let ipv6 = ipv4.to_ipv6_mapped();
        tracing::info!(%ipv4, %ipv6);

        let url = Url::parse(&format!("http://[{ipv6}]:8080")).unwrap();
        let err = HttpMetadataFetcher::default()
            .check_metadata_uri_safety(&url)
            .await
            .unwrap_err();
        tracing::info!("metadata sanitization failed as expected: {err:#}");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert!(
            err.to_string()
                .contains("IP 169.254.169.254 is in reserved range")
        );
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_malicious_uri_ipv6_loopback() {
        let url = Url::parse("http://[::1]:8080").unwrap();
        let err = HttpMetadataFetcher::default()
            .check_metadata_uri_safety(&url)
            .await
            .unwrap_err();
        tracing::info!("metadata sanitization failed as expected: {err:#}");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert!(err.to_string().contains("IP ::1 is in reserved range"));
    }

    #[tokio::test]
    #[test_log::test]
    async fn test_malicious_uri_ipv6_link_local() {
        let url = Url::parse("http://[fe80::ffff:ffff:ffff:ffff]:8080").unwrap();
        let err = HttpMetadataFetcher::default()
            .check_metadata_uri_safety(&url)
            .await
            .unwrap_err();
        tracing::info!("metadata sanitization failed as expected: {err:#}");
        assert_eq!(err.status(), StatusCode::BAD_REQUEST);
        assert!(
            err.to_string()
                .contains("IP fe80::ffff:ffff:ffff:ffff is in reserved range")
        );
    }
}
