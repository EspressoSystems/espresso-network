//! This file contains the `PersistentStore` struct, which is a wrapper around a `RecordStore`
//! that occasionally saves the DHT to a persistent storage.

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::Context;
use async_trait::async_trait;
use delegate::delegate;
use libp2p::kad::store::{RecordStore, Result};
use serde::{Deserialize, Serialize};
use tokio::{sync::Semaphore, time::timeout};
use tracing::{debug, warn};

/// A trait that we use to save and load the DHT to a file on disk
/// or other storage medium
#[async_trait]
pub trait DhtPersistentStorage: Send + Sync + 'static + Clone {
    /// Save the DHT (as a list of serializable records) to the persistent storage
    ///
    /// # Errors
    /// - If we fail to save the DHT to the persistent storage provider
    async fn save(&self, _records: Vec<SerializableRecord>) -> anyhow::Result<()>;

    /// Load the DHT (as a list of serializable records) from the persistent storage
    ///
    /// # Errors
    /// - If we fail to load the DHT from the persistent storage provider
    async fn load(&self) -> anyhow::Result<Vec<SerializableRecord>>;
}

/// A no-op `PersistentStorage` that does not persist the DHT
#[derive(Clone)]
pub struct DhtNoPersistence;

#[async_trait]
impl DhtPersistentStorage for DhtNoPersistence {
    async fn save(&self, _records: Vec<SerializableRecord>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn load(&self) -> anyhow::Result<Vec<SerializableRecord>> {
        Ok(vec![])
    }
}

/// A `PersistentStorage` that persists the DHT to a file on disk. Used mostly for
/// testing.
#[derive(Clone)]
pub struct DhtFilePersistence {
    /// The path to the file on disk
    path: String,
}

impl DhtFilePersistence {
    /// Create a new `DhtFilePersistence` with the given path
    #[must_use]
    pub fn new(path: String) -> Self {
        Self { path }
    }
}

#[async_trait]
impl DhtPersistentStorage for DhtFilePersistence {
    /// Save the DHT to the file on disk
    ///
    /// # Errors
    /// - If we fail to serialize the records
    /// - If we fail to write the serialized records to the file
    async fn save(&self, records: Vec<SerializableRecord>) -> anyhow::Result<()> {
        // Bincode-serialize the records
        let to_save =
            bincode::serialize(&records).with_context(|| "Failed to serialize records")?;

        // Write the serialized records to the file
        std::fs::write(&self.path, to_save).with_context(|| "Failed to write records to file")?;

        Ok(())
    }

    /// Load the DHT from the file on disk
    ///
    /// # Errors
    /// - If we fail to read the file
    /// - If we fail to deserialize the records
    async fn load(&self) -> anyhow::Result<Vec<SerializableRecord>> {
        // Read the contents of the file
        let contents =
            std::fs::read(&self.path).with_context(|| "Failed to read records from file")?;

        // Deserialize the contents
        let records: Vec<SerializableRecord> =
            bincode::deserialize(&contents).with_context(|| "Failed to deserialize records")?;

        Ok(records)
    }
}

/// A `RecordStore` wrapper that occasionally saves the DHT to a persistent storage.
pub struct PersistentStore<R: RecordStore, D: DhtPersistentStorage> {
    /// The underlying record store
    underlying_record_store: R,

    /// The persistent storage
    persistent_storage: D,

    /// The semaphore for limiting the number of concurrent operations (to one)
    semaphore: Arc<Semaphore>,

    /// The maximum number of records that can be added to the store before the store is saved to the persistent storage
    max_record_delta: u64,

