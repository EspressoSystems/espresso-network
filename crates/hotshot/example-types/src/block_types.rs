// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    fmt::{Debug, Display},
    mem::size_of,
    sync::Arc,
};

use alloy::primitives::FixedBytes;
use async_trait::async_trait;
use committable::{Commitment, Committable, RawCommitmentBuilder};
use derive_more::Display;
use hotshot_types::{
    data::{BlockError, Leaf2, VidCommitment, ViewNumber, vid_commitment},
    light_client::LightClientState,
    traits::{
        BlockPayload, ValidatedState,
        block_contents::{
            BlockHeader, BuilderFee, EncodeBytes, GENESIS_VID_NUM_STORAGE_NODES, TestableBlock,
            Transaction,
        },
        node_implementation::NodeType,
    },
    utils::BuilderCommitment,
};
use rand::{Rng, thread_rng};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Keccak256};
use thiserror::Error;
use time::OffsetDateTime;
use vbs::version::Version;

use crate::{
    node_types::TestTypes,
    state_types::{TestInstanceState, TestValidatedState},
    testable_delay::{DelayConfig, SupportedTraitTypesForAsyncDelay, TestableDelay},
};

/// The transaction in a [`TestBlockPayload`].
#[derive(Default, PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Debug)]
#[serde(try_from = "Vec<u8>")]
pub struct TestTransaction(Vec<u8>);

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("Transaction too long")]
    TransactionTooLong,
}

impl TryFrom<Vec<u8>> for TestTransaction {
    type Error = TransactionError;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_new(value).ok_or(TransactionError::TransactionTooLong)
    }
}

impl TestTransaction {
    /// Construct a new transaction
    ///
    /// # Panics
    /// If `bytes.len()` > `u32::MAX`
    pub fn new(bytes: Vec<u8>) -> Self {
        Self::try_new(bytes).expect("Vector too long")
    }

    /// Construct a new transaction.
    /// Returns `None` if `bytes.len()` > `u32::MAX`
    /// for cross-platform compatibility
    pub fn try_new(bytes: Vec<u8>) -> Option<Self> {
        if u32::try_from(bytes.len()).is_err() {
            None
        } else {
            Some(Self(bytes))
        }
    }

    /// Get reference to raw bytes of transaction
    pub fn bytes(&self) -> &Vec<u8> {
        &self.0
    }

    /// Convert transaction to raw vector of bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Encode a list of transactions into bytes.
    ///
    /// # Errors
    /// If the transaction length conversion fails.
    pub fn encode(transactions: &[Self]) -> Vec<u8> {
        // A multi-MiB block should not grow through ~log2(size) reallocating copies.
        let total = transactions
            .iter()
            .map(|txn| size_of::<u32>() + txn.0.len())
            .sum();
        let mut encoded = Vec::with_capacity(total);

        for txn in transactions {
            // The transaction length is converted from `usize` to `u32` to ensure consistent
            // number of bytes on different platforms.
            let txn_size = u32::try_from(txn.0.len())
                .expect("Invalid transaction length")
                .to_le_bytes();

            // Concatenate the bytes of the transaction size and the transaction itself.
            encoded.extend(txn_size);
            encoded.extend(&txn.0);
        }

        encoded
    }
}

impl Committable for TestTransaction {
    fn commit(&self) -> Commitment<Self> {
        let builder = committable::RawCommitmentBuilder::new("Txn Comm");
        let mut hasher = Keccak256::new();
        hasher.update(&self.0);
        let generic_array = hasher.finalize();
        builder.generic_byte_array(&generic_array).finalize()
    }

    fn tag() -> String {
        "TEST_TXN".to_string()
    }
}

impl Transaction for TestTransaction {
    fn minimum_block_size(&self) -> u64 {
        // the estimation on transaction size is the length of the transaction
        self.0.len() as u64
    }
}

