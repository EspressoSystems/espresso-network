//! Metadata fetching and validation for validator nodes.

use std::{fmt, str::FromStr, time::Duration};

use anyhow::{bail, Context, Result};
use hotshot_types::signature_key::BLSPubKey;
use thiserror::Error;
use url::Url;

// Re-export types from submodules for convenience
pub use crate::metadata_types::NodeMetadataContent;
pub use crate::openmetrics::parse_openmetrics;

/// Errors that can occur when fetching or parsing metadata.
///
/// Error variants indicate what was attempted:
/// - `SchemaError`: Content was valid JSON syntax but didn't match our schema.
///   OpenMetrics parsing was not attempted because JSON syntax was valid.
/// - `BothFormatsFailed`: Content had invalid JSON syntax, so both JSON and
///   OpenMetrics parsing were attempted and both failed.
#[derive(Debug, Error)]
pub enum MetadataError {
    /// Valid JSON syntax but doesn't match the expected schema.
    ///
    /// This means the content parsed as JSON but was missing required fields
    /// (like `pub_key`) or had incorrect types. OpenMetrics parsing was not
    /// attempted because the content was valid JSON.
    #[error("valid JSON but incorrect schema: {0}")]
    SchemaError(#[source] serde_json::Error),

    /// Neither JSON nor OpenMetrics parsing succeeded.
    ///
    /// This means the content had invalid JSON syntax, so we tried parsing
    /// as OpenMetrics format but that also failed.
    #[error("failed to parse as JSON ({json_error}) or OpenMetrics ({openmetrics_error})")]
    BothFormatsFailed {
        json_error: serde_json::Error,
        openmetrics_error: anyhow::Error,
    },

    /// Response body was empty.
    #[error("empty response body")]
    EmptyBody,

    /// HTTP or network error occurred while fetching.
    #[error("failed to fetch metadata")]
    FetchError(#[from] reqwest::Error),
}

/// Fetch metadata from a URI, auto-detecting JSON vs OpenMetrics format.
///
/// Format detection:
/// - Ignores Content-Type header (some hosts like GitHub raw serve JSON as text/plain)
/// - Always tries JSON parsing first
/// - Falls back to OpenMetrics/Prometheus format if JSON fails
pub async fn fetch_metadata(url: &Url) -> Result<NodeMetadataContent, MetadataError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()?;

    let response = client.get(url.as_str()).send().await?.error_for_status()?;

    let text = response.text().await?;

    if text.is_empty() {
        return Err(MetadataError::EmptyBody);
    }

    // Parse in two explicit steps:
    // 1. Validate JSON syntax
    // 2. Validate schema
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(json_value) => {
            // Valid JSON syntax, now try our schema
            serde_json::from_value(json_value).map_err(MetadataError::SchemaError)
        },
        Err(json_err) => {
            // Not valid JSON, try OpenMetrics
            parse_openmetrics(&text).map_err(|openmetrics_error| MetadataError::BothFormatsFailed {
                json_error: json_err,
                openmetrics_error,
            })
        },
    }
}

