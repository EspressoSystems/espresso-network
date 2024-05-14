use crate::{
    block2::{
        iter::{Index, Iter},
        ns_table::NsTable,
        uint_bytes::{u64_to_bytes, usize_to_bytes},
        NS_ID_BYTE_LEN, NS_OFFSET_BYTE_LEN, NUM_NSS_BYTE_LEN,
    },
    NamespaceId, Transaction,
};
use commit::{Commitment, Committable};
use hotshot_query_service::availability::QueryablePayload;
use hotshot_types::{traits::BlockPayload, utils::BuilderCommitment};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{collections::HashMap, fmt::Display};

use super::{
    newtypes::{
        AsPayloadBytes, NamespacePayloadBuilder, NumTxsRange2, TxPayloadRange, TxTableEntriesRange2,
    },
    NsPayload2, NsPayloadRange2, TxProof2,
};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Payload {
    // Concatenated payload bytes for each namespace
    #[serde(with = "base64_bytes")]
    payload: Vec<u8>,

    ns_table: NsTable,
    // TODO Revisit caching of frequently used items
    //
    // TODO type should be `OnceLock<SmallRangeProofType>` instead of `OnceLock<Option<SmallRangeProofType>>`.
    // We can correct this after `once_cell_try` is stabilized <https://github.com/rust-lang/rust/issues/109737>.
    // #[derivative(Hash = "ignore")]
    // #[derivative(PartialEq = "ignore")]
    // #[serde(skip)]
    // pub tx_table_len_proof: OnceLock<Option<SmallRangeProofType>>,
}

impl BlockPayload for Payload {
    type Error = crate::Error;
    type Transaction = Transaction;
    type Metadata = Vec<u8>; // namespace table bytes

    // TODO change `BlockPayload::Encode` trait bounds to enable copyless encoding such as AsRef<[u8]>
    // https://github.com/EspressoSystems/HotShot/issues/2115
    type Encode<'a> = std::iter::Cloned<<&'a Vec<u8> as IntoIterator>::IntoIter>;

    // TODO change `BlockPayload` trait: return type should not include `Self::Metadata`
    fn from_transactions(
        transactions: impl IntoIterator<Item = Self::Transaction>,
    ) -> Result<(Self, Self::Metadata), Self::Error> {
        // add each tx to its namespace
        let mut namespaces = HashMap::<NamespaceId, NamespacePayloadBuilder>::new();
        for tx in transactions.into_iter() {
            let namespace = namespaces.entry(tx.namespace()).or_default();
            namespace.append_tx(tx);
        }

        // build block payload and namespace table
        // TODO building the ns_table here breaks abstraction
        let mut payload = Vec::new();
        let mut ns_table = Vec::from(usize_to_bytes::<NUM_NSS_BYTE_LEN>(namespaces.len()));
        for (ns_id, namespace) in namespaces {
            payload.extend(namespace.into_bytes());

            // TODO hack to serialize `NamespaceId` to `NS_ID_BYTE_LEN` bytes
            ns_table.extend(u64_to_bytes::<NS_ID_BYTE_LEN>(u64::from(ns_id)));

            ns_table.extend(usize_to_bytes::<NS_OFFSET_BYTE_LEN>(payload.len()));
        }
        Ok((
            Self {
                payload,
                ns_table: NsTable::from_bytes(A(()), ns_table.clone()),
            },
            ns_table,
        ))
    }

    fn from_bytes<I>(encoded_transactions: I, ns_table: &Self::Metadata) -> Self
    where
        I: Iterator<Item = u8>,
    {
        Self {
            payload: encoded_transactions.into_iter().collect(),
            ns_table: NsTable::from_bytes(A(()), ns_table.clone()), // TODO don't clone ns_table
        }
    }

    // TODO change `BlockPayload` trait: return type should not include `Self::Metadata`
    fn genesis() -> (Self, Self::Metadata) {
        Self::from_transactions([]).unwrap()
    }

