//! Where the payloads of a store live: an object store, one object per payload, keyed by its VID
//! commitment, so that anyone who has a block's header knows where its payload is, before the
//! payload is there.
//!
//! Each payload has a record in the store, under its commitment: what describes it, and whether
//! it is uploaded. A height whose block has the payload takes both from the record when its item
//! is written ([`Part::PayloadStored`]), or later, when repair looks for missing payloads.
//!
//! A writer that must report a payload durable before it is uploaded, as a storage instance must,
//! stages it in a local directory first; an [`Uploader`] copies staged payloads to the object
//! store, records them as uploaded, and deletes the staged copies. A restarted uploader uploads
//! what is still staged. A writer without a staging directory uploads at once.

use std::{path::PathBuf, sync::Arc};

use anyhow::Context;
use app_tables::{AppWrite, TableName};
use bytes::Bytes;
use object_store::{ObjectStore, PutPayload, path::Path};
use serde::{Deserialize, Serialize};
use tokio::{fs, io::AsyncWriteExt, sync::mpsc};

use crate::{Backend, PayloadInfo};

/// The table of payload records, by VID commitment.
static RECORDS: std::sync::LazyLock<TableName> =
    std::sync::LazyLock::new(|| TableName::of("payload-records"));

/// What the store knows of a payload it keeps by commitment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PayloadRecord {
    pub info: PayloadInfo,

    /// Whether the payload is in the object store; until then it is staged.
    pub uploaded: bool,
}

/// The record of the payload whose VID commitment is `commitment`.
pub(crate) async fn record(
    backend: &dyn Backend,
    commitment: &str,
) -> anyhow::Result<Option<PayloadRecord>> {
    let Some(bytes) = backend.app_get(&RECORDS, commitment.as_bytes()).await? else {
        return Ok(None);
    };
    Ok(Some(bincode::deserialize(&bytes)?))
}

pub(crate) async fn put_record(
    backend: &dyn Backend,
    commitment: &str,
    record: &PayloadRecord,
) -> anyhow::Result<()> {
    // An upload is not undone by a later write of the same payload.
    let uploaded = record.uploaded
        || self::record(backend, commitment)
            .await?
            .is_some_and(|stored| stored.uploaded);
    let record = PayloadRecord {
        uploaded,
        ..*record
    };
    backend
        .app_write(vec![AppWrite::Put {
            table: RECORDS.clone(),
            key: Bytes::copy_from_slice(commitment.as_bytes()),
            value: bincode::serialize(&record)?.into(),
        }])
        .await
}

/// The object store payloads live in.
#[derive(Clone, Debug)]
pub struct PayloadStore {
    objects: Arc<dyn ObjectStore>,

    /// What every payload's key starts with.
    prefix: Path,

    /// The URL clients fetch the object store's objects from, if they can reach it directly.
    public_url: Option<String>,
}

impl PayloadStore {
    /// Payloads in `objects`, under `prefix`; clients reach `objects` at `public_url`, if at all.
    pub fn new(objects: Arc<dyn ObjectStore>, prefix: &str, public_url: Option<String>) -> Self {
        Self {
            objects,
            prefix: Path::from(prefix),
            public_url: public_url.map(|url| url.trim_end_matches('/').to_string()),
        }
    }

    /// The store the URL `url` names, such as `file:///var/payloads` or `s3://bucket/prefix`,
    /// with credentials from the environment as `object_store` reads them.
    pub fn from_url(url: &str, public_url: Option<String>) -> anyhow::Result<Self> {
        let url = url::Url::parse(url).with_context(|| format!("parsing {url}"))?;
        let (objects, prefix) = object_store::parse_url_opts(&url, std::env::vars())
            .with_context(|| format!("opening the object store {url}"))?;
        Ok(Self {
            objects: Arc::from(objects),
            prefix,
            public_url: public_url.map(|url| url.trim_end_matches('/').to_string()),
        })
    }

    /// The key of the payload whose VID commitment is `commitment`. A commitment is a tag, `~`
    /// and base64url; the key has a `.` for the `~`, which object stores would escape, and URLs
    /// with it would then not match.
    pub fn key(&self, commitment: &str) -> Path {
        self.prefix.child(commitment.replace('~', "."))
    }