    /// The running delta between the records in the persistent storage and the records in the underlying store
    record_delta: Arc<AtomicU64>,
}

/// A serializable version of a Libp2p `Record`
#[derive(Serialize, Deserialize)]
pub struct SerializableRecord {
    /// The key of the record
    pub key: libp2p::kad::RecordKey,
    /// The value of the record
    pub value: Vec<u8>,
    /// The (original) publisher of the record.
    pub publisher: Option<libp2p::PeerId>,
    /// The record expiration time in seconds since the Unix epoch
    ///
    /// This is an approximation of the expiration time because we can't
    /// serialize an `Instant` directly.
    pub expires_unix_secs: Option<u64>,
}

/// Approximate an `Instant` to the number of seconds since the Unix epoch
fn instant_to_unix_seconds(instant: Instant) -> anyhow::Result<u64> {
    // Get the current instant and system time
    let now_instant = Instant::now();
    let now_system = SystemTime::now();

    // Get the duration of time between the instant and now
    if instant > now_instant {
        Ok(now_system
            .checked_add(instant - now_instant)
            .with_context(|| "Overflow when approximating expiration time")?
            .duration_since(UNIX_EPOCH)
            .with_context(|| "Failed to get duration since Unix epoch")?
            .as_secs())
    } else {
        Ok(now_system
            .checked_sub(now_instant - instant)
            .with_context(|| "Underflow when approximating expiration time")?
            .duration_since(UNIX_EPOCH)
            .with_context(|| "Failed to get duration since Unix epoch")?
            .as_secs())
    }
}

/// Convert a unix-second timestamp to an `Instant`
fn unix_seconds_to_instant(unix_secs: u64) -> anyhow::Result<Instant> {
    // Get the current instant and unix time
    let now_instant = Instant::now();
    let unix_secs_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .with_context(|| "Failed to get duration since Unix epoch")?
        .as_secs();

    if unix_secs > unix_secs_now {
        // If the instant is in the future, add the duration to the current time
        now_instant
            .checked_add(Duration::from_secs(unix_secs - unix_secs_now))
            .with_context(|| "Overflow when calculating future instant")
    } else {
        // If the instant is in the past, subtract the duration from the current time
        now_instant
            .checked_sub(Duration::from_secs(unix_secs_now - unix_secs))
            .with_context(|| "Underflow when calculating past instant")
    }
}

/// Allow conversion from a `libp2p::kad::Record` to a `SerializableRecord`
impl TryFrom<libp2p::kad::Record> for SerializableRecord {
    type Error = anyhow::Error;

    fn try_from(record: libp2p::kad::Record) -> anyhow::Result<Self> {
        Ok(SerializableRecord {
            key: record.key,
            value: record.value,
            publisher: record.publisher,
            expires_unix_secs: record.expires.map(instant_to_unix_seconds).transpose()?,
        })
    }
}

/// Allow conversion from a `SerializableRecord` to a `libp2p::kad::Record`
impl TryFrom<SerializableRecord> for libp2p::kad::Record {
    type Error = anyhow::Error;

    fn try_from(record: SerializableRecord) -> anyhow::Result<Self> {
        Ok(libp2p::kad::Record {
            key: record.key,
            value: record.value,
            publisher: record.publisher,
            expires: record
                .expires_unix_secs
                .map(unix_seconds_to_instant)
                .transpose()?,
        })
    }
}

impl<R: RecordStore, D: DhtPersistentStorage> PersistentStore<R, D> {
    /// Create a new `PersistentStore` with the given underlying store and path.
    /// On creation, the DHT is restored from the persistent storage if possible.
    ///
    /// `max_record_delta` is the maximum number of records that can be added to the store before
    /// the store is saved to the persistent storage.
    pub async fn new(
        underlying_record_store: R,
        persistent_storage: D,
        max_record_delta: u64,
    ) -> Self {
        // Create the new store
        let mut store = PersistentStore {
            underlying_record_store,
            persistent_storage,
            max_record_delta,
            record_delta: Arc::new(AtomicU64::new(0)),
            semaphore: Arc::new(Semaphore::new(1)),
        };

        // Try to restore the DHT from the persistent store. If it fails, warn and start with an empty store
        if let Err(err) = store.restore_from_persistent_storage().await {
            warn!(
                "Failed to restore DHT from persistent storage: {:?}. Starting with empty store",
                err
            );
        }

        // Return the new store
        store
    }

