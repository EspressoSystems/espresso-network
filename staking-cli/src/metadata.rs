use std::{fmt, str::FromStr, time::Duration};

use anyhow::{bail, Context, Result};
use hotshot_types::signature_key::BLSPubKey;
use serde::{Deserialize, Serialize};
use url::Url;

// Schema types copied from:
// https://github.com/EspressoSystems/staking-ui-service/blob/main/src/types/common.rs#L194-L273

/// Optional descriptive information about a node.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NodeMetadataContent {
    /// The public key of the node this metadata belongs to.
    ///
    /// This is the only required field of the [`NodeMetadataContent`]. It is included in the
    /// metadata content for authentication purposes. If this does not match the public key of the
    /// node whose metadata is being fetched, then the metadata is treated as invalid. This feature
    /// applies in two scenarios:
    ///
    /// 1. The operator of the node has innocently but erroneously pointed the node's metadata URI
    ///    to the metadata page for a different node (this is an easy mistake to make when running
    ///    multiple nodes). In this case we will detect the error and display no metadata for the
    ///    misconfigured node, which is better for users than displaying incorrect metadata, and is
    ///    a clear sign to the operator that something is wrong.
    ///
    /// 2. A malicious operator attempts to impersonate a trusted party by setting the metadata URI
    ///    for the malicious node to the metadata URI of some existing trusted node (e.g.
    ///    `https://trusted-operator.com/metadata`). Users of the UI see that the malicious node is
    ///    associated with a `trusted-operator.com` domain name and thus believe it to be more
    ///    trustworthy than it perhaps is. We would detect this, since the malicious operator and
    ///    the trusted operator must have nodes with different public keys, and we would display
    ///    no metadata for the malicious operator.
    ///
    /// Note that the mere presence of a matching public key in a metadata dump does not in itself
    /// guarantee that this metadata was intended for this node. The metadata must also have been
    /// sourced from the URI that was registered for that node in the contract. Specifically:
    /// * A metadata dump having the expected public key ensures that the operator of the web site
    ///   which served the metadata intended it for that particular node.
    /// * A node having a certain metadata URI in the contract ensures that the operator of the
    ///   _node_ intended its metadata to be sourced from that particular web site.
    pub pub_key: BLSPubKey,

    /// Human-readable name for the node.
    pub name: Option<String>,

    /// Longer description of the node.
    pub description: Option<String>,

    /// Company or individual operating the node.
    pub company_name: Option<String>,

    /// Website for `company_name`.
    pub company_website: Option<Url>,

    /// Consensus client the node is running.
    pub client_version: Option<String>,

    /// Icon for the node (at different resolutions and pixel aspect ratios).
    pub icon: Option<ImageSet>,
}

/// Different versions of the same image, at different resolutions and pixel aspect ratios.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ImageSet {
    /// 14x14 icons at different pixel ratios.
    #[serde(rename = "14x14")]
    pub small: RatioSet,

    /// 24x24 icons at different pixel ratios.
    #[serde(rename = "24x24")]
    pub large: RatioSet,
}

/// Different versions of the same image, at different pixel aspect ratios.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct RatioSet {
    /// Image source for 1:1 pixel aspect ratio
    #[serde(rename = "@1x")]
    pub ratio1: Option<Url>,

    /// Image source for 2:1 pixel aspect ratio
    #[serde(rename = "@2x")]
    pub ratio2: Option<Url>,

    /// Image source for 3:1 pixel aspect ratio
    #[serde(rename = "@3x")]
    pub ratio3: Option<Url>,
}

/// Fetch and validate metadata from a URI, ensuring the pub_key matches.
pub async fn validate_metadata_uri(
    uri: &Url,
    expected_pub_key: &BLSPubKey,
) -> Result<NodeMetadataContent> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .context("failed to build HTTP client")?;

    let response = client
        .get(uri.as_str())
        .send()
        .await
        .with_context(|| format!("failed to fetch metadata from {uri}"))?
        .error_for_status()
        .context("metadata URI returned error status")?;

    let content: NodeMetadataContent = response
        .json()
        .await
        .context("failed to parse metadata as JSON")?;

    if &content.pub_key != expected_pub_key {
        bail!(
            "metadata pub_key mismatch: expected {}, got {}",
            expected_pub_key,
            content.pub_key
        );
    }

    Ok(content)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataUri(Option<Url>);

