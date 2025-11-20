use std::fmt;

use anyhow::{bail, Result};
use url::Url;

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

impl fmt::Display for MetadataUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            Some(url) => write!(f, "{}", url),
            None => Ok(()),
        }
    }
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
        let url = Url::parse("https://example.com/metadata")?;
        let uri = MetadataUri::try_from(url)?;
        assert_eq!(uri.to_string(), "https://example.com/metadata");
        Ok(())
    }

    #[test]
    fn test_metadata_uri_max_length() -> Result<()> {
        let long_path = "a".repeat(2000);
        let url = Url::parse(&format!("https://example.com/{}", long_path))?;
        let uri = MetadataUri::try_from(url.clone())?;
        assert_eq!(uri.to_string(), url.as_str());
        Ok(())
    }

    #[test]
    fn test_metadata_uri_too_long() {
        let long_path = "a".repeat(2100);
        let url = Url::parse(&format!("https://example.com/{}", long_path)).unwrap();
        let result = MetadataUri::try_from(url);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("cannot exceed 2048 bytes"));
    }
}