/// A [`BlockPayload`] that contains a list of `TestTransaction`.
#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Clone, Debug)]
pub struct TestBlockPayload {
    /// List of transactions.
    pub transactions: Vec<TestTransaction>,
}

impl TestBlockPayload {
    /// Create a genesis block payload with bytes `vec![0]`, to be used for
    /// consensus task initiation.
    /// # Panics
    /// If the `VidScheme` construction fails.
    #[must_use]
    pub fn genesis() -> Self {
        TestBlockPayload {
            transactions: vec![],
        }
    }
}

impl Display for TestBlockPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockPayload #txns={}", self.transactions.len())
    }
}

impl<TYPES: NodeType> TestableBlock<TYPES> for TestBlockPayload {
    fn genesis() -> Self {
        Self::genesis()
    }

    fn txn_count(&self) -> u64 {
        self.transactions.len() as u64
    }
}

/// Transaction count below which [`BlockPayload::transaction_commitments`] hashes
/// serially.
///
/// Splitting and joining across the rayon pool costs more than the handful of
/// Keccak256 rounds it would distribute, and most tests build blocks of a few
/// transactions. The parallel path exists for the bench payload, which is tens of
/// thousands of 1 KiB transactions.
const MIN_PARALLEL_TRANSACTIONS: usize = 32;

#[derive(
    Debug, Display, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[display("{{num_transactions:{num_transactions}}}")]
pub struct TestMetadata {
    pub num_transactions: u64,
    /// Total encoded-payload byte length. When this is `> 0` and
    /// `num_transactions > 1`, [`EncodeBytes::encode`] emits a well-formed
    /// namespace table (the wire format `hotshot_types::data::ns_table` parses)
    /// splitting the payload into `num_transactions` evenly-sized namespaces,
    /// so the AvidM dispersal and recovery paths parallelize per namespace.
    ///
    /// Default `0` keeps the single-namespace behaviour every existing call
    /// site relies on.
    #[serde(default)]
    pub payload_byte_len: u64,
}

impl EncodeBytes for TestMetadata {
    fn encode(&self) -> Arc<[u8]> {
        let n = self.num_transactions as usize;
        let total = self.payload_byte_len as usize;
        if n <= 1 || total == 0 {
            return Arc::new([]);
        }
        // [u32_le ns_count] then [u32_le ns_id | u32_le ns_end_offset] per namespace.
        let chunk = total / n;
        let mut buf = Vec::with_capacity(size_of::<u32>() * (1 + 2 * n));
        buf.extend_from_slice(&(n as u32).to_le_bytes());
        for i in 0..n {
            // The last namespace absorbs the remainder so the table covers the
            // whole payload even when `total` isn't divisible by `n`.
            let end = if i + 1 == n { total } else { (i + 1) * chunk };
            buf.extend_from_slice(&(i as u32).to_le_bytes());
            buf.extend_from_slice(&(end as u32).to_le_bytes());
        }
        buf.into()
    }
}

impl EncodeBytes for TestBlockPayload {
    fn encode(&self) -> Arc<[u8]> {
        TestTransaction::encode(&self.transactions).into()
    }
}

#[async_trait]
impl<TYPES: NodeType> BlockPayload<TYPES> for TestBlockPayload {
    type Error = BlockError;
    type Instance = TestInstanceState;
    type Transaction = TestTransaction;
    type Metadata = TestMetadata;
    type ValidatedState = TestValidatedState;

    async fn from_transactions(
        transactions: impl IntoIterator<Item = Self::Transaction> + Send,
        _validated_state: &Self::ValidatedState,
        _instance_state: &Self::Instance,
    ) -> Result<(Self, Self::Metadata), Self::Error> {
        let txns_vec: Vec<TestTransaction> = transactions.into_iter().collect();
        let metadata = TestMetadata {
            num_transactions: txns_vec.len() as u64,
            payload_byte_len: 0,
        };
        Ok((
            Self {
                transactions: txns_vec,
            },
            metadata,
        ))
    }

