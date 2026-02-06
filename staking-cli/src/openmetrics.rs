//! OpenMetrics/Prometheus format parsing for validator metadata.
//!
//! This module is copied from staking-ui-service and should be replaced with
//! imports from that crate once version compatibility is resolved.
//!
//! Source: https://github.com/EspressoSystems/staking-ui-service/blob/main/src/input/l1/metadata.rs

use std::{collections::HashMap, io::BufRead};

use anyhow::{Context, Result};
use hotshot_types::signature_key::BLSPubKey;
use url::Url;

use crate::metadata_types::{ImageSet, NodeMetadataContent, RatioSet};

/// Parse OpenMetrics/Prometheus format text into NodeMetadataContent.
///
/// Extracts metadata from the following metrics:
/// - `consensus_node{key="BLS_VER_KEY~..."}` - pub_key (REQUIRED)
/// - `consensus_node_identity_general{name, description, company_name, company_website}` - identity fields
/// - `consensus_version{desc}` - client_version
/// - `consensus_node_identity_icon{small_1x, small_2x, small_3x, large_1x, large_2x, large_3x}` - icon URLs
pub fn parse_openmetrics(text: &str) -> Result<NodeMetadataContent> {
    let lines = text.as_bytes().lines();
    let scrape = prometheus_parse::Scrape::parse(lines)
        .map_err(|e| anyhow::anyhow!("failed to parse OpenMetrics: {e}"))?;

    let mut pub_key: Option<BLSPubKey> = None;
    let mut name: Option<String> = None;
    let mut description: Option<String> = None;
    let mut company_name: Option<String> = None;
    let mut company_website: Option<Url> = None;
    let mut client_version: Option<String> = None;
    let mut icon_urls: HashMap<String, String> = HashMap::new();

    for sample in &scrape.samples {
        match sample.metric.as_str() {
            "consensus_node" => {
                if let Some(key) = sample.labels.get("key") {
                    pub_key = Some(
                        key.parse()
                            .with_context(|| format!("invalid pub_key format: {key}"))?,
                    );
                }
            },
            "consensus_node_identity_general" => {
                if let Some(v) = sample.labels.get("name") {
                    if !v.is_empty() {
                        name = Some(v.to_string());
                    }
                }
                if let Some(v) = sample.labels.get("description") {
                    if !v.is_empty() {
                        description = Some(v.to_string());
                    }
                }
                if let Some(v) = sample.labels.get("company_name") {
                    if !v.is_empty() {
                        company_name = Some(v.to_string());
                    }
                }
                if let Some(v) = sample.labels.get("company_website") {
                    if !v.is_empty() {
                        company_website = Url::parse(v).ok();
                    }
                }
            },
            "consensus_version" => {
                if let Some(v) = sample.labels.get("desc") {
                    if !v.is_empty() {
                        client_version = Some(v.to_string());
                    }
                }
            },
            "consensus_node_identity_icon" => {
                for (label, value) in sample.labels.iter() {
                    if !value.is_empty() {
                        icon_urls.insert(label.to_string(), value.to_string());
                    }
                }
            },
            _ => {},
        }
    }

    let pub_key = pub_key.ok_or_else(|| {
        anyhow::anyhow!("missing required consensus_node metric with key label in OpenMetrics data")
    })?;

    // Build icon set from collected URLs
    let icon = if icon_urls.is_empty() {
        None
    } else {
        let parse_url = |key: &str| -> Option<Url> {
            icon_urls.get(key).and_then(|v| {
                Url::parse(v)
                    .inspect_err(|e| {
                        tracing::warn!("skipping malformed icon URL for {key}: {e}");
                    })
                    .ok()
            })
        };

        Some(ImageSet {
            small: RatioSet {
                ratio1: parse_url("small_1x"),
                ratio2: parse_url("small_2x"),
                ratio3: parse_url("small_3x"),
            },
            large: RatioSet {
                ratio1: parse_url("large_1x"),
                ratio2: parse_url("large_2x"),
                ratio3: parse_url("large_3x"),
            },
        })
    };

    Ok(NodeMetadataContent {
        pub_key,
        name,
        description,
        company_name,
        company_website,
        client_version,
        icon,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    fn generate_bls_pub_key() -> BLSPubKey {
        use jf_signature::bls_over_bn254::KeyPair;
        let keypair = KeyPair::generate(&mut rand::thread_rng());
        BLSPubKey::from(keypair.ver_key())
    }

    #[test]
    fn test_parse_openmetrics_complete() {
        let bls_vk = generate_bls_pub_key();
        let metrics = format!(
            r#"# HELP consensus_node node
# TYPE consensus_node gauge
consensus_node{{key="{}"}} 1
# HELP consensus_node_identity_general node_identity_general
# TYPE consensus_node_identity_general gauge
consensus_node_identity_general{{company_name="Espresso Systems",company_website="https://www.espressosys.com/",name="sequencer0",description="A test validator"}} 1
# HELP consensus_version version
# TYPE consensus_version gauge
consensus_version{{desc="20240701-15-gbd0957fd-dirty"}} 1
# HELP consensus_node_identity_icon node_identity_icon
# TYPE consensus_node_identity_icon gauge
consensus_node_identity_icon{{small_1x="https://example.com/s1.png",small_2x="https://example.com/s2.png",large_1x="https://example.com/l1.png"}} 1
"#,
            bls_vk
        );

        let content = parse_openmetrics(&metrics).unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert_eq!(content.name, Some("sequencer0".to_string()));
        assert_eq!(content.description, Some("A test validator".to_string()));
        assert_eq!(content.company_name, Some("Espresso Systems".to_string()));
        assert_eq!(
            content.company_website,
            Some(Url::parse("https://www.espressosys.com/").unwrap())
        );
        assert_eq!(
            content.client_version,
            Some("20240701-15-gbd0957fd-dirty".to_string())
        );
        let icon = content.icon.unwrap();
        assert_eq!(
            icon.small.ratio1,
            Some(Url::parse("https://example.com/s1.png").unwrap())
        );
        assert_eq!(
            icon.small.ratio2,
            Some(Url::parse("https://example.com/s2.png").unwrap())
        );
        assert!(icon.small.ratio3.is_none());
        assert_eq!(
            icon.large.ratio1,
            Some(Url::parse("https://example.com/l1.png").unwrap())
        );
    }

    #[test]
    fn test_parse_openmetrics_minimal() {
        let bls_vk = generate_bls_pub_key();
        let metrics = format!(
            r#"# HELP consensus_node node
# TYPE consensus_node gauge
consensus_node{{key="{}"}} 1
"#,
            bls_vk
        );

        let content = parse_openmetrics(&metrics).unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert!(content.name.is_none());
        assert!(content.description.is_none());
        assert!(content.company_name.is_none());
        assert!(content.company_website.is_none());
        assert!(content.client_version.is_none());
        assert!(content.icon.is_none());
    }

    #[test]
    fn test_parse_openmetrics_missing_pubkey() {
        let metrics = r#"# HELP consensus_version version
# TYPE consensus_version gauge
consensus_version{desc="1.0.0"} 1
"#;

        let result = parse_openmetrics(metrics);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing required consensus_node metric"));
    }

    #[test]
    fn test_parse_openmetrics_invalid_pubkey() {
        let metrics = r#"# HELP consensus_node node
# TYPE consensus_node gauge
consensus_node{key="invalid_key_format"} 1
"#;

        let result = parse_openmetrics(metrics);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("invalid pub_key"));
    }

    #[test]
    fn test_parse_openmetrics_malformed_urls() {
        let bls_vk = generate_bls_pub_key();
        let metrics = format!(
            r#"# HELP consensus_node node
# TYPE consensus_node gauge
consensus_node{{key="{}"}} 1
# HELP consensus_node_identity_icon node_identity_icon
# TYPE consensus_node_identity_icon gauge
consensus_node_identity_icon{{small_1x="not a valid url",small_2x="https://valid.com/icon.png"}} 1
"#,
            bls_vk
        );

        // Should succeed, gracefully skipping malformed URLs
        let content = parse_openmetrics(&metrics).unwrap();
        assert_eq!(content.pub_key, bls_vk);
        let icon = content.icon.unwrap();
        assert!(icon.small.ratio1.is_none()); // malformed URL skipped
        assert_eq!(
            icon.small.ratio2,
            Some(Url::parse("https://valid.com/icon.png").unwrap())
        );
    }
}
