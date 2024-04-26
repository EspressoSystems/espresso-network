// use serde::{Deserialize, Serialize};

use super::NsIndex;
use crate::block2::{
    payload_bytes::{
        num_txs_as_bytes, num_txs_from_bytes, tx_offset_as_bytes, tx_offset_from_bytes,
        NUM_TXS_BYTE_LEN, TX_OFFSET_BYTE_LEN,
    },
    Payload,
};
use crate::{NamespaceId, Transaction};
use serde::{Deserialize, Serialize};
use std::ops::Range;

use tx_iter::{TxIndex, TxIter};

// TODO move all the modules from inside ns_table back up to block2?
// TODO move this to ns_table.rs so we can construct a `Payload` there and keep `NsTable` fields private?
#[derive(Default)]
pub struct NamespacePayloadBuilder {
    tx_table_entries: Vec<u8>,
    tx_bodies: Vec<u8>,
}

impl NamespacePayloadBuilder {
    /// Add a transaction's payload to this namespace
    pub fn append_tx(&mut self, tx: Transaction) {
        self.tx_bodies.extend(tx.into_payload());
        self.tx_table_entries
            .extend(tx_offset_as_bytes(self.tx_bodies.len()));
    }

    /// Serialize to bytes and consume self.
    pub fn into_bytes(self) -> Vec<u8> {
        let mut result = Vec::with_capacity(
            NUM_TXS_BYTE_LEN + self.tx_table_entries.len() + self.tx_bodies.len(),
        );
        let num_txs = self.tx_table_entries.len() / TX_OFFSET_BYTE_LEN;
        result.extend(num_txs_as_bytes(num_txs));
        result.extend(self.tx_table_entries);
        result.extend(self.tx_bodies);
        result
    }
}

/// TODO explain: [`NsPayloadOwned`] to [`NsPayload`] as [`Vec<T>`] is to `[T]`.
#[repr(transparent)]
// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
// #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[derive(Debug)]
pub struct NsPayload([u8]);

#[repr(transparent)]
// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct NsPayloadOwned(Vec<u8>);

impl NsPayload {
    /// Number of bytes used to encode the number of txs in the tx table.
    ///
    /// Returns the minimum of [`NUM_TXS_BYTE_LEN`] and the byte length of the
    /// entire namespace payload.
    ///
    /// In all nontrivial cases this quantity is [`NUM_TXS_BYTE_LEN`]. Anything
    /// else is a degenerate case.
    pub fn num_txs_byte_len(&self) -> usize {
        NUM_TXS_BYTE_LEN.min(self.0.len())
    }

    /// Number of entries in this namespace's tx table.
    ///
    /// Returns the minimum of:
    /// - The declared number of txs from the tx table.
    /// - The maximum number of tx table entries that could fit into the
    ///   namespace payload.
    pub fn num_txs(&self) -> usize {
        let num_txs_byte_len = self.num_txs_byte_len();
        std::cmp::min(
            // Read the declared number of txs from the tx table
            num_txs_from_bytes(&self.0[..num_txs_byte_len]),
            // Max number of entries that could fit in the namespace payload
            self.0.len().saturating_sub(num_txs_byte_len) / TX_OFFSET_BYTE_LEN,
        )
    }

    /// Byte length of this namespace's tx table.
    ///
    /// Guaranteed to be no larger than this namespace's payload byte length.
    pub fn tx_table_byte_len(&self) -> usize {
        self.num_txs()
            .saturating_mul(TX_OFFSET_BYTE_LEN)
            .saturating_add(NUM_TXS_BYTE_LEN)
            .min(self.0.len())
    }

    /// Read subslice range for the `index`th tx from the tx
    /// table.
    ///
    /// Returned range guaranteed to satisfy `start <= end <= namespace_byte_len`.
    ///
    /// Panics if `index >= self.num_txs()`.
    pub fn tx_payload_range(&self, index: &TxIndex) -> Range<usize> {
        let tx_table_byte_len = self.tx_table_byte_len();
        let end = self
            .read_tx_offset(index)
            .saturating_add(tx_table_byte_len)
            .min(self.0.len());
        let start = self
            .read_tx_offset_prev(index)
            .unwrap_or(0)
            .saturating_add(tx_table_byte_len)
            .min(end);
        tracing::info!("tx_payload_range {:?}", start..end);
        start..end
    }