    /// Where clients fetch the payload whose VID commitment is `commitment`, if they can reach
    /// the object store.
    pub fn url(&self, commitment: &str) -> Option<String> {
        let base = self.public_url.as_ref()?;
        Some(format!("{base}/{}", self.key(commitment)))
    }

    /// The payload whose VID commitment is `commitment`, if it is uploaded.
    pub async fn get(&self, commitment: &str) -> anyhow::Result<Option<Bytes>> {
        match self.objects.get(&self.key(commitment)).await {
            Ok(object) => Ok(Some(object.bytes().await?)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// The bytes in `range` of the payload whose VID commitment is `commitment`, if it is
    /// uploaded, read without the rest of the payload.
    pub async fn get_range(
        &self,
        commitment: &str,
        range: std::ops::Range<u64>,
    ) -> anyhow::Result<Option<Bytes>> {
        if range.is_empty() {
            return Ok(Some(Bytes::new()));
        }
        match self.objects.get_range(&self.key(commitment), range).await {
            Ok(bytes) => Ok(Some(bytes)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// Uploads `payload`, whose VID commitment is `commitment`. Uploading it again does no harm.
    pub async fn put(&self, commitment: &str, payload: Bytes) -> anyhow::Result<()> {
        self.objects
            .put(&self.key(commitment), PutPayload::from_bytes(payload))
            .await?;
        Ok(())
    }
}

/// Payloads a writer has made durable locally and not yet uploaded.
#[derive(Clone, Debug)]
pub(crate) struct Staging {
    dir: PathBuf,
    uploads: mpsc::UnboundedSender<String>,
}

impl Staging {
    /// Writes `payload`, whose VID commitment is `commitment`, to the staging directory, durably,
    /// and queues it for upload.
    pub(crate) async fn stage(&self, commitment: &str, payload: &[u8]) -> anyhow::Result<()> {
        let path = self.dir.join(commitment);
        let partial = self.dir.join(format!(".{commitment}"));
        let mut file = fs::File::create(&partial).await?;
        file.write_all(payload).await?;
        file.sync_all().await?;
        fs::rename(&partial, &path).await?;
        fs::File::open(&self.dir).await?.sync_all().await?;
        let _ = self.uploads.send(commitment.to_string());
        Ok(())
    }
}

/// Uploads staged payloads, oldest first, for as long as the store it stages for exists.
pub struct Uploader {
    dir: PathBuf,
    payloads: PayloadStore,
    backend: Arc<dyn Backend>,
    queue: mpsc::UnboundedReceiver<String>,
}

impl Uploader {
    /// Uploads what a previous run left staged, then each payload as it is staged.
    pub async fn run(mut self) {
        match self.leftovers().await {
            Ok(leftovers) => {
                for commitment in leftovers {
                    self.upload(&commitment).await;
                }
            },
            Err(err) => tracing::error!("reading the staged payloads failed: {err:#}"),
        }
        while let Some(commitment) = self.queue.recv().await {
            self.upload(&commitment).await;
        }
    }

    async fn leftovers(&self) -> anyhow::Result<Vec<String>> {
        let mut staged = Vec::new();
        let mut entries = fs::read_dir(&self.dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let name = entry.file_name().to_string_lossy().into_owned();
            match name.starts_with('.') {
                // A write that did not finish was never reported durable.
                true => {
                    let _ = fs::remove_file(entry.path()).await;
                },
                false => staged.push(name),
            }
        }
        Ok(staged)
    }

    /// Uploads the payload staged under `commitment`, trying again until it succeeds, then records
    /// it as uploaded and deletes the staged copy. One no longer staged was uploaded already.
    async fn upload(&self, commitment: &str) {
        let path = self.dir.join(commitment);
        let mut delay = std::time::Duration::from_millis(100);
        loop {
            match self.try_upload(commitment, &path).await {
                Ok(()) => return,
                Err(err) => {
                    tracing::warn!(%commitment, "uploading a payload failed: {err:#}");
                    tokio::time::sleep(delay).await;
                    delay = (delay * 2).min(std::time::Duration::from_secs(30));
                },
            }
        }
    }

    async fn try_upload(&self, commitment: &str, path: &std::path::Path) -> anyhow::Result<()> {
        let payload = match fs::read(path).await {
            Ok(payload) => Bytes::from(payload),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err.into()),
        };
        self.payloads.put(commitment, payload.clone()).await?;
        let info = match record(self.backend.as_ref(), commitment).await? {
            Some(record) => record.info,
            None => PayloadInfo::of(&payload, None),
        };
        put_record(
            self.backend.as_ref(),
            commitment,
            &PayloadRecord {
                info,
                uploaded: true,
            },
        )
        .await?;
        fs::remove_file(path).await?;
        Ok(())
    }
}

/// A staging area in `dir` for payloads that go to `payloads`, with the uploader that empties it,
/// which marks uploads in `backend`.
pub(crate) async fn staging(
    dir: PathBuf,
    payloads: PayloadStore,
    backend: Arc<dyn Backend>,
) -> anyhow::Result<(Staging, Uploader)> {
    fs::create_dir_all(&dir)
        .await
        .with_context(|| format!("creating {}", dir.display()))?;
    let (uploads, queue) = mpsc::unbounded_channel();
    let staging = Staging {
        dir: dir.clone(),
        uploads,
    };
    let uploader = Uploader {
        dir,
        payloads,
        backend,
        queue,
    };
    Ok((staging, uploader))
}

#[cfg(test)]
mod tests {
    use espresso_types::{Leaf2, NodeState, ValidatedState};
    use hotshot_types::{
        simple_certificate::QuorumCertificate2, traits::block_contents::BlockHeader,
    };
    use object_store::memory::InMemory;
    use versions::{NEW_PROTOCOL_VERSION, Upgrade};

    use super::*;
    use crate::{ChainStore, Item, Part, Parts, mem::MemBackend};

    async fn genesis() -> Item {
        let instance = NodeState::mock().with_current_version(NEW_PROTOCOL_VERSION);
        let state = ValidatedState {
            chain_config: instance.chain_config.into(),
            ..Default::default()
        };
        let upgrade = Upgrade::trivial(NEW_PROTOCOL_VERSION);
        Item {
            leaf: Leaf2::genesis(&state, &instance, NEW_PROTOCOL_VERSION).await,
            cert1: QuorumCertificate2::genesis(&state, &instance, upgrade).await,
        }
    }

    fn with_payload(payload: &'static [u8]) -> Parts {
        Parts {
            payload: Some(Bytes::from_static(payload)),
            ..Default::default()
        }
    }

    fn objects() -> PayloadStore {
        PayloadStore::new(Arc::new(InMemory::new()), "payloads", None)
    }

    /// A store that uploads at once keeps the payload in the object store under its commitment,
    /// and only what describes it in the backend.
    #[tokio::test]
    async fn uploads_payloads_under_their_commitment() {
        let payloads = objects();
        let store = ChainStore::new(MemBackend::default()).with_payloads(payloads.clone());
        let item = genesis().await;
        store.put(&item, &with_payload(b"payload")).await.unwrap();

        let commitment = item.leaf.block_header().payload_commitment().to_string();
        assert_eq!(
            payloads.get(&commitment).await.unwrap().as_deref(),
            Some(&b"payload"[..])
        );
        assert_eq!(
            store.payload(0).await.unwrap().as_deref(),
            Some(&b"payload"[..])
        );
        let summary = store.summary(0).await.unwrap().unwrap().parts().unwrap();
        assert!(summary.payload_stored);
        assert_eq!(summary.payload_info.map(|info| info.size), Some(7));
        assert!(
            store
                .missing_parts(Part::Payload, 0, 10)
                .await
                .unwrap()
                .is_empty()
        );
        let record = store.record(0).await.unwrap().unwrap();
        assert_eq!(
            record.parts().unwrap().payload.as_deref(),
            Some(&b"payload"[..]),
            "a record carries the payload from the object store"
        );
    }

    /// A staged payload is missing until the uploader has uploaded it; one a previous run left
    /// staged is uploaded when the uploader runs again.
    #[tokio::test]
    async fn uploads_staged_payloads_and_leftovers() {
        let dir = std::env::temp_dir().join(format!("staging-{:016x}", rand_suffix()));
        let payloads = objects();
        let backend = MemBackend::default();
        let item = genesis().await;
        {
            let (store, _uploader) = ChainStore::new(MemBackend::default())
                .with_staged_payloads(payloads.clone(), dir.clone())
                .await
                .unwrap();
            store.put(&item, &with_payload(b"left")).await.unwrap();
            // The uploader never ran.
        }
        let (store, uploader) = ChainStore::new(backend)
            .with_staged_payloads(payloads.clone(), dir.clone())
            .await
            .unwrap();
        store.put(&item, &Parts::default()).await.unwrap();
        assert_eq!(
            store.missing_parts(Part::Payload, 0, 10).await.unwrap(),
            vec![0]
        );
        assert_eq!(store.payload(0).await.unwrap(), None);

        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(uploader.run());
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !store
                .missing_parts(Part::Payload, 0, 10)
                .await
                .unwrap()
                .is_empty()
            {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the leftover is uploaded");
        assert_eq!(
            store.payload(0).await.unwrap().as_deref(),
            Some(&b"left"[..])
        );
        let staged = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(staged, 0, "the staged copy is deleted");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A payload stored by its commitment before any block has it gives a block that has it its
    /// info when the block's item is written; staged, the block counts as missing it until it is
    /// uploaded, and is marked once repair looks again after the upload.
    #[tokio::test]
    async fn a_payload_stored_before_its_block_reaches_the_block() {
        let dir = std::env::temp_dir().join(format!("staging-{:016x}", rand_suffix()));
        let (store, uploader) = ChainStore::new(MemBackend::default())
            .with_staged_payloads(objects(), dir.clone())
            .await
            .unwrap();
        let item = genesis().await;
        let header = item.leaf.block_header();
        let commitment = header.payload_commitment().to_string();
        let info = store
            .put_payload(
                &commitment,
                Bytes::from_static(b"early"),
                Some(header.metadata()),
            )
            .await
            .unwrap();

        store.put(&item, &Parts::default()).await.unwrap();
        let parts = store.summary(0).await.unwrap().unwrap().parts().unwrap();
        assert_eq!(parts.payload_info, Some(info));
        assert!(!parts.payload_stored, "not uploaded yet");
        assert_eq!(
            store.missing_parts(Part::Payload, 0, 10).await.unwrap(),
            vec![0]
        );

        let mut tasks = tokio::task::JoinSet::new();
        tasks.spawn(uploader.run());
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !store
                .missing_parts(Part::Payload, 0, 10)
                .await
                .unwrap()
                .is_empty()
            {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("marked once uploaded");
        let parts = store.summary(0).await.unwrap().unwrap().parts().unwrap();
        assert!(parts.payload_stored);
        assert_eq!(
            store.payload(0).await.unwrap().as_deref(),
            Some(&b"early"[..])
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A block stored before its payload gets the payload's info once the payload is stored.
    #[tokio::test]
    async fn a_block_stored_before_its_payload_gets_its_info() {
        let store = ChainStore::new(MemBackend::default()).with_payloads(objects());
        let item = genesis().await;
        store.put(&item, &Parts::default()).await.unwrap();
        let parts = store.summary(0).await.unwrap().unwrap().parts().unwrap();
        assert_eq!(parts.payload_info, None);

        let header = item.leaf.block_header();
        let commitment = header.payload_commitment().to_string();
        let info = store
            .put_payload(
                &commitment,
                Bytes::from_static(b"late"),
                Some(header.metadata()),
            )
            .await
            .unwrap();
        let parts = store.summary(0).await.unwrap().unwrap().parts().unwrap();
        assert_eq!(parts.payload_info, Some(info));
        assert!(parts.payload_stored, "uploaded at once");
    }

    /// The object's key is the payload commitment, which only the block names, so a payload
    /// cannot be stored by height before its block.
    #[tokio::test]
    async fn refuses_a_payload_before_its_block() {
        let store = ChainStore::new(MemBackend::default()).with_payloads(objects());
        assert!(store.put_parts(0, &with_payload(b"early")).await.is_err());
    }

    fn rand_suffix() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
            ^ std::process::id() as u64
    }
}
