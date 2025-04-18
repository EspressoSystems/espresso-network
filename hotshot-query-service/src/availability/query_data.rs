// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use std::fmt::Debug;

use committable::{Commitment, Committable};
use derive_more::derive::From;
use hotshot_types::{
    data::{Leaf, Leaf2, VidCommitment, VidShare},
    simple_certificate::{LightClientStateUpdateCertificate, QuorumCertificate2},
    traits::{
        self,
        block_contents::{BlockHeader, GENESIS_VID_NUM_STORAGE_NODES},
        node_implementation::{NodeType, Versions},
        EncodeBytes,
    },
    vid::advz::{advz_scheme, ADVZCommitment, ADVZCommon},
};
use jf_vid::VidScheme;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use snafu::{ensure, Snafu};

use crate::{
    types::HeightIndexed, Header, Metadata, Payload, QuorumCertificate, Transaction, VidCommon,
};

pub type LeafHash<Types> = Commitment<Leaf2<Types>>;
pub type LeafHashLegacy<Types> = Commitment<Leaf<Types>>;
pub type QcHash<Types> = Commitment<QuorumCertificate2<Types>>;

/// A block hash is the hash of the block header.
///
/// A block consists of a header and a payload. But the header itself contains a commitment to the
/// payload, so we can commit to the entire block simply by hashing the header.
pub type BlockHash<Types> = Commitment<Header<Types>>;
pub type TransactionHash<Types> = Commitment<Transaction<Types>>;
pub type TransactionIndex<Types> = <Payload<Types> as QueryablePayload<Types>>::TransactionIndex;
pub type TransactionInclusionProof<Types> =
    <Payload<Types> as QueryablePayload<Types>>::InclusionProof;

pub type Timestamp = time::OffsetDateTime;

pub trait QueryableHeader<Types: NodeType>: BlockHeader<Types> {
    fn timestamp(&self) -> u64;
}

/// A block payload whose contents (e.g. individual transactions) can be examined.
///
/// Note to implementers: this trait has only a few required methods. The provided methods, for
/// querying transactions in various ways, are implemented in terms of the required
/// [`iter`](Self::iter) and [`transaction_with_proof`](Self::transaction_with_proof) methods, and
/// the default implementations may be inefficient (e.g. performing an O(n) search, or computing an
/// unnecessary inclusion proof). It is good practice to override these default implementations if
/// your block type supports more efficient implementations (e.g. sublinear indexing by hash).
pub trait QueryablePayload<Types: NodeType>: traits::BlockPayload<Types> {
    /// An index which can be used to efficiently retrieve a transaction for the block.
    ///
    /// This is left abstract so that different block implementations can index transactions
    /// internally however they want (e.g. by position or by hash). Meanwhile, many high-level
    /// functions for querying transactions by different means can be implemented by returning a
    /// `TransactionIndex` and then finally using it to retrieve the desired transaction.
    type TransactionIndex: Clone
        + Debug
        + PartialEq
        + Eq
        + Ord
        + Serialize
        + DeserializeOwned
        + Send
        + Sync;

    /// Enumerate the transactions in this block.
    type Iter<'a>: Iterator<Item = Self::TransactionIndex>
    where
        Self: 'a;

    /// A proof that a certain transaction exists in the block.
    ///
    /// The proof system and the statement which is proved will vary by application, with different
    /// applications proving stronger or weaker statements depending on the trust assumptions at
    /// play. Some may prove a very strong statement (for example, a shared sequencer proving that
    /// the transaction belongs not only to the block but to a section of the block dedicated to a
    /// specific rollup), otherwise may prove something substantially weaker (for example, a trusted
    /// query service may use `()` for the proof).
    type InclusionProof: Clone + Debug + PartialEq + Eq + Serialize + DeserializeOwned + Send + Sync;

    /// The number of transactions in the block.
    fn len(&self, meta: &Self::Metadata) -> usize;

    /// Whether this block is empty of transactions.
    fn is_empty(&self, meta: &Self::Metadata) -> bool {
        self.len(meta) == 0
    }