    /// Access the bytes of this [`NsPayload`].
    pub fn as_byte_slice(&self) -> &[u8] {
        &self.0
    }

    /// TODO store `ns_id` in `NsPayload` struct?
    pub fn export_all_txs(&self, ns_id: &NamespaceId) -> Vec<Transaction> {
        TxIter::new(self)
            .map(|i| Transaction::new(*ns_id, self.0[self.tx_payload_range(&i)].to_vec()))
            .collect()
    }
    pub fn export_tx(&self, ns_id: &NamespaceId, index: &TxIndex) -> Transaction {
        Transaction::new(*ns_id, self.0[self.tx_payload_range(index)].to_vec())
    }
}

impl Payload {
    /// TODO panics if index out of bounds
    pub fn ns_payload(&self, index: &NsIndex) -> &NsPayload {
        NsPayload::new(&self.payload[self.ns_table.ns_payload_range(index, self.payload.len())])
    }
}

/// Crazy boilerplate code to make it so that [`NsPayloadOwned`] is to
/// [`NsPayload`] as [`Vec<T>`] is to `[T]`. See [How can I create newtypes for
/// an unsized type and its owned counterpart (like `str` and `String`) in safe
/// Rust? - Stack Overflow](https://stackoverflow.com/q/64977525)
mod ns_payload_owned {
    use super::{NsPayload, NsPayloadOwned};
    use std::borrow::Borrow;
    use std::ops::Deref;

    impl NsPayload {
        // pub(super) because I want it private to this file, but I also want to
        // contain this boilerplate code in its on module.ß
        pub(super) fn new(p: &[u8]) -> &NsPayload {
            unsafe { &*(p as *const [u8] as *const NsPayload) }
        }
    }

    impl Deref for NsPayloadOwned {
        type Target = NsPayload;
        fn deref(&self) -> &NsPayload {
            NsPayload::new(&self.0)
        }
    }

    impl Borrow<NsPayload> for NsPayloadOwned {
        fn borrow(&self) -> &NsPayload {
            self.deref()
        }
    }

    impl ToOwned for NsPayload {
        type Owned = NsPayloadOwned;
        fn to_owned(&self) -> NsPayloadOwned {
            NsPayloadOwned(self.0.to_owned())
        }
    }
}

// TODO move this module to a separate file?
// TODO pub(crate) only for iter module
pub(crate) mod tx_iter {
    use super::*; // TODO temp
    /// TODO explain: index has same byte length as num_txs, store in serialized form
    #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
    pub struct TxIndex([u8; NUM_TXS_BYTE_LEN]);

    impl NsPayload {
        /// Read the tx offset from the `index`th entry from the tx table.
        ///
        /// Panics if `index >= self.num_txs()`.
        pub fn read_tx_offset(&self, index: &TxIndex) -> usize {
            let start = tx_offset_from_bytes(&index.0) * TX_OFFSET_BYTE_LEN + NUM_TXS_BYTE_LEN;
            tx_offset_from_bytes(&self.0[start..start + TX_OFFSET_BYTE_LEN])
        }

        /// Read the tx offset from the `(index-1)`th entry from the tx table.
        /// Returns `None` if `index` is zero.
        ///
        /// Panics if `index >= self.num_txs()`.
        pub fn read_tx_offset_prev(&self, index: &TxIndex) -> Option<usize> {
            if index.0 == [0; NUM_TXS_BYTE_LEN] {
                None
            } else {
                let prev_index = TxIndex(num_txs_as_bytes(num_txs_from_bytes(&index.0) - 1));
                Some(self.read_tx_offset(&prev_index))
            }
        }
    }

    pub struct TxIter(Range<usize>);

    impl TxIter {
        pub fn new(ns_payload: &NsPayload) -> Self {
            Self(0..ns_payload.num_txs())
        }
    }

    // TODO explain: boilerplate `impl Iterator` delegates to `Range`
    impl Iterator for TxIter {
        type Item = TxIndex;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.next().map(|i| TxIndex(num_txs_as_bytes(i)))
        }
    }
}
