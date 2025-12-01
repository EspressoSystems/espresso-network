//! Dealing with node metadata.

use std::time::Duration;

use crate::{
    Error, Result,
    error::ResultExt,
    types::common::{ImageSet, NodeMetadata, NodeMetadataContent},
};
use prometheus_parse::Scrape;
use reqwest::{Url, header::HeaderValue};
use tide_disco::http::mime::{JSON, PLAIN};
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

    /// Download and parse node metadata content from a third-party URI.
    fn fetch_content(&self, url: &Url) -> impl Send + Future<Output = Result<NodeMetadataContent>>;
}

/// Object that can download metadata from a given URI.
#[derive(Clone, Debug)]
pub struct HttpMetadataFetcher {
    client: reqwest::Client,
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
        }
    }
}

impl MetadataFetcher for HttpMetadataFetcher {
    #[instrument(skip(self))]
    async fn fetch_content(&self, url: &Url) -> Result<NodeMetadataContent> {
        let res =
            self.client.get(url.clone()).send().await.context(|| {
                Error::internal().context(format!("downloading metadata from {url}"))
            })?;

        let content_type = res.headers().get("Content-Type");
        if content_type == Some(&HeaderValue::from_str(&JSON.to_string()).unwrap()) {
            // Parse response as JSON.
            res.json().await.context(|| {
                Error::internal().context(format!("malformed JSON metadata from {url}"))
            })
        } else if content_type.is_none()
            || content_type == Some(&HeaderValue::from_str(&PLAIN.to_string()).unwrap())
        {
            // Parse response as a set of prometheus metrics.
            let text = res.text().await.context(|| {
                Error::internal().context(format!("reading metadata response from {url}"))
            })?;
            parse_prometheus(&text).map_err(|err| err.context(format!("from {url}")))
        } else {
            Err(Error::internal().context(format!("unrecognized content type {content_type:?}")))
        }
    }
}

/// Interpret prometheus labels as node metadata according to Espresso convention.
fn parse_prometheus(text: &str) -> Result<NodeMetadataContent> {
    let metrics = Scrape::parse(text.lines().map(String::from).map(Ok))
        .context(|| Error::internal().context("malformed prometheus metadata"))?;

    let mut metadata = NodeMetadataContent::default();
    for sample in metrics.samples {
        match sample.metric.as_str() {
            "consensus_node_identity_general" => {
                for (label, value) in sample.labels.iter() {
                    match label.as_str() {
                        "name" => metadata.name = Some(value.clone()),
                        "description" => metadata.description = Some(value.clone()),
                        "company_name" => metadata.company_name = Some(value.clone()),
                        "company_website" => match value.parse() {
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
    use std::{borrow::Cow, sync::Arc};

    use crate::types::common::{ImageSet, RatioSet};

    use super::*;

    use async_lock::RwLock;
    use futures::FutureExt;
    use hotshot_query_service::metrics::PrometheusMetrics;
    use hotshot_types::traits::metrics::Metrics;
    use portpicker::pick_unused_port;
    use pretty_assertions::assert_eq;
    use tokio::spawn;
    use toml::toml;
    use vbs::version::{StaticVersion, StaticVersionType};

    #[test_log::test]
    fn test_parse_prometheus_all_fields() {
        let text = r#"
        # HELP consensus_version version
        # TYPE consensus_version gauge
        consensus_version{desc="db0900d",rev="db0900dc7bc539479dfc58c5ace8f3aed736491a",timestamp="2025-11-21T14:40:40.000000000-05:00"} 1
        # HELP consensus_node_identity_general node_identity_general
        # TYPE consensus_node_identity_general gauge
        consensus_node_identity_general{company_name="Espresso Systems",company_website="https://www.espressosys.com/",name="test-node",description="a test node",network_type="AWS",node_type="espresso-sequencer 0.1.0",operating_system="Linux"} 1
        # HELP consensus_node_identity_location node_identity_location
        # TYPE consensus_node_identity_location gauge
        consensus_node_identity_location{country="US",latitude="39.96264474822646",longitude="-83.00332937380611"} 1
        # HELP consensus_node_identity_icon node_identity_icon
        # TYPE consensus_node_identity_icon gauge
        consensus_node_identity_icon{small_1x="https://www.espressosys.com/small-icon.png",large_2x="https://www.espressosys.com/large-icon.png","medium_1x"="https://www.espressosys.com/ignored-icon.png"} 1
        "#;
        let metadata = parse_prometheus(text).unwrap();
        assert_eq!(
            metadata,
            NodeMetadataContent {
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
        let text = r#"
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
        consensus_node{key="BLS_VER_KEY~widXdplI_m2zFsHBNxCmo5GfJa0VZtkPI88pRal6eRP9ZDwZth4iMXHTruzDFhnJW6-g3LVr3JJHUG6P3-IVECesRGFjvOEM4TofF2CCPD16uSGYJMpbgWKyw1x2OQYpZTkfDqtwTUtCWRrTNiFfJZYEJfyeQwUACFfF8fCNFwJZ"} 1
        "#;
        assert_eq!(
            parse_prometheus(text).unwrap(),
            NodeMetadataContent::default()
        );
    }

    #[test_log::test]
    fn test_parse_prometheus_invalid_urls() {
        let text = r#"
        consensus_node_identity_general{company_website="notaurl"} 1
        consensus_node_identity_icon{small_1x="notaurl"} 1
        "#;
        assert_eq!(
            parse_prometheus(text).unwrap(),
            NodeMetadataContent {
                icon: Some(ImageSet::default()),
                ..Default::default()
            }
        );
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
        let expected = NodeMetadataContent {
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

        let fetcher = HttpMetadataFetcher::default();
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
    async fn test_fetch_prometheus() {
        let expected = NodeMetadataContent {
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

        let fetcher = HttpMetadataFetcher::default();
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
}