    /// Try saving the DHT to persistent storage if a task is not already in progress.
    ///
    /// Returns `true` if the DHT was saved, `false` otherwise.
    fn try_save_to_persistent_storage(&mut self) -> bool {
        // Try to acquire the semaphore, warning if another save operation is already in progress
        let Ok(permit) = Arc::clone(&self.semaphore).try_acquire_owned() else {
            warn!("Skipping DHT save to persistent storage - another save operation is already in progress");
            return false;
        };

        // Get all records and convert them to their serializable counterparts
        let serializable_records: Vec<_> = self
            .underlying_record_store
            .records()
            .filter_map(|record| {
                SerializableRecord::try_from(record.into_owned())
                    .map_err(|err| {
                        warn!("Failed to convert record to serializable record: {:?}", err);
                    })
                    .ok()
            })
            .collect();

        // Spawn a task to save the DHT to the persistent storage
        let persistent_storage = self.persistent_storage.clone();
        let record_delta = Arc::clone(&self.record_delta);
        tokio::spawn(async move {
            debug!("Saving DHT to persistent storage");

            // Save the DHT to the persistent storage
            match timeout(
                Duration::from_secs(10),
                persistent_storage.save(serializable_records),
            )
            .await
            .map_err(|_| anyhow::anyhow!("save operation timed out"))
            {
                Ok(Ok(())) => {},
                Ok(Err(error)) | Err(error) => {
                    warn!("Failed to save DHT to persistent storage: {error}");
                },
            };

            // Reset the record delta
            record_delta.store(0, Ordering::Release);

            drop(permit);

            debug!("Saved DHT to persistent storage");
        });

        true
    }

    /// Attempt to restore the DHT to the underlying store from the persistent storage
    ///
    /// # Errors
    /// - If we fail to load from the persistent storage
    pub async fn restore_from_persistent_storage(&mut self) -> anyhow::Result<()> {
        debug!("Restoring DHT from persistent storage");

        // Read the contents of the persistent store
        let serializable_records = self
            .persistent_storage
            .load()
            .await
            .with_context(|| "Failed to read DHT from persistent storage")?;

        // Put all records into the new store
        for serializable_record in serializable_records {
            // Convert the serializable record back to a `libp2p::kad::Record`
            match libp2p::kad::Record::try_from(serializable_record) {
                Ok(record) => {
                    // Put the record into the new store
                    if let Err(err) = self.underlying_record_store.put(record) {
                        warn!(
                            "Failed to restore record from persistent storage: {:?}",
                            err
                        );
                    }
                },
                Err(err) => {
                    warn!("Failed to parse record from persistent storage: {:?}", err);
                },
            };
        }

        debug!("Restored DHT from persistent storage");

        Ok(())
    }
}

/// Implement the `RecordStore` trait for `PersistentStore`
impl<R: RecordStore, D: DhtPersistentStorage> RecordStore for PersistentStore<R, D> {
    type ProvidedIter<'a>
        = R::ProvidedIter<'a>
    where
        R: 'a,
        D: 'a;
    type RecordsIter<'a>
        = R::RecordsIter<'a>
    where
        R: 'a,
        D: 'a;
    // Delegate all `RecordStore` methods except `put` to the inner store
    delegate! {
        to self.underlying_record_store {
            fn add_provider(&mut self, record: libp2p::kad::ProviderRecord) -> libp2p::kad::store::Result<()>;
            fn get(&self, k: &libp2p::kad::RecordKey) -> Option<std::borrow::Cow<'_, libp2p::kad::Record>>;
            fn provided(&self) -> Self::ProvidedIter<'_>;
            fn providers(&self, key: &libp2p::kad::RecordKey) -> Vec<libp2p::kad::ProviderRecord>;
            fn records(&self) -> Self::RecordsIter<'_>;
            fn remove_provider(&mut self, k: &libp2p::kad::RecordKey, p: &libp2p::PeerId);
        }
    }

    /// Overwrite the `put` method to potentially sync the DHT to the persistent store
    fn put(&mut self, record: libp2p::kad::Record) -> Result<()> {
        // Try to write to the underlying store
        let result = self.underlying_record_store.put(record);

        // If the record was successfully written,
        if result.is_ok() {
            // Update the record delta
            self.record_delta.fetch_add(1, Ordering::Relaxed);

            // Check if it's above the maximum record delta
            if self.record_delta.load(Ordering::Relaxed) > self.max_record_delta {
                // Try to save the DHT to persistent storage
                self.try_save_to_persistent_storage();
            }
        }

        result
    }

    /// Overwrite the `remove` method to potentially sync the DHT to the persistent store
    fn remove(&mut self, k: &libp2p::kad::RecordKey) {
        // Remove the record from the underlying store
        self.underlying_record_store.remove(k);

        // Update the record delta
        self.record_delta.fetch_add(1, Ordering::Relaxed);

        // Check if it's above the maximum record delta
        if self.record_delta.load(Ordering::Relaxed) > self.max_record_delta {
            // Try to save the DHT to persistent storage
            self.try_save_to_persistent_storage();
        }
    }
}

#[cfg(test)]
mod tests {
    use libp2p::{
        kad::{store::MemoryStore, RecordKey},
        PeerId,
    };
    use tracing_subscriber::EnvFilter;

