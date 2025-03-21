//! This file contains the `ValidatedStore` struct, which is a wrapper around a `RecordStore` that
//! validates records before storing them.
//!
//! The `ValidatedStore` struct is used to ensure that only valid records are stored in the DHT.

use std::marker::PhantomData;

use delegate::delegate;
use hotshot_types::traits::signature_key::SignatureKey;
use libp2p::kad::store::{Error, RecordStore, Result};
use tracing::warn;

use crate::network::behaviours::dht::record::{RecordKey, RecordValue};

/// A `RecordStore` wrapper that validates records before storing them.
pub struct ValidatedStore<R: RecordStore, K: SignatureKey> {
    /// The underlying store
    store: R,

    /// Phantom type for the key
    phantom: std::marker::PhantomData<K>,
}

impl<R: RecordStore, K: SignatureKey> ValidatedStore<R, K> {
    /// Create a new `ValidatedStore` with the given underlying store
    pub fn new(store: R) -> Self {
        ValidatedStore {
            store,
            phantom: PhantomData,
        }
    }
}

/// Implement the `RecordStore` trait for `ValidatedStore`
impl<R: RecordStore, K: SignatureKey> RecordStore for ValidatedStore<R, K>
where
    K: 'static,
{
    type ProvidedIter<'a>
        = R::ProvidedIter<'a>
    where
        R: 'a,
        K: 'a;
    type RecordsIter<'a>
        = R::RecordsIter<'a>
    where
        R: 'a,
        K: 'a;

    // Delegate all `RecordStore` methods except `put` to the inner store
    delegate! {
        to self.store{
            fn add_provider(&mut self, record: libp2p::kad::ProviderRecord) -> libp2p::kad::store::Result<()>;
            fn get(&self, k: &libp2p::kad::RecordKey) -> Option<std::borrow::Cow<'_, libp2p::kad::Record>>;
            fn provided(&self) -> Self::ProvidedIter<'_>;
            fn providers(&self, key: &libp2p::kad::RecordKey) -> Vec<libp2p::kad::ProviderRecord>;
            fn records(&self) -> Self::RecordsIter<'_>;
            fn remove(&mut self, k: &libp2p::kad::RecordKey);
            fn remove_provider(&mut self, k: &libp2p::kad::RecordKey, p: &libp2p::PeerId);
        }
    }

    /// Overwrite the `put` method to validate the record before storing it
    fn put(&mut self, record: libp2p::kad::Record) -> Result<()> {
        // Convert the record to the correct type
        if let Ok(record_value) = RecordValue::<K>::try_from(record.clone()) {
            // Convert the key to the correct type
            let Ok(record_key) = RecordKey::try_from_bytes(&record.key.to_vec()) else {
                warn!("Failed to convert record key");
                return Err(Error::MaxRecords);
            };

            // If the record is signed by the correct key,
            if record_value.validate(&record_key) {
                // Store the record
                if let Err(err) = self.store.put(record.clone()) {
                    warn!("Failed to store record: {:?}", err);
                    return Err(Error::MaxRecords);
                }
            } else {
                warn!("Failed to validate record");
                return Err(Error::MaxRecords);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use hotshot_types::signature_key::BLSPubKey;
    use libp2p::{
        kad::{store::MemoryStore, Record},
        PeerId,
    };

    use super::*;
    use crate::network::behaviours::dht::record::Namespace;

    /// Test that a valid record is stored
    #[test]
    fn test_valid_stored() {
        // Generate a staking keypair
        let (public_key, private_key) = BLSPubKey::generated_from_seed_indexed([1; 32], 1337);

        // Create a value. The key is the public key
        let value = vec![5, 6, 7, 8];

        // Create a record key (as we need to sign both the key and the value)
        let record_key = RecordKey::new(Namespace::Lookup, public_key.to_bytes());

        // Sign the record and value with the private key
        let record_value: RecordValue<BLSPubKey> =
            RecordValue::new_signed(&record_key, value.clone(), &private_key).unwrap();

        // Initialize the store
        let mut store: ValidatedStore<MemoryStore, BLSPubKey> =
            ValidatedStore::new(MemoryStore::new(PeerId::random()));

        // Serialize the record value
        let record_value_bytes =
            bincode::serialize(&record_value).expect("Failed to serialize record value");

        // Create and store the record
        let record = Record::new(record_key.to_bytes(), record_value_bytes);
        store.put(record).expect("Failed to store record");

        // Check that the record is stored
        let libp2p_record_key = libp2p::kad::RecordKey::new(&record_key.to_bytes());
        let stored_record = store.get(&libp2p_record_key).expect("Failed to get record");
        let stored_record_value: RecordValue<BLSPubKey> =
            bincode::deserialize(&stored_record.value).expect("Failed to deserialize record value");

        // Make sure the stored record is the same as the original record
        assert_eq!(
            record_value, stored_record_value,
            "Stored record is not the same as original"
        );
    }

    /// Test that an invalid record is not stored
    #[test]
    fn test_invalid_not_stored() {
        // Generate a staking keypair
        let (public_key, _) = BLSPubKey::generated_from_seed_indexed([1; 32], 1337);

        // Create a record key (as we need to sign both the key and the value)
        let record_key = RecordKey::new(Namespace::Lookup, public_key.to_bytes());

        // Create a new (unsigned, invalid) record value
        let record_value: RecordValue<BLSPubKey> = RecordValue::new(vec![2, 3]);

        // Initialize the store
        let mut store: ValidatedStore<MemoryStore, BLSPubKey> =
            ValidatedStore::new(MemoryStore::new(PeerId::random()));

        // Serialize the record value
        let record_value_bytes =
            bincode::serialize(&record_value).expect("Failed to serialize record value");

        // Make sure we are unable to store the record
        let record = Record::new(record_key.to_bytes(), record_value_bytes);
        assert!(store.put(record).is_err(), "Should not have stored record");

        // Check that the record is not stored
        let libp2p_record_key = libp2p::kad::RecordKey::new(&record_key.to_bytes());
        assert!(
            store.get(&libp2p_record_key).is_none(),
            "Should not have stored record"
        );
    }
}