    fn from_bytes(encoded_transactions: &[u8], _metadata: &Self::Metadata) -> Self {
        // Every transaction costs at least its length prefix, so this bounds the
        // count without a second pass and keeps the decode to one allocation.
        let mut transactions =
            Vec::with_capacity(encoded_transactions.len() / (size_of::<u32>() + 1));
        let mut current_index = 0;
        while current_index < encoded_transactions.len() {
            // Decode the transaction length.
            let txn_start_index = current_index + size_of::<u32>();
            let mut txn_len_bytes = [0; size_of::<u32>()];
            txn_len_bytes.copy_from_slice(&encoded_transactions[current_index..txn_start_index]);
            let txn_len: usize = u32::from_le_bytes(txn_len_bytes) as usize;

            // Get the transaction.
            let next_index = txn_start_index + txn_len;
            transactions.push(TestTransaction(
                encoded_transactions[txn_start_index..next_index].to_vec(),
            ));
            current_index = next_index;
        }

        Self { transactions }
    }

    fn empty() -> (Self, Self::Metadata) {
        (
            Self::genesis(),
            TestMetadata {
                num_transactions: 0,
                payload_byte_len: 0,
            },
        )
    }

    fn builder_commitment(&self, _metadata: &Self::Metadata) -> BuilderCommitment {
        let mut digest = sha2::Sha256::new();
        for txn in &self.transactions {
            digest.update(&txn.0);
        }
        BuilderCommitment::from_raw_digest(digest.finalize())
    }

    fn transactions<'a>(
        &'a self,
        _metadata: &'a Self::Metadata,
    ) -> impl 'a + Iterator<Item = Self::Transaction> {
        self.transactions.iter().cloned()
    }

    /// The per-transaction Keccak256 is the dominant cost of recovering a block
    /// with many small transactions, and each hash is independent.
    ///
    /// Callers pair a commitment index with a transaction index, so the result
    /// must stay in [`Self::transactions`] order; `par_iter` is an indexed
    /// parallel iterator, so `collect` preserves it. Only past
    /// [`MIN_PARALLEL_TRANSACTIONS`] is it worth going wide at all.
    fn transaction_commitments(
        &self,
        _metadata: &Self::Metadata,
    ) -> Vec<Commitment<Self::Transaction>> {
        use p3_maybe_rayon::prelude::*;

        if self.transactions.len() < MIN_PARALLEL_TRANSACTIONS {
            return self.transactions.iter().map(|tx| tx.commit()).collect();
        }
        self.transactions.par_iter().map(|tx| tx.commit()).collect()
    }

    fn txn_bytes(&self) -> usize {
        self.transactions.iter().map(|tx| tx.0.len()).sum()
    }
}

/// A [`BlockHeader`] that commits to [`TestBlockPayload`].
#[derive(PartialEq, Eq, Hash, Clone, Debug, Deserialize, Serialize)]
pub struct TestBlockHeader {
    /// Block number.
    pub block_number: u64,
    /// VID commitment to the payload.
    pub payload_commitment: VidCommitment,
    /// Fast commitment for builder verification
    pub builder_commitment: BuilderCommitment,
    /// block metadata
    pub metadata: TestMetadata,
    /// Timestamp when this header was created.
    pub timestamp: u64,
    /// Timestamp when this header was created.
    pub timestamp_millis: u64,
    /// random
    pub random: u64,
    /// version
    pub version: Version,
}