    // TODO change `BlockPayload::Encode` trait bounds to enable copyless encoding such as AsRef<[u8]>
    // https://github.com/EspressoSystems/HotShot/issues/2115
    fn encode(&self) -> Result<Self::Encode<'_>, Self::Error> {
        Ok(self.payload.iter().cloned())
    }

    // TODO change `BlockPayload` trait: remove arg `Self::Metadata`
    fn transaction_commitments(
        &self,
        _metadata: &Self::Metadata,
    ) -> Vec<Commitment<Self::Transaction>> {
        todo!()
    }

    // TODO change `BlockPayload` trait: remove arg `Self::Metadata`
    fn builder_commitment(&self, _metadata: &Self::Metadata) -> BuilderCommitment {
        let ns_table_bytes = self.ns_table.as_byte_slice();
        let mut digest = sha2::Sha256::new();
        digest.update((self.payload.len() as u64).to_le_bytes());
        digest.update((ns_table_bytes.len() as u64).to_le_bytes());
        digest.update(&self.payload);
        digest.update(ns_table_bytes);
        BuilderCommitment::from_raw_digest(digest.finalize())
    }

    // TODO change `BlockPayload` trait: remove arg `Self::Metadata`
    // TODO change return type so it's not a reference! :facepalm:
    fn get_transactions(&self, _metadata: &Self::Metadata) -> &Vec<Self::Transaction> {
        todo!()
    }
}

impl QueryablePayload for Payload {
    // TODO change `QueryablePayload` trait: remove `Ord` bound from `TransactionIndex`
    type TransactionIndex = Index;
    type Iter<'a> = Iter<'a>;
    type InclusionProof = TxProof2;

    // TODO change `QueryablePayload` trait: remove arg `Self::Metadata`
    fn len(&self, _meta: &Self::Metadata) -> usize {
        // Counting txs is nontrivial. The easiest solution is to consume an
        // iterator. If performance is a concern then we could cache this count
        // on construction of `Payload`.
        self.iter(_meta).count()
    }

    // TODO change `QueryablePayload` trait: remove arg `Self::Metadata`
    fn iter<'a>(&'a self, _meta: &'a Self::Metadata) -> Self::Iter<'a> {
        Iter::new(self)
    }

    // TODO change `QueryablePayload` trait: remove arg `Self::Metadata`
    fn transaction_with_proof(
        &self,
        _meta: &Self::Metadata,
        _index: &Self::TransactionIndex,
    ) -> Option<(Self::Transaction, Self::InclusionProof)> {
        todo!()
    }
}

impl Display for Payload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:#?}")
    }
}

impl Committable for Payload {
    fn commit(&self) -> commit::Commitment<Self> {
        todo!()
    }
}

/// TODO explain: ZST to unlock visibility in other modules. can only be
/// constructed in this module.
pub struct A(());

impl Payload {
    pub fn transaction(&self, index: &Index) -> Option<Transaction> {
        // TODO helper methods please!
        // TODO check index.ns(), index.tx() in bounds
        let ns_payload_range = self
            .ns_table()
            .ns_payload_range2(index.ns(), self.payload.len());
        let ns_payload = self.read_ns_payload(&ns_payload_range);
        let num_txs = ns_payload.read(&NumTxsRange2::new(&ns_payload_range.byte_len()));
        let tx_table_entries = ns_payload.read(&TxTableEntriesRange2::new(index.tx()));
        let tx_payload_range =
            TxPayloadRange::new(&num_txs, &tx_table_entries, &ns_payload_range.byte_len());
        let tx_payload = ns_payload
            .read(&tx_payload_range)
            .to_payload_bytes()
            .as_ref()
            .to_vec();
        let ns_id = self.ns_table().read_ns_id(index.ns());
        // TODO don't copy the tx bytes into the return value
        // https://github.com/EspressoSystems/hotshot-query-service/issues/267
        Some(Transaction::new(ns_id, tx_payload))
    }

    pub fn as_byte_slice(&self) -> &[u8] {
        &self.payload
    }
    pub fn ns_table(&self) -> &NsTable {
        &self.ns_table
    }

    // lots of manual delegation boo!

    /// TODO panics if index out of bounds
    // pub fn ns_payload(&self, index: &NsIndex) -> &NsPayload {
    //     let range = self.ns_payload_range(index).as_range();
    //     NsPayload::new(A(()), &self.payload[range])
    // }

    pub fn read_ns_payload(&self, range: &NsPayloadRange2) -> &NsPayload2 {
        NsPayload2::new(&self.payload[range.as_range()])
    }

    // pub fn ns_payload_range(&self, index: &NsIndex) -> NsPayloadRange {
    //     self.ns_table.ns_payload_range(index, self.payload.len())
    // }

    // pub fn ns_payload_range2(&self, index: &NsIndex) -> NsPayloadRange2 {
    //     self.ns_table.ns_payload_range2(index, self.payload.len())
    // }
}