/// Fetch and validate metadata from a URI, ensuring the pub_key matches.
pub async fn validate_metadata_uri(
    uri: &Url,
    expected_pub_key: &BLSPubKey,
) -> Result<NodeMetadataContent> {
    let content = fetch_metadata(uri)
        .await
        .with_context(|| format!("from {uri}"))?;

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
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(err_msg.contains("missing field"));
        assert!(err_msg.contains("pub_key"));
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
    use pretty_assertions::assert_matches;
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

        let route = warp::any().map(move || {
            warp::reply::with_header(json_body.clone(), "content-type", "application/json")
        });

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/", port)).unwrap();
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

        let route = warp::any().map(move || {
            warp::reply::with_header(json_body.clone(), "content-type", "application/json")
        });

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/", port)).unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pub_key mismatch"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_metadata_json_with_text_plain_content_type() {
        // Test that JSON parses correctly even with text/plain content-type (GitHub raw scenario)
        let bls_vk = generate_bls_pub_key();
        let metadata = NodeMetadataContent {
            pub_key: bls_vk,
            name: Some("Text Plain JSON".to_string()),
            description: None,
            company_name: None,
            company_website: None,
            client_version: None,
            icon: None,
        };
        let json_body = serde_json::to_string(&metadata).unwrap();

        let route = warp::any()
            .map(move || warp::reply::with_header(json_body.clone(), "content-type", "text/plain"));

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/", port)).unwrap();
        let content = fetch_metadata(&uri).await.unwrap();
        assert_eq!(content.pub_key, bls_vk);
        assert_eq!(content.name, Some("Text Plain JSON".to_string()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_metadata_invalid_both_formats_shows_both_errors() {
        // Test that error message shows both JSON and OpenMetrics parsing failures
        let invalid_content = "This is neither valid JSON nor OpenMetrics";

        let route = warp::any()
            .map(move || warp::reply::with_header(invalid_content, "content-type", "text/plain"));

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let err = fetch_metadata(&uri).await.unwrap_err();

        assert_matches!(err, MetadataError::BothFormatsFailed { .. });
        // Error message should mention both JSON and OpenMetrics failures
        let err_msg = err.to_string();
        assert!(err_msg.contains("failed to parse as JSON"));
        assert!(err_msg.contains("OpenMetrics"));
    }

    #[tokio::test]
    async fn test_validate_metadata_fetch_timeout() {
        // Use a non-routable IP to trigger a timeout
        let bls_vk = generate_bls_pub_key();
        let uri = Url::parse("http://10.255.255.1:12345/metadata").unwrap();
        let result = validate_metadata_uri(&uri, &bls_vk).await;
        assert!(result.is_err());
        let err = result.unwrap_err();

        // Error chain should include the URL context and the base fetch error
        let err_chain = format!("{:#}", err); // Display full error chain
        assert!(err_chain.contains("from http://10.255.255.1:12345/metadata"));
        assert!(err_chain.contains("failed to fetch metadata"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_metadata_uri_includes_url_context_for_schema_error() {
        // Valid JSON but wrong schema - verify URL appears in error
        let invalid_json = r#"{"name": "Service", "version": "1.0"}"#;
        let route = warp::any().map(move || {
            warp::reply::with_header(invalid_json, "content-type", "application/json")
        });

        let port = serve_on_random_port(route).await;
        let bls_vk = generate_bls_pub_key();
        let uri = Url::parse(&format!("http://127.0.0.1:{}/test-path", port)).unwrap();
        let err = validate_metadata_uri(&uri, &bls_vk).await.unwrap_err();

        // Error should include URL context from validate_metadata_uri
        let err_msg = format!("{:#}", err);
        assert!(err_msg.contains(&format!("from http://127.0.0.1:{}/test-path", port)));
        assert!(err_msg.contains("valid JSON but incorrect schema"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_validate_metadata_uri_includes_url_context_for_both_formats_failed() {
        // Invalid content - verify URL appears in error
        let invalid_content = "<html>Not JSON or OpenMetrics</html>";
        let route = warp::any()
            .map(move || warp::reply::with_header(invalid_content, "content-type", "text/html"));

        let port = serve_on_random_port(route).await;
        let bls_vk = generate_bls_pub_key();
        let uri = Url::parse(&format!("http://127.0.0.1:{}/custom-endpoint", port)).unwrap();
        let err = validate_metadata_uri(&uri, &bls_vk).await.unwrap_err();

        // Error should include URL context from validate_metadata_uri
        let err_msg = format!("{:#}", err);
        assert!(err_msg.contains(&format!("from http://127.0.0.1:{}/custom-endpoint", port)));
        assert!(err_msg.contains("failed to parse as JSON"));
        assert!(err_msg.contains("OpenMetrics"));
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

        let route = warp::any().map(move || {
            warp::reply::with_header(json_body.clone(), "content-type", "application/json")
        });

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/", port)).unwrap();
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

        let route = warp::any().map(move || {
            warp::reply::with_header(
                metrics_body.clone(),
                "content-type",
                "text/plain; charset=utf-8",
            )
        });

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/", port)).unwrap();
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

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_metadata_empty_body() {
        let route =
            warp::any().map(|| warp::reply::with_header("", "content-type", "application/json"));

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let err = fetch_metadata(&uri).await.unwrap_err();

        assert_matches!(err, MetadataError::EmptyBody);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_metadata_valid_json_wrong_schema() {
        // Valid JSON but doesn't match NodeMetadataContent schema
        let invalid_json = r#"{"name": "Some Service", "version": "1.0"}"#;

        let route = warp::any().map(move || {
            warp::reply::with_header(invalid_json, "content-type", "application/json")
        });

        let port = serve_on_random_port(route).await;
        let uri = Url::parse(&format!("http://127.0.0.1:{}/metadata", port)).unwrap();
        let err = fetch_metadata(&uri).await.unwrap_err();

        assert_matches!(err, MetadataError::SchemaError(_));
        // Should mention the schema validation error with missing required field
        let err_msg = err.to_string();
        assert!(err_msg.contains("valid JSON but incorrect schema"));
        assert!(err_msg.contains("missing field"));
        assert!(err_msg.contains("pub_key"));
    }
}