impl MetadataUri {
    pub fn empty() -> Self {
        Self(None)
    }
}

impl TryFrom<Url> for MetadataUri {
    type Error = anyhow::Error;

    fn try_from(url: Url) -> Result<Self> {
        if url.as_str().len() > 2048 {
            bail!("metadata URI cannot exceed 2048 bytes");
        }
        Ok(Self(Some(url)))
    }
}

impl FromStr for MetadataUri {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let url = Url::parse(s)?;
        MetadataUri::try_from(url)
    }
}

impl fmt::Display for MetadataUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(url) => write!(f, "{}", url),
            None => Ok(()),
        }
    }
}

impl MetadataUri {
    /// Returns the inner URL if present.
    pub fn url(&self) -> Option<&Url> {
        self.0.as_ref()
    }
}

#[cfg(test)]
mod test {
    use jf_signature::bls_over_bn254::KeyPair;

    use super::*;

    fn generate_bls_pub_key() -> BLSPubKey {
        let keypair = KeyPair::generate(&mut rand::thread_rng());
        BLSPubKey::from(keypair.ver_key())
    }

    #[test]
    fn test_empty_metadata_uri() {
        let empty = MetadataUri::empty();
        assert_eq!(empty.to_string(), "");
    }

    #[test]
    fn test_valid_metadata_uri() -> Result<()> {
        let uri: MetadataUri = "https://example.com/metadata".parse()?;
        assert_eq!(uri.to_string(), "https://example.com/metadata");
        Ok(())
    }

    #[test]
    fn test_metadata_uri_max_length() -> Result<()> {
        let long_path = "a".repeat(2000);
        let url_str = format!("https://example.com/{}", long_path);
        let uri: MetadataUri = url_str.parse()?;
        assert_eq!(uri.to_string(), url_str);
        Ok(())
    }

    #[test]
    fn test_metadata_uri_too_long() {
        let long_path = "a".repeat(2100);
        let url_str = format!("https://example.com/{}", long_path);
        let result = url_str.parse::<MetadataUri>();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot exceed 2048 bytes"));
    }

    #[test]
    fn test_metadata_uri_url_accessor() -> Result<()> {
        let uri: MetadataUri = "https://example.com/metadata".parse()?;
        assert!(uri.url().is_some());
        assert_eq!(uri.url().unwrap().as_str(), "https://example.com/metadata");

        let empty = MetadataUri::empty();
        assert!(empty.url().is_none());
        Ok(())
    }

    #[test]
    fn test_parse_valid_metadata_full() {
        let bls_vk = generate_bls_pub_key();
        let json = serde_json::json!({
            "pub_key": bls_vk.to_string(),
            "name": "Test Validator",
            "description": "A test validator node",
            "company_name": "Test Corp",
            "company_website": "https://test.com",
            "client_version": "1.0.0",
            "icon": {
                "14x14": {
                    "@1x": "https://example.com/icon-14-1x.png",
                    "@2x": "https://example.com/icon-14-2x.png",
                    "@3x": "https://example.com/icon-14-3x.png"
                },
                "24x24": {
                    "@1x": "https://example.com/icon-24-1x.png",
                    "@2x": "https://example.com/icon-24-2x.png",
                    "@3x": "https://example.com/icon-24-3x.png"
                }
            }
        });

        let content: NodeMetadataContent = serde_json::from_value(json).unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert_eq!(content.name, Some("Test Validator".to_string()));
        assert_eq!(
            content.description,
            Some("A test validator node".to_string())
        );
        assert_eq!(content.company_name, Some("Test Corp".to_string()));
        assert_eq!(
            content.company_website,
            Some(Url::parse("https://test.com").unwrap())
        );
        assert_eq!(content.client_version, Some("1.0.0".to_string()));
        assert!(content.icon.is_some());
        let icon = content.icon.unwrap();
        assert_eq!(
            icon.small.ratio1,
            Some(Url::parse("https://example.com/icon-14-1x.png").unwrap())
        );
        assert_eq!(
            icon.large.ratio3,
            Some(Url::parse("https://example.com/icon-24-3x.png").unwrap())
        );
    }