    /// List the transaction indices in the block.
    fn iter<'a>(&'a self, meta: &'a Self::Metadata) -> Self::Iter<'a>;

    /// Enumerate the transactions in the block with their indices.
    fn enumerate<'a>(
        &'a self,
        meta: &'a Self::Metadata,
    ) -> Box<dyn 'a + Iterator<Item = (Self::TransactionIndex, Self::Transaction)>> {
        Box::new(self.iter(meta).map(|ix| {
            // `self.transaction` should always return `Some` if we are using an index which was
            // yielded by `self.iter`.
            let tx = self.transaction(meta, &ix).unwrap();
            (ix, tx)
        }))
    }

    /// Get a transaction by its block-specific index, along with an inclusion proof.
    fn transaction_with_proof(
        &self,
        meta: &Self::Metadata,
        index: &Self::TransactionIndex,
    ) -> Option<(Self::Transaction, Self::InclusionProof)>;

    /// Get a transaction by its block-specific index.
    fn transaction(
        &self,
        meta: &Self::Metadata,
        index: &Self::TransactionIndex,
    ) -> Option<Self::Transaction> {
        Some(self.transaction_with_proof(meta, index)?.0)
    }

    /// Get an inclusion proof for a transaction with a given index.
    fn proof(
        &self,
        meta: &Self::Metadata,
        index: &Self::TransactionIndex,
    ) -> Option<Self::InclusionProof> {
        Some(self.transaction_with_proof(meta, index)?.1)
    }

    /// Get the index of the `nth` transaction.
    fn nth(&self, meta: &Self::Metadata, n: usize) -> Option<Self::TransactionIndex> {
        self.iter(meta).nth(n)
    }

    /// Get the `nth` transaction.
    fn nth_transaction(&self, meta: &Self::Metadata, n: usize) -> Option<Self::Transaction> {
        self.transaction(meta, &self.nth(meta, n)?)
    }

    /// Get the `nth` transaction, along with an inclusion proof.
    fn nth_transaction_with_proof(
        &self,
        meta: &Self::Metadata,
        n: usize,
    ) -> Option<(Self::Transaction, Self::InclusionProof)> {
        self.transaction_with_proof(meta, &self.nth(meta, n)?)
    }

    /// Get the index of the transaction with a given hash, if it is in the block.
    fn by_hash(
        &self,
        meta: &Self::Metadata,
        hash: Commitment<Self::Transaction>,
    ) -> Option<Self::TransactionIndex> {
        self.iter(meta).find(|i| {
            if let Some(tx) = self.transaction(meta, i) {
                tx.commit() == hash
            } else {
                false
            }
        })
    }

    /// Get the transaction with a given hash, if it is in the block.
    fn transaction_by_hash(
        &self,
        meta: &Self::Metadata,
        hash: Commitment<Self::Transaction>,
    ) -> Option<Self::Transaction> {
        self.transaction(meta, &self.by_hash(meta, hash)?)
    }

    /// Get the transaction with a given hash, if it is in the block, along with an inclusion proof.
    fn transaction_by_hash_with_proof(
        &self,
        meta: &Self::Metadata,
        hash: Commitment<Self::Transaction>,
    ) -> Option<(Self::Transaction, Self::InclusionProof)> {
        self.transaction_with_proof(meta, &self.by_hash(meta, hash)?)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct LeafQueryData<Types: NodeType> {
    pub(crate) leaf: Leaf2<Types>,
    pub(crate) qc: QuorumCertificate2<Types>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct LeafQueryDataLegacy<Types: NodeType> {
    pub(crate) leaf: Leaf<Types>,
    pub(crate) qc: QuorumCertificate<Types>,
}

impl<Types: NodeType> From<LeafQueryDataLegacy<Types>> for LeafQueryData<Types> {
    fn from(legacy: LeafQueryDataLegacy<Types>) -> Self {
        Self {
            leaf: legacy.leaf.into(),
            qc: legacy.qc.to_qc2(),
        }
    }
}

#[derive(Clone, Debug, Snafu)]
#[snafu(display("QC references leaf {qc_leaf}, but expected {leaf}"))]
pub struct InconsistentLeafError<Types: NodeType> {
    pub leaf: LeafHash<Types>,
    pub qc_leaf: LeafHash<Types>,
}

#[derive(Clone, Debug, Snafu)]
#[snafu(display("QC references leaf {qc_leaf}, but expected {leaf}"))]
pub struct InconsistentLeafLegacyError<Types: NodeType> {
    pub leaf: LeafHashLegacy<Types>,
    pub qc_leaf: LeafHashLegacy<Types>,
}

impl<Types: NodeType> LeafQueryDataLegacy<Types> {
    /// Collect information about a [`Leaf`].
    ///
    /// Returns a new [`LeafQueryData`] object populated from `leaf` and `qc`.
    ///
    /// # Errors
    ///
    /// Fails with an [`InconsistentLeafError`] if `qc` does not reference `leaf`.
    pub fn new(
        mut leaf: Leaf<Types>,
        qc: QuorumCertificate<Types>,
    ) -> Result<Self, InconsistentLeafLegacyError<Types>> {
        // TODO: Replace with the new `commit` function in HotShot. Add an `upgrade_lock` parameter
        // and a `HsVer: Versions` bound, then call `leaf.commit(upgrade_lock).await`. This will
        // require updates in callers and relevant types as well.
        let leaf_commit = <Leaf<Types> as Committable>::commit(&leaf);
        ensure!(
            qc.data.leaf_commit == leaf_commit,
            InconsistentLeafLegacySnafu {
                leaf: leaf_commit,
                qc_leaf: qc.data.leaf_commit
            }
        );

        // We only want the leaf for the block header and consensus metadata. The payload will be
        // stored separately.
        leaf.unfill_block_payload();

        Ok(Self { leaf, qc })
    }

    pub async fn genesis<HsVer: Versions>(
        validated_state: &Types::ValidatedState,
        instance_state: &Types::InstanceState,
    ) -> Self {
        Self {
            leaf: Leaf::genesis::<HsVer>(validated_state, instance_state).await,
            qc: QuorumCertificate::genesis::<HsVer>(validated_state, instance_state).await,
        }
    }

    pub fn leaf(&self) -> &Leaf<Types> {
        &self.leaf
    }

    pub fn qc(&self) -> &QuorumCertificate<Types> {
        &self.qc
    }

    pub fn header(&self) -> &Header<Types> {
        self.leaf.block_header()
    }

    pub fn hash(&self) -> LeafHashLegacy<Types> {
        // TODO: Replace with the new `commit` function in HotShot. Add an `upgrade_lock` parameter
        // and a `HsVer: Versions` bound, then call `leaf.commit(upgrade_lock).await`. This will
        // require updates in callers and relevant types as well.
        <Leaf<Types> as Committable>::commit(&self.leaf)
    }

    pub fn block_hash(&self) -> BlockHash<Types> {
        self.header().commit()
    }

    pub fn payload_hash(&self) -> VidCommitment {
        self.header().payload_commitment()
    }
}

impl<Types: NodeType> LeafQueryData<Types> {
    /// Collect information about a [`Leaf`].
    ///
    /// Returns a new [`LeafQueryData`] object populated from `leaf` and `qc`.
    ///
    /// # Errors
    ///
    /// Fails with an [`InconsistentLeafError`] if `qc` does not reference `leaf`.
    pub fn new(
        mut leaf: Leaf2<Types>,
        qc: QuorumCertificate2<Types>,
    ) -> Result<Self, InconsistentLeafError<Types>> {
        // TODO: Replace with the new `commit` function in HotShot. Add an `upgrade_lock` parameter
        // and a `HsVer: Versions` bound, then call `leaf.commit(upgrade_lock).await`. This will
        // require updates in callers and relevant types as well.
        let leaf_commit = <Leaf2<Types> as Committable>::commit(&leaf);
        ensure!(
            qc.data.leaf_commit == leaf_commit,
            InconsistentLeafSnafu {
                leaf: leaf_commit,
                qc_leaf: qc.data.leaf_commit
            }
        );

        // We only want the leaf for the block header and consensus metadata. The payload will be
        // stored separately.
        leaf.unfill_block_payload();

        Ok(Self { leaf, qc })
    }

    pub async fn genesis<HsVer: Versions>(
        validated_state: &Types::ValidatedState,
        instance_state: &Types::InstanceState,
    ) -> Self {
        Self {
            leaf: Leaf2::genesis::<HsVer>(validated_state, instance_state).await,
            qc: QuorumCertificate2::genesis::<HsVer>(validated_state, instance_state).await,
        }
    }

    pub fn leaf(&self) -> &Leaf2<Types> {
        &self.leaf
    }

    pub fn qc(&self) -> &QuorumCertificate2<Types> {
        &self.qc
    }

    pub fn header(&self) -> &Header<Types> {
        self.leaf.block_header()
    }

    pub fn hash(&self) -> LeafHash<Types> {
        // TODO: Replace with the new `commit` function in HotShot. Add an `upgrade_lock` parameter
        // and a `HsVer: Versions` bound, then call `leaf.commit(upgrade_lock).await`. This will
        // require updates in callers and relevant types as well.
        <Leaf2<Types> as Committable>::commit(&self.leaf)
    }

    pub fn block_hash(&self) -> BlockHash<Types> {
        self.header().commit()
    }

    pub fn payload_hash(&self) -> VidCommitment {
        self.header().payload_commitment()
    }
}

impl<Types: NodeType> HeightIndexed for LeafQueryData<Types> {
    fn height(&self) -> u64 {
        self.header().block_number()
    }
}

impl<Types: NodeType> HeightIndexed for LeafQueryDataLegacy<Types> {
    fn height(&self) -> u64 {
        self.header().block_number()
    }
}

#[derive(Clone, Debug, Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct HeaderQueryData<Types: NodeType> {
    pub header: Header<Types>,
}

impl<Types: NodeType> HeaderQueryData<Types> {
    pub fn new(header: Header<Types>) -> Self {
        Self { header }
    }

    pub fn header(&self) -> &Header<Types> {
        &self.header
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct BlockQueryData<Types: NodeType> {
    pub(crate) header: Header<Types>,
    pub(crate) payload: Payload<Types>,
    pub(crate) hash: BlockHash<Types>,
    pub(crate) size: u64,
    pub(crate) num_transactions: u64,
}

impl<Types: NodeType> BlockQueryData<Types> {
    pub fn new(header: Header<Types>, payload: Payload<Types>) -> Self
    where
        Payload<Types>: QueryablePayload<Types>,
    {
        Self {
            hash: header.commit(),
            size: payload_size::<Types>(&payload),
            num_transactions: payload.len(header.metadata()) as u64,
            header,
            payload,
        }
    }

    pub async fn genesis<HsVer: Versions>(
        validated_state: &Types::ValidatedState,
        instance_state: &Types::InstanceState,
    ) -> Self
    where
        Payload<Types>: QueryablePayload<Types>,
    {
        let leaf = Leaf2::<Types>::genesis::<HsVer>(validated_state, instance_state).await;
        Self::new(leaf.block_header().clone(), leaf.block_payload().unwrap())
    }

    pub fn header(&self) -> &Header<Types> {
        &self.header
    }

    pub fn metadata(&self) -> &Metadata<Types> {
        self.header.metadata()
    }

    pub fn payload_hash(&self) -> VidCommitment {
        self.header.payload_commitment()
    }

    pub fn payload(&self) -> &Payload<Types> {
        &self.payload
    }

    pub fn hash(&self) -> BlockHash<Types> {
        self.hash
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn num_transactions(&self) -> u64 {
        self.num_transactions
    }
}

impl<Types: NodeType> BlockQueryData<Types>
where
    Payload<Types>: QueryablePayload<Types>,
{
    pub fn transaction(&self, ix: &TransactionIndex<Types>) -> Option<Transaction<Types>> {
        self.payload().transaction(self.metadata(), ix)
    }

    pub fn transaction_by_hash(
        &self,
        hash: Commitment<Transaction<Types>>,
    ) -> Option<TransactionIndex<Types>> {
        self.payload().by_hash(self.metadata(), hash)
    }

    pub fn len(&self) -> usize {
        self.payload.len(self.metadata())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn enumerate(
        &self,
    ) -> impl '_ + Iterator<Item = (TransactionIndex<Types>, Transaction<Types>)> {
        self.payload.enumerate(self.metadata())
    }
}

impl<Types: NodeType> HeightIndexed for BlockQueryData<Types> {
    fn height(&self) -> u64 {
        self.header.block_number()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct ADVZPayloadQueryData<Types: NodeType> {
    pub(crate) height: u64,
    pub(crate) block_hash: BlockHash<Types>,
    pub(crate) hash: ADVZCommitment,
    pub(crate) size: u64,
    pub(crate) data: Payload<Types>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct PayloadQueryData<Types: NodeType> {
    pub(crate) height: u64,
    pub(crate) block_hash: BlockHash<Types>,
    pub(crate) hash: VidCommitment,
    pub(crate) size: u64,
    pub(crate) data: Payload<Types>,
}

impl<Types: NodeType> From<BlockQueryData<Types>> for PayloadQueryData<Types> {
    fn from(block: BlockQueryData<Types>) -> Self {
        Self {
            height: block.height(),
            block_hash: block.hash(),
            hash: block.header.payload_commitment(),
            size: block.size(),
            data: block.payload,
        }
    }
}

impl<Types: NodeType> PayloadQueryData<Types> {
    pub fn to_legacy(&self) -> Option<ADVZPayloadQueryData<Types>> {
        let VidCommitment::V0(advz_commit) = self.hash else {
            return None;
        };

        Some(ADVZPayloadQueryData {
            height: self.height,
            block_hash: self.block_hash,
            hash: advz_commit,
            size: self.size,
            data: self.data.clone(),
        })
    }

    pub async fn genesis<HsVer: Versions>(
        validated_state: &Types::ValidatedState,
        instance_state: &Types::InstanceState,
    ) -> Self
    where
        Payload<Types>: QueryablePayload<Types>,
    {
        BlockQueryData::genesis::<HsVer>(validated_state, instance_state)
            .await
            .into()
    }

    pub fn hash(&self) -> VidCommitment {
        self.hash
    }

    pub fn block_hash(&self) -> BlockHash<Types> {
        self.block_hash
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn data(&self) -> &Payload<Types> {
        &self.data
    }
}

impl<Types: NodeType> HeightIndexed for PayloadQueryData<Types> {
    fn height(&self) -> u64 {
        self.height
    }
}

/// The old VidCommonQueryData, associated with ADVZ VID Scheme.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct ADVZCommonQueryData<Types: NodeType> {
    pub(crate) height: u64,
    pub(crate) block_hash: BlockHash<Types>,
    pub(crate) payload_hash: ADVZCommitment,
    pub(crate) common: ADVZCommon,
}

impl<Types: NodeType> ADVZCommonQueryData<Types> {
    pub fn new(header: Header<Types>, common: ADVZCommon) -> anyhow::Result<Self> {
        let VidCommitment::V0(payload_hash) = header.payload_commitment() else {
            return Err(anyhow::anyhow!("Inconsistent header type."));
        };
        Ok(Self {
            height: header.block_number(),
            block_hash: header.commit(),
            payload_hash,
            common,
        })
    }

    pub async fn genesis<HsVer: Versions>(
        validated_state: &Types::ValidatedState,
        instance_state: &Types::InstanceState,
    ) -> anyhow::Result<Self> {
        let leaf = Leaf::<Types>::genesis::<HsVer>(validated_state, instance_state).await;
        let payload = leaf.block_payload().unwrap();
        let bytes = payload.encode();
        let disperse = advz_scheme(GENESIS_VID_NUM_STORAGE_NODES)
            .disperse(bytes)
            .unwrap();

        Self::new(leaf.block_header().clone(), disperse.common)
    }

    pub fn block_hash(&self) -> BlockHash<Types> {
        self.block_hash
    }

    pub fn payload_hash(&self) -> ADVZCommitment {
        self.payload_hash
    }

    pub fn common(&self) -> &ADVZCommon {
        &self.common
    }
}

impl<Types: NodeType> HeightIndexed for ADVZCommonQueryData<Types> {
    fn height(&self) -> u64 {
        self.height
    }
}

impl<Types: NodeType> HeightIndexed for (ADVZCommonQueryData<Types>, Option<VidShare>) {
    fn height(&self) -> u64 {
        self.0.height
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct VidCommonQueryData<Types: NodeType> {
    pub(crate) height: u64,
    pub(crate) block_hash: BlockHash<Types>,
    pub(crate) payload_hash: VidCommitment,
    pub(crate) common: VidCommon,
}

impl<Types: NodeType> VidCommonQueryData<Types> {
    pub fn new(header: Header<Types>, common: VidCommon) -> Self {
        Self {
            height: header.block_number(),
            block_hash: header.commit(),
            payload_hash: header.payload_commitment(),
            common,
        }
    }

    pub async fn genesis<HsVer: Versions>(
        validated_state: &Types::ValidatedState,
        instance_state: &Types::InstanceState,
    ) -> Self {
        let leaf = Leaf::<Types>::genesis::<HsVer>(validated_state, instance_state).await;
        let payload = leaf.block_payload().unwrap();
        let bytes = payload.encode();
        let disperse = advz_scheme(GENESIS_VID_NUM_STORAGE_NODES)
            .disperse(bytes)
            .unwrap();

        Self::new(leaf.block_header().clone(), VidCommon::V0(disperse.common))
    }

    pub fn block_hash(&self) -> BlockHash<Types> {
        self.block_hash
    }

    pub fn payload_hash(&self) -> VidCommitment {
        self.payload_hash
    }

    pub fn common(&self) -> &VidCommon {
        &self.common
    }
}

impl<Types: NodeType> HeightIndexed for VidCommonQueryData<Types> {
    fn height(&self) -> u64 {
        self.height
    }
}

impl<Types: NodeType> HeightIndexed for (VidCommonQueryData<Types>, Option<VidShare>) {
    fn height(&self) -> u64 {
        self.0.height
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct TransactionQueryData<Types: NodeType>
where
    Payload<Types>: QueryablePayload<Types>,
{
    transaction: Transaction<Types>,
    hash: TransactionHash<Types>,
    index: u64,
    proof: TransactionInclusionProof<Types>,
    block_hash: BlockHash<Types>,
    block_height: u64,
}

impl<Types: NodeType> TransactionQueryData<Types>
where
    Payload<Types>: QueryablePayload<Types>,
{
    pub(crate) fn new(
        block: &BlockQueryData<Types>,
        i: TransactionIndex<Types>,
        index: u64,
    ) -> Option<Self> {
        let (transaction, proof) = block
            .payload()
            .transaction_with_proof(block.metadata(), &i)?;
        Some(Self {
            hash: transaction.commit(),
            transaction,
            index,
            proof,
            block_hash: block.hash(),
            block_height: block.height(),
        })
    }

    pub(crate) fn with_hash(
        block: &BlockQueryData<Types>,
        hash: TransactionHash<Types>,
    ) -> Option<Self> {
        block
            .enumerate()
            .enumerate()
            .find_map(|(i, (index, tx))| {
                if tx.commit() == hash {
                    Some(Self::new(block, index, i as u64))
                } else {
                    None
                }
            })
            .flatten()
    }

    /// The underlying transaction data.
    pub fn transaction(&self) -> &Transaction<Types> {
        &self.transaction
    }

    /// The hash of this transaction.
    pub fn hash(&self) -> TransactionHash<Types> {
        self.hash
    }

    /// The (0-based) position of this transaction within its block.
    pub fn index(&self) -> u64 {
        self.index
    }

    /// A proof of inclusion of this transaction in its block.
    pub fn proof(&self) -> &TransactionInclusionProof<Types> {
        &self.proof
    }

    /// The height of the block containing this transaction.
    pub fn block_height(&self) -> u64 {
        self.block_height
    }

    /// The hash of the block containing this transaction.
    pub fn block_hash(&self) -> BlockHash<Types> {
        self.block_hash
    }
}

pub(crate) fn payload_size<Types: NodeType>(payload: &Payload<Types>) -> u64 {
    payload.encode().len() as u64
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct BlockSummaryQueryData<Types: NodeType> {
    pub(crate) header: Header<Types>,
    pub(crate) hash: BlockHash<Types>,
    pub(crate) size: u64,
    pub(crate) num_transactions: u64,
}

// Add some basic getters to the BlockSummaryQueryData type.
impl<Types: NodeType> BlockSummaryQueryData<Types> {
    pub fn header(&self) -> &Header<Types> {
        &self.header
    }

    pub fn hash(&self) -> BlockHash<Types> {
        self.hash
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn num_transactions(&self) -> u64 {
        self.num_transactions
    }
}

impl<Types: NodeType> HeightIndexed for BlockSummaryQueryData<Types> {
    fn height(&self) -> u64 {
        self.header.block_number()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(bound = "")]
pub struct TransactionSummaryQueryData<Types: NodeType> {
    pub(crate) hash: TransactionHash<Types>,
    pub(crate) header: Header<Types>,
    // We want a way to determine a summary for each rollup entry, without
    // the data directly, but rather a summary of the data.
    // For now, we'll roll with the `Payload` itself.
    pub(crate) transaction: Transaction<Types>,
}

// Since BlockSummaryQueryData can be derived entirely from BlockQueryData, we
// implement the From trait to allow for a seamless conversion using rust
// contentions.
impl<Types: NodeType> From<BlockQueryData<Types>> for BlockSummaryQueryData<Types>
where
    Payload<Types>: QueryablePayload<Types>,
{
    fn from(value: BlockQueryData<Types>) -> Self {
        BlockSummaryQueryData {
            header: value.header,
            hash: value.hash,
            size: value.size,
            num_transactions: value.num_transactions,
        }
    }
}

/// A summary of a payload without all the data.
///
/// This type is useful when you only want information about a payload, such as its size or
/// transaction count, but you don't want to load the entire payload, which might be very large.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadMetadata<Types>
where
    Types: NodeType,
{
    pub height: u64,
    pub block_hash: BlockHash<Types>,
    pub hash: VidCommitment,
    pub size: u64,
    pub num_transactions: u64,
}

impl<Types> HeightIndexed for PayloadMetadata<Types>
where
    Types: NodeType,
{
    fn height(&self) -> u64 {
        self.height
    }
}

impl<Types> From<BlockQueryData<Types>> for PayloadMetadata<Types>
where
    Types: NodeType,
{
    fn from(block: BlockQueryData<Types>) -> Self {
        Self {
            height: block.height(),
            block_hash: block.hash(),
            hash: block.payload_hash(),
            size: block.size(),
            num_transactions: block.num_transactions(),
        }
    }
}

/// A summary of a VID payload without all the data.
///
/// This is primarily useful when you want to check if a VID object exists, but not load the whole
/// object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VidCommonMetadata<Types>
where
    Types: NodeType,
{
    pub height: u64,
    pub block_hash: BlockHash<Types>,
    pub payload_hash: VidCommitment,
}

impl<Types> HeightIndexed for VidCommonMetadata<Types>
where
    Types: NodeType,
{
    fn height(&self) -> u64 {
        self.height
    }
}

impl<Types> From<VidCommonQueryData<Types>> for VidCommonMetadata<Types>
where
    Types: NodeType,
{
    fn from(common: VidCommonQueryData<Types>) -> Self {
        Self {
            height: common.height(),
            block_hash: common.block_hash(),
            payload_hash: common.payload_hash(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, From)]
#[serde(bound = "")]
pub struct StateCertQueryData<Types: NodeType>(pub LightClientStateUpdateCertificate<Types>);

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Limits {
    pub small_object_range_limit: usize,
    pub large_object_range_limit: usize,
}

impl<Types: NodeType> HeightIndexed for StateCertQueryData<Types> {
    fn height(&self) -> u64 {
        self.0.light_client_state.block_height
    }
}