    use super::*;

    #[tokio::test]
    async fn test_save_and_restore() {
        // Try initializing tracing
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        // Create a test store
        let mut store = PersistentStore::new(
            MemoryStore::new(PeerId::random()),
            DhtFilePersistence::new("/tmp/test1.dht".to_string()),
            10,
        )
        .await;

        // The key is a random 16-byte array
        let key = RecordKey::new(&rand::random::<[u8; 16]>().to_vec());

        // The value is a random 16-byte array
        let random_value = rand::random::<[u8; 16]>();

        // Put a record into the store
        store
            .put(libp2p::kad::Record::new(key.clone(), random_value.to_vec()))
            .expect("Failed to put record into store");

        // Try to save the store to a persistent storage
        assert!(store.try_save_to_persistent_storage());

        // Wait a bit for the save to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create a new store from the persistent storage
        let new_store = PersistentStore::new(
            MemoryStore::new(PeerId::random()),
            DhtFilePersistence::new("/tmp/test1.dht".to_string()),
            10,
        )
        .await;

        // Check that the new store has the record
        let restored_record = new_store
            .get(&key)
            .expect("Failed to get record from store");

        // Check that the restored record has the same value as the original record
        assert_eq!(restored_record.value, random_value.to_vec());
    }

    #[tokio::test]
    async fn test_record_delta() {
        // Try initializing tracing
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();

        // Create a test store
        let mut store = PersistentStore::new(
            MemoryStore::new(PeerId::random()),
            DhtFilePersistence::new("/tmp/test2.dht".to_string()),
            10,
        )
        .await;

        let mut keys = Vec::new();
        let mut values = Vec::new();

        // Put 10 records into the store
        for _ in 0..10 {
            // Create a random key and value
            let key = RecordKey::new(&rand::random::<[u8; 16]>().to_vec());
            let value = rand::random::<[u8; 16]>();

            keys.push(key.clone());
            values.push(value);

            store
                .put(libp2p::kad::Record::new(key, value.to_vec()))
                .expect("Failed to put record into store");
        }

        // Create a new store from the allegedly unpersisted DHT
        let new_store = PersistentStore::new(
            MemoryStore::new(PeerId::random()),
            DhtFilePersistence::new("/tmp/test2.dht".to_string()),
            10,
        )
        .await;

        // Check that the new store has none of the records
        for key in &keys {
            assert!(new_store.get(key).is_none());
        }

        // Store one more record into the new store
        store
            .put(libp2p::kad::Record::new(
                keys[0].clone(),
                values[0].to_vec(),
            ))
            .expect("Failed to put record into store");

        // Wait a bit for the save to complete
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create a new store from the allegedly saved DHT
        let new_store = PersistentStore::new(
            MemoryStore::new(PeerId::random()),
            DhtFilePersistence::new("/tmp/test2.dht".to_string()),
            10,
        )
        .await;

        // Check that the new store has all of the records
        for (i, key) in keys.iter().enumerate() {
            let restored_record = new_store.get(key).expect("Failed to get record from store");
            assert_eq!(restored_record.value, values[i]);
        }

        // Check that the record delta is 0
        assert_eq!(store.record_delta.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_approximate_instant() {
        // Create an expiry time in the future
        let expiry_future = Instant::now() + Duration::from_secs(10);

        // Approximate the expiry time
        let approximate_expiry =
            unix_seconds_to_instant(instant_to_unix_seconds(expiry_future).unwrap())
                .unwrap()
                .duration_since(Instant::now());

        // Make sure it's close to 10 seconds in the future
        assert!(approximate_expiry >= Duration::from_secs(9));
        assert!(approximate_expiry <= Duration::from_secs(11));

        // Create an expiry time in the past
        let expiry_past = Instant::now().checked_sub(Duration::from_secs(10)).unwrap();

        // Approximate the expiry time
        let approximate_expiry =
            unix_seconds_to_instant(instant_to_unix_seconds(expiry_past).unwrap()).unwrap();
        let time_difference = approximate_expiry.elapsed();

        // Make sure it's close to 10 seconds in the past
        assert!(time_difference >= Duration::from_secs(9));
        assert!(time_difference <= Duration::from_secs(11));
    }
}
