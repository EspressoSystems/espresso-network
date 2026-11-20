//! Records that make the API services of the validators findable.
//!
//! At the start of each epoch, every validator signs a record for that epoch with the URL of its
//! API service, and the Ed25519 key that service authenticates requests with if it has one, and
//! sends it to every consensus node. A node keeps, per epoch, the newest record of each validator
//! staked in that epoch, for the latest [`RETAINED_EPOCHS`] epochs, and serves them to its API
//! service, which lists them exactly as received. A client checks each signature against the
//! stake table of the record's epoch, so it need not trust the service that lists them.

use std::collections::BTreeMap;

pub(crate) use consensus_api::discovery::{ServiceRecord, SignedServiceRecord};
use hotshot_types::{data::EpochNumber, traits::node_implementation::NodeType};

/// How many of the latest epochs records are kept for, the window in which nodes keep stake
/// tables.
pub const RETAINED_EPOCHS: u64 = 20;

/// Per epoch, the newest valid record of each validator.
#[derive(Debug)]
pub struct ServiceBook<T: NodeType> {
    epochs: BTreeMap<EpochNumber, BTreeMap<T::SignatureKey, SignedServiceRecord<T>>>,
}

impl<T: NodeType> Default for ServiceBook<T> {
    fn default() -> Self {
        Self {
            epochs: BTreeMap::new(),
        }
    }
}

impl<T: NodeType> ServiceBook<T> {
    pub fn new(records: impl IntoIterator<Item = SignedServiceRecord<T>>) -> Self {
        let mut book = Self::default();
        for record in records {
            book.insert(record);
        }
        book
    }

    /// Keeps `record` if it is valid and newer than the one held for its signer in its epoch, and
    /// says whether it did. Whether the signer is staked in the epoch is the caller's to check.
    pub fn insert(&mut self, record: SignedServiceRecord<T>) -> bool {
        if !record.is_valid() {
            return false;
        }
        let epoch = self.epochs.entry(record.record.epoch).or_default();
        if epoch
            .get(&record.signer)
            .is_some_and(|held| held.record.sequence >= record.record.sequence)
        {
            return false;
        }
        epoch.insert(record.signer.clone(), record);
        true
    }

    /// The records of `epoch`, by signer.
    pub fn records(&self, epoch: EpochNumber) -> impl Iterator<Item = &SignedServiceRecord<T>> {
        self.epochs.get(&epoch).into_iter().flat_map(|e| e.values())
    }

    /// Forgets the records of the epochs before `epoch`.
    pub fn drop_before(&mut self, epoch: EpochNumber) {
        self.epochs = self.epochs.split_off(&epoch);
    }
}

#[cfg(test)]
mod tests {
    use consensus_api::discovery::MAX_API_URL_LEN;
    use hotshot_example_types::node_types::TestTypes;
    use hotshot_types::{signature_key::BLSPubKey, traits::signature_key::SignatureKey};

    use super::*;

    fn record(url: &str, sequence: u64) -> ServiceRecord {
        record_in(1, url, sequence)
    }

    fn record_in(epoch: u64, url: &str, sequence: u64) -> ServiceRecord {
        ServiceRecord {
            epoch: EpochNumber::new(epoch),
            api_url: url.into(),
            api_key: Some([7; 32]),
            sequence,
        }
    }

    fn signed(index: u64, record: ServiceRecord) -> SignedServiceRecord<TestTypes> {
        let (public, private) = BLSPubKey::generated_from_seed_indexed([0; 32], index);
        record.sign::<TestTypes>(public, &private).unwrap()
    }

    #[test]
    fn keeps_the_newest_valid_record_of_each_signer() {
        let mut book = ServiceBook::<TestTypes>::default();
        assert!(book.insert(signed(0, record("http://a:1", 2))));
        assert!(book.insert(signed(1, record("http://b:1", 1))));
        assert!(
            !book.insert(signed(0, record("http://a:2", 1))),
            "an older record is ignored"
        );
        assert!(
            !book.insert(signed(0, record("http://a:2", 2))),
            "a record with the same sequence number is ignored"
        );
        assert!(book.insert(signed(0, record("http://a:3", 3))));
        let urls: Vec<_> = book
            .records(EpochNumber::new(1))
            .map(|r| r.record.api_url.clone())
            .collect();
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"http://a:3".to_string()));
        assert!(urls.contains(&"http://b:1".to_string()));
    }

    #[test]
    fn rejects_records_not_signed_by_their_signer() {
        let mut book = ServiceBook::<TestTypes>::default();
        let mut forged = signed(0, record("http://a:1", 1));
        forged.signer = signed(1, record("http://a:1", 1)).signer;
        assert!(!book.insert(forged));

        let mut altered = signed(0, record("http://a:1", 1));
        altered.record.api_url = "http://evil:1".into();
        assert!(!book.insert(altered));

        let mut rekeyed = signed(0, record("http://a:1", 1));
        rekeyed.record.api_key = None;
        assert!(!book.insert(rekeyed));

        let mut moved = signed(0, record_in(1, "http://a:1", 1));
        moved.record.epoch = EpochNumber::new(2);
        assert!(!book.insert(moved), "the epoch is signed too");
        assert_eq!(book.records(EpochNumber::new(1)).count(), 0);
        assert_eq!(book.records(EpochNumber::new(2)).count(), 0);
    }

    /// Records of different epochs are kept apart, and dropping old epochs keeps the rest.
    #[test]
    fn keeps_records_per_epoch() {
        let mut book = ServiceBook::<TestTypes>::default();
        assert!(book.insert(signed(0, record_in(1, "http://a:1", 5))));
        assert!(
            book.insert(signed(0, record_in(2, "http://a:2", 1))),
            "a later epoch takes a record whatever the sequence number"
        );
        assert!(book.insert(signed(1, record_in(2, "http://b:2", 1))));
        assert_eq!(book.records(EpochNumber::new(1)).count(), 1);
        assert_eq!(book.records(EpochNumber::new(2)).count(), 2);

        book.drop_before(EpochNumber::new(2));
        assert_eq!(book.records(EpochNumber::new(1)).count(), 0);
        assert_eq!(book.records(EpochNumber::new(2)).count(), 2);
    }

    #[test]
    fn rejects_records_without_a_usable_url() {
        let mut book = ServiceBook::<TestTypes>::default();
        assert!(!book.insert(signed(0, record("", 1))));
        assert!(!book.insert(signed(0, record(&"x".repeat(MAX_API_URL_LEN + 1), 1))));
        assert!(book.insert(signed(0, record(&"x".repeat(MAX_API_URL_LEN), 1))));
    }
}
