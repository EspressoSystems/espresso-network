//! Payload bytes as the store answers them: inline, or fetched from where the store says they
//! are.

use std::{io::SeekFrom, ops::Range, time::Duration};

use anyhow::ensure;
use bytes::Bytes;
use reqwest::{StatusCode, header::RANGE};
use store_api::{PayloadAt, PayloadData, Store};
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt},
};
use url::Url;

/// How long fetching a payload from where the store says it is may take, connecting included.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// The client payloads are fetched with.
pub(crate) fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(FETCH_TIMEOUT)
        .build()
        .expect("a client with timeouts only")
}

/// The payload `at` names, or `range` of it, once the store can serve it.
pub(crate) async fn read(
    store: &dyn Store,
    http: &reqwest::Client,
    at: &PayloadAt,
    range: Option<Range<u64>>,
) -> anyhow::Result<Option<Bytes>> {
    match store.payload(at, range.clone()).await? {
        None => Ok(None),
        Some(PayloadData::Inline(bytes)) => Ok(Some(bytes)),
        Some(PayloadData::Url(url)) => fetch(http, &url, range).await,
    }
}

/// The object at `url`, or `range` of it; `None` while there is none, as before a payload is
/// uploaded. A `file` URL is read from the local file system, as where the API instance shares a
/// host with the object store.
pub(crate) async fn fetch(
    http: &reqwest::Client,
    url: &str,
    range: Option<Range<u64>>,
) -> anyhow::Result<Option<Bytes>> {
    let parsed = Url::parse(url)?;
    if parsed.scheme() != "file" {
        return fetch_http(http, url, range).await;
    }
    let path = parsed
        .to_file_path()
        .map_err(|()| anyhow::anyhow!("not a local file: {url}"))?;
    let mut file = match File::open(&path).await {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let Some(range) = range else {
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        return Ok(Some(bytes.into()));
    };
    let len = file.metadata().await?.len();
    ensure!(range.end <= len, "a range past the end of the payload");
    file.seek(SeekFrom::Start(range.start)).await?;
    let mut bytes = vec![0; usize::try_from(range.end - range.start)?];
    file.read_exact(&mut bytes).await?;
    Ok(Some(bytes.into()))
}

/// The object at the HTTP `url`, or `range` of it.
async fn fetch_http(
    http: &reqwest::Client,
    url: &str,
    range: Option<Range<u64>>,
) -> anyhow::Result<Option<Bytes>> {
    let mut request = http.get(url);
    if let Some(range) = &range {
        if range.is_empty() {
            return Ok(Some(Bytes::new()));
        }
        request = request.header(RANGE, format!("bytes={}-{}", range.start, range.end - 1));
    }
    let response = request.send().await?;
    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = response.error_for_status()?;
    let partial = response.status() == StatusCode::PARTIAL_CONTENT;
    let bytes = response.bytes().await?;
    match range {
        // A server that does not take ranges sends the whole object.
        Some(range) if !partial => {
            let (start, end) = (usize::try_from(range.start)?, usize::try_from(range.end)?);
            ensure!(end <= bytes.len(), "a range past the end of the payload");
            Ok(Some(bytes.slice(start..end)))
        },
        _ => Ok(Some(bytes)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `file` URL is read from the file system, whole or by range, and a missing file is no
    /// payload yet.
    #[tokio::test]
    async fn reads_a_file_url() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("payload");
        tokio::fs::write(&path, b"0123456789").await.unwrap();
        let url = Url::from_file_path(&path).unwrap().to_string();
        let http = client();

        let whole = fetch(&http, &url, None).await.unwrap();
        assert_eq!(whole.as_deref(), Some(&b"0123456789"[..]));
        let range = fetch(&http, &url, Some(2..5)).await.unwrap();
        assert_eq!(range.as_deref(), Some(&b"234"[..]));
        fetch(&http, &url, Some(8..11))
            .await
            .expect_err("a range past the end");
        let missing = Url::from_file_path(dir.path().join("none"))
            .unwrap()
            .to_string();
        assert_eq!(fetch(&http, &missing, None).await.unwrap(), None);
    }
}