impl TestBlockHeader {
    pub fn new<TYPES: NodeType<BlockHeader = Self>>(
        parent_leaf: &Leaf2<TYPES>,
        payload_commitment: VidCommitment,
        builder_commitment: BuilderCommitment,
        metadata: TestMetadata,
        version: Version,
    ) -> Self {
        let parent = parent_leaf.block_header();

        let time = OffsetDateTime::now_utc();

        let mut timestamp = time.unix_timestamp() as u64;
        let mut timestamp_millis = (time.unix_timestamp_nanos() / 1_000_000) as u64;

        if timestamp < parent.timestamp {
            // Prevent decreasing timestamps.
            timestamp = parent.timestamp;
        }

        if timestamp_millis < parent.timestamp_millis {
            // Prevent decreasing timestamps.
            timestamp_millis = parent.timestamp_millis;
        }

        let random = thread_rng().gen_range(0..=u64::MAX);

        Self {
            block_number: parent.block_number + 1,
            payload_commitment,
            builder_commitment,
            metadata,
            timestamp,
            timestamp_millis,
            random,
            version,
        }
    }
}

impl<
    TYPES: NodeType<
            BlockHeader = Self,
            BlockPayload = TestBlockPayload,
            InstanceState = TestInstanceState,
        >,
> BlockHeader<TYPES> for TestBlockHeader
{
    type Error = std::convert::Infallible;

    async fn new(
        _parent_state: &TYPES::ValidatedState,
        instance_state: &<TYPES::ValidatedState as ValidatedState<TYPES>>::Instance,
        parent_leaf: &Leaf2<TYPES>,
        payload_commitment: VidCommitment,
        builder_commitment: BuilderCommitment,
        metadata: <TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
        _builder_fee: BuilderFee<TYPES>,
        version: Version,
        _view_number: u64,
    ) -> Result<Self, Self::Error> {
        Self::run_delay_settings_from_config(&instance_state.delay_config).await;
        Ok(Self::new(
            parent_leaf,
            payload_commitment,
            builder_commitment,
            metadata,
            version,
        ))
    }

    fn genesis(
        _instance_state: &<TYPES::ValidatedState as ValidatedState<TYPES>>::Instance,
        payload: TYPES::BlockPayload,
        metadata: &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata,
        version: Version,
    ) -> Self {
        let builder_commitment =
            <TestBlockPayload as BlockPayload<TYPES>>::builder_commitment(&payload, metadata);

        let payload_bytes = payload.encode();
        let payload_commitment = vid_commitment(
            &payload_bytes,
            &metadata.encode(),
            GENESIS_VID_NUM_STORAGE_NODES,
            version,
        );

        Self {
            block_number: 0,
            payload_commitment,
            builder_commitment,
            metadata: *metadata,
            timestamp: 0,
            timestamp_millis: 0,
            random: 0,
            version,
        }
    }

    fn block_number(&self) -> u64 {
        self.block_number
    }

    fn payload_commitment(&self) -> VidCommitment {
        self.payload_commitment
    }

    fn metadata(&self) -> &<TYPES::BlockPayload as BlockPayload<TYPES>>::Metadata {
        &self.metadata
    }

    fn builder_commitment(&self) -> BuilderCommitment {
        self.builder_commitment.clone()
    }

    fn version(&self) -> Version {
        self.version
    }

    fn get_light_client_state(&self, view: ViewNumber) -> anyhow::Result<LightClientState> {
        LightClientState::new(
            view.u64(),
            self.block_number,
            self.payload_commitment.as_ref(),
        )
    }

    fn auth_root(&self) -> anyhow::Result<FixedBytes<32>> {
        Ok(FixedBytes::from([0u8; 32]))
    }

    fn timestamp(&self) -> u64 {
        self.timestamp
    }

    fn timestamp_millis(&self) -> u64 {
        self.timestamp_millis
    }
}

impl Committable for TestBlockHeader {
    fn commit(&self) -> Commitment<Self> {
        RawCommitmentBuilder::new("Header Comm")
            .u64_field(
                "block number",
                <TestBlockHeader as BlockHeader<TestTypes>>::block_number(self),
            )
            .constant_str("payload commitment")
            .fixed_size_bytes(
                <TestBlockHeader as BlockHeader<TestTypes>>::payload_commitment(self).as_ref(),
            )
            .finalize()
    }

