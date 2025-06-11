use committable::{Commitment, Committable};
use hotshot_query_service::explorer::ExplorerTransaction;
use hotshot_types::traits::block_contents::Transaction as HotShotTransaction;
use serde::{de::Error, Deserialize, Deserializer};

use super::NsPayloadBuilder;
use crate::{NamespaceId, SeqTypes, Transaction};

impl From<u32> for NamespaceId {
    fn from(value: u32) -> Self {
        Self(value as u64)
    }
}

impl From<NamespaceId> for u32 {
    fn from(value: NamespaceId) -> Self {
        value.0 as Self
    }
}

impl From<i64> for NamespaceId {
    fn from(value: i64) -> Self {
        Self(value as u64)
    }
}

impl From<NamespaceId> for i64 {
    fn from(value: NamespaceId) -> Self {
        value.0 as Self
    }
}

impl<'de> Deserialize<'de> for NamespaceId {
    fn deserialize<D>(deserializer: D) -> Result<NamespaceId, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Unexpected;

        let ns_id = <u64 as Deserialize>::deserialize(deserializer)?;
        if ns_id > u32::MAX as u64 {
            Err(D::Error::invalid_value(
                Unexpected::Unsigned(ns_id),
                &"at most u32::MAX",
            ))
        } else {
            Ok(NamespaceId(ns_id))
        }
    }
}

impl NamespaceId {
    #[cfg(any(test, feature = "testing"))]
    pub fn random(rng: &mut dyn rand::RngCore) -> Self {
        Self(rng.next_u32() as u64)
    }
}

impl Transaction {
    pub fn new(namespace: NamespaceId, payload: Vec<u8>) -> Self {
        Self { namespace, payload }
    }

    pub fn namespace(&self) -> NamespaceId {
        self.namespace
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn into_payload(self) -> Vec<u8> {
        self.payload
    }

    pub fn size_in_block(&self, new_ns: bool) -> u64 {
        if new_ns {
            // each new namespace adds overhead
            self.minimum_block_size()
        } else {
            (self.payload().len() + NsPayloadBuilder::tx_table_entry_byte_len()) as u64
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn random(rng: &mut dyn rand::RngCore) -> Self {
        use rand::Rng;
        let len = rng.gen_range(0..100);
        Self::new(
            NamespaceId::random(rng),
            (0..len).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
        )
    }
    #[cfg(any(test, feature = "testing"))]
    /// Useful for when we want to test size of transaction(s)
    pub fn of_size(len: usize) -> Self {
        Self::new(
            NamespaceId(1),
            (0..len).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
        )
    }
}

impl HotShotTransaction for Transaction {
    fn minimum_block_size(&self) -> u64 {
        // Any block containing this transaction will have at least:
        let len =
            // bytes for the payload of the transaction itself
            self.payload().len()
            // a transaction table entry in the transaction's namespace payload
            + NsPayloadBuilder::tx_table_entry_byte_len()
            // a header of the transaction table in the transaction's namespace payload
            + NsPayloadBuilder::tx_table_header_byte_len();
        // The block will also have an entry in the namespace table for the transaction's namespace;
        // however this takes up space in the header, not the payload, so doesn't count against the
        // size of the block.

        len as u64
    }
}

impl Committable for Transaction {
    fn commit(&self) -> Commitment<Self> {
        committable::RawCommitmentBuilder::new("Transaction")
            .u64_field("namespace", self.namespace.0)
            .var_size_bytes(&self.payload)
            .finalize()
    }

    fn tag() -> String {
        "TX".into()
    }
}

impl ExplorerTransaction<SeqTypes> for Transaction {
    fn namespace_id(&self) -> NamespaceId {
        self.namespace
    }

    fn payload_size(&self) -> u64 {
        self.payload.len() as u64
    }
}