    #[test]
    fn test_parse_valid_metadata_minimal() {
        let bls_vk = generate_bls_pub_key();
        let json = serde_json::json!({
            "pub_key": bls_vk.to_string()
        });

        let content: NodeMetadataContent = serde_json::from_value(json).unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert!(content.name.is_none());
        assert!(content.description.is_none());
        assert!(content.company_name.is_none());
        assert!(content.company_website.is_none());
        assert!(content.client_version.is_none());
        assert!(content.icon.is_none());
    }

    #[test]
    fn test_parse_missing_pub_key() {
        let json = serde_json::json!({
            "name": "Test Validator"
        });

        let result: Result<NodeMetadataContent, _> = serde_json::from_value(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pub_key"));
    }

    #[test]
    fn test_parse_invalid_json() {
        let invalid_json = "{ not valid json }";
        let result: Result<NodeMetadataContent, _> = serde_json::from_str(invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_extra_fields_ignored() {
        let bls_vk = generate_bls_pub_key();
        let json = serde_json::json!({
            "pub_key": bls_vk.to_string(),
            "unknownField": "should be ignored",
            "anotherUnknown": 12345
        });

        let content: NodeMetadataContent = serde_json::from_value(json).unwrap();
        assert_eq!(content.pub_key, bls_vk);
    }
}

#[cfg(all(test, feature = "testing"))]
mod validation_tests {
    use jf_signature::bls_over_bn254::KeyPair;
    use warp::Filter;

    use super::*;

    fn generate_bls_pub_key() -> BLSPubKey {
        let keypair = KeyPair::generate(&mut rand::thread_rng());
        BLSPubKey::from(keypair.ver_key())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_metadata_correct_pub_key() {
        let bls_vk = generate_bls_pub_key();
        let metadata = NodeMetadataContent {
            pub_key: bls_vk,
            name: Some("Test Validator".to_string()),
            description: None,
            company_name: None,
            company_website: None,
            client_version: None,
            icon: None,
        };
        let json_body = serde_json::to_string(&metadata).unwrap();

        let route = warp::path("metadata").map(move || {
            warp::reply::with_header(json_body.clone(), "content-type", "application/json")
        });

        let port = portpicker::pick_unused_port().unwrap();
        tokio::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert_eq!(content.name, Some("Test Validator".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_metadata_wrong_pub_key() {
        let bls_vk = generate_bls_pub_key();
        let different_bls_vk = generate_bls_pub_key();

        let metadata = NodeMetadataContent {
            pub_key: different_bls_vk,
            name: None,
            description: None,
            company_name: None,
            company_website: None,
            client_version: None,
            icon: None,
        };
        let json_body = serde_json::to_string(&metadata).unwrap();

        let route = warp::path("metadata").map(move || {
            warp::reply::with_header(json_body.clone(), "content-type", "application/json")
        });

        let port = portpicker::pick_unused_port().unwrap();
        tokio::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pub_key mismatch"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_metadata_not_json() {
        let route = warp::path("metadata")
            .map(|| warp::reply::with_header("<html>Not JSON</html>", "content-type", "text/html"));

        let port = portpicker::pick_unused_port().unwrap();
        tokio::spawn(warp::serve(route).run(([127, 0, 0, 1], port)));
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let bls_vk = generate_bls_pub_key();
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("failed to parse metadata as JSON"));
    }

    #[tokio::test]
    async fn test_validate_metadata_fetch_timeout() {
        // Use a non-routable IP to trigger a timeout
        let bls_vk = generate_bls_pub_key();
        let uri = Url::parse("http://10.255.255.1:12345/metadata").unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("failed to fetch metadata from"));
    }
}