    fn tag() -> String {
        "TEST_HEADER".to_string()
    }
}

#[async_trait]
impl TestableDelay for TestBlockHeader {
    async fn run_delay_settings_from_config(delay_config: &DelayConfig) {
        if let Some(settings) =
            delay_config.get_setting(&SupportedTraitTypesForAsyncDelay::BlockHeader)
        {
            Self::handle_async_delay(settings).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use hotshot_types::data::ns_table::parse_ns_table;

    use super::*;
    use crate::node_types::TestTypes;

    fn payload(num_txs: usize) -> Vec<u8> {
        let transactions: Vec<TestTransaction> = (0..num_txs)
            .map(|_| TestTransaction::new(vec![0u8; 1024]))
            .collect();
        TestTransaction::encode(&transactions)
    }

    /// The namespace table the bench relies on has to survive `parse_ns_table`,
    /// which silently falls back to one namespace for any table it can't parse.
    /// A run that degrades that way looks healthy and measures the wrong thing,
    /// so pin the round trip rather than the bytes.
    #[test]
    fn metadata_encodes_a_parseable_namespace_table() {
        let bytes = payload(64);
        let metadata = TestMetadata {
            num_transactions: 8,
            payload_byte_len: bytes.len() as u64,
        };

        let ns_table = parse_ns_table(bytes.len(), &metadata.encode());

        assert_eq!(ns_table.len(), 8, "namespace table collapsed to a fallback");
        assert_eq!(ns_table[0].start, 0);
        assert_eq!(
            ns_table.last().unwrap().end,
            bytes.len(),
            "namespace table must cover the whole payload"
        );
        for pair in ns_table.windows(2) {
            assert_eq!(
                pair[0].end, pair[1].start,
                "namespaces must be contiguous and non-overlapping"
            );
        }
    }

    /// The parallel `transaction_commitments` override must agree with the
    /// serial default element for element, on both sides of
    /// [`MIN_PARALLEL_TRANSACTIONS`].
    ///
    /// Callers pair a commitment index with a transaction index, so a reordering
    /// would misattribute transactions rather than fail loudly. The counts
    /// straddle the threshold deliberately: below it the override never reaches
    /// rayon, so a suite built only from small blocks would stop covering the
    /// parallel branch the moment the threshold was introduced.
    #[test]
    fn transaction_commitments_are_in_transaction_order() {
        for n in [
            1,
            MIN_PARALLEL_TRANSACTIONS - 1,
            MIN_PARALLEL_TRANSACTIONS,
            4 * MIN_PARALLEL_TRANSACTIONS,
        ] {
            let payload = TestBlockPayload {
                transactions: (0..n as u32)
                    .map(|i| TestTransaction::new(i.to_le_bytes().to_vec()))
                    .collect(),
            };
            let metadata = TestMetadata {
                num_transactions: n as u64,
                payload_byte_len: 0,
            };

            let serial: Vec<_> = BlockPayload::<TestTypes>::transactions(&payload, &metadata)
                .map(|txn| txn.commit())
                .collect();

            assert_eq!(serial.len(), n, "fixture must produce {n} transactions");
            assert_eq!(
                BlockPayload::<TestTypes>::transaction_commitments(&payload, &metadata),
                serial,
                "commitments diverged from the serial default at {n} transactions",
            );
        }
    }

    /// Every pre-existing caller leaves `payload_byte_len` at 0 and must keep
    /// the single-namespace behaviour it had before the field existed.
    #[test]
    fn metadata_without_payload_len_stays_single_namespace() {
        let bytes = payload(4);
        let metadata = TestMetadata {
            num_transactions: 8,
            payload_byte_len: 0,
        };

        assert!(metadata.encode().is_empty());
        assert_eq!(
            parse_ns_table(bytes.len(), &metadata.encode()),
            vec![0..bytes.len()]
        );
    }
}
