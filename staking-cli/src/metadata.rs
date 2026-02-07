//! Metadata fetching and validation for validator nodes.

use std::{fmt, str::FromStr, time::Duration};

use anyhow::{bail, Context, Result};
use hotshot_types::signature_key::BLSPubKey;
use url::Url;

// Re-export types from submodules for convenience
pub use crate::metadata_types::NodeMetadataContent;
pub use crate::openmetrics::parse_openmetrics;

/// Fetch metadata from a URI, auto-detecting JSON vs OpenMetrics format.
///
/// Format detection:
/// - If Content-Type header contains "application/json", parse as JSON
/// - Otherwise, parse as OpenMetrics/Prometheus format
pub async fn fetch_metadata(url: &Url) -> Result<NodeMetadataContent> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .context("failed to build HTTP client")?;

    let response = client
        .get(url.as_str())
        .send()
        .await
        .with_context(|| format!("failed to fetch metadata from {url}"))?
        .error_for_status()
        .context("metadata URI returned error status")?;

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let is_json = content_type.contains("application/json");

    if is_json {
        response
            .json()
            .await
            .context("failed to parse metadata as JSON")
    } else {
        let text = response
            .text()
            .await
            .context("failed to read response body")?;
        parse_openmetrics(&text)
    }
}

/// Fetch and validate metadata from a URI, ensuring the pub_key matches.
pub async fn validate_metadata_uri(
    uri: &Url,
    expected_pub_key: &BLSPubKey,
) -> Result<NodeMetadataContent> {
    let content = fetch_metadata(uri).await?;

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

    /// Returns the inner URL if present.
    pub fn url(&self) -> Option<&Url> {
        self.0.as_ref()
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

#[cfg(test)]
fn generate_bls_pub_key() -> BLSPubKey {
    use jf_signature::bls_over_bn254::KeyPair;
    let keypair = KeyPair::generate(&mut rand::thread_rng());
    BLSPubKey::from(keypair.ver_key())
}

#[cfg(test)]
mod test {
    use super::*;

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
    use warp::Filter;

    use super::*;
    use crate::deploy::serve_on_random_port;

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

        let port = serve_on_random_port(route).await;
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

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pub_key mismatch"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_metadata_not_json_parses_as_openmetrics() {
        // When content-type is not JSON, we try to parse as OpenMetrics
        // HTML content will fail OpenMetrics parsing
        let route = warp::path("metadata")
            .map(|| warp::reply::with_header("<html>Not JSON</html>", "content-type", "text/html"));

        let port = serve_on_random_port(route).await;
        let bls_vk = generate_bls_pub_key();
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_err());
        // Now it fails on OpenMetrics parsing (missing consensus_node metric)
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing required consensus_node metric"));
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_metadata_json_content_type() {
        let bls_vk = generate_bls_pub_key();
        let metadata = NodeMetadataContent {
            pub_key: bls_vk,
            name: Some("JSON Validator".to_string()),
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

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let content = fetch_metadata(&uri).await.unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert_eq!(content.name, Some("JSON Validator".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_metadata_openmetrics() {
        let bls_vk = generate_bls_pub_key();
        let metrics_body = format!(
            r#"# HELP consensus_node node
# TYPE consensus_node gauge
consensus_node{{key="{}"}} 1
# HELP consensus_node_identity_general node_identity_general
# TYPE consensus_node_identity_general gauge
consensus_node_identity_general{{name="OpenMetrics Validator",company_name="Test Corp"}} 1
"#,
            bls_vk
        );

        let route = warp::path("metrics").map(move || {
            warp::reply::with_header(
                metrics_body.clone(),
                "content-type",
                "text/plain; charset=utf-8",
            )
        });

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metrics", port)).unwrap();
        let content = fetch_metadata(&uri).await.unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert_eq!(content.name, Some("OpenMetrics Validator".to_string()));
        assert_eq!(content.company_name, Some("Test Corp".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_metadata_follows_redirect() {
        let bls_vk = generate_bls_pub_key();
        let metadata = NodeMetadataContent {
            pub_key: bls_vk,
            name: Some("Redirected Validator".to_string()),
            description: None,
            company_name: None,
            company_website: None,
            client_version: None,
            icon: None,
        };
        let json_body = serde_json::to_string(&metadata).unwrap();

        // We need the port before creating the redirect route, so bind listener first
        let listener =
            tokio::net::TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 0u16)))
                .await
                .unwrap();
        let port = listener.local_addr().unwrap().port();

        // Final destination
        let final_route = warp::path("final").map(move || {
            warp::reply::with_header(json_body.clone(), "content-type", "application/json")
        });

        // Redirect route - now we know the port
        let redirect_route = warp::path("redirect").map(move || {
            warp::reply::with_header(
                warp::reply::with_status("", warp::http::StatusCode::TEMPORARY_REDIRECT),
                "location",
                format!("http://127.0.0.1:{}/final", port),
            )
        });

        let routes = redirect_route.or(final_route);
        tokio::spawn(warp::serve(routes).incoming(listener).run());

        let uri = Url::parse(&format!("http://127.0.0.1:{}/redirect", port)).unwrap();
        let content = fetch_metadata(&uri).await.unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert_eq!(content.name, Some("Redirected Validator".to_string()));
    }
}
