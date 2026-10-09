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

//! A synthetic chain of decided new-protocol blocks, for tests that need stored data but not live
//! consensus.

use std::{marker::PhantomData, ops::RangeInclusive, sync::Arc};

use committable::Committable;
use hotshot_types::{
    data::{
        EpochNumber, Leaf2, QuorumProposal2, QuorumProposalWrapper, VidCommitment, VidCommon,
        VidDisperseShare, VidShare, ViewNumber, ns_table::parse_ns_table,
        vid_disperse::AvidmGf2DisperseShare,
    },
    event::LeafInfo,
    new_protocol::CoordinatorEvent,
    signature_key::BLSPubKey,
    simple_certificate::{Certificate2, QuorumCertificate2},
    simple_vote::{QuorumData2, Vote2Data},
    traits::{
        BlockPayload,
        block_contents::{EncodeBytes, GENESIS_VID_NUM_STORAGE_NODES},
        signature_key::SignatureKey,
    },
    utils::epoch_from_block_number,
    vid::avidm_gf2::{AvidmGf2Scheme, init_avidm_gf2_param},
};
use versions::NEW_PROTOCOL_VERSION;

use super::{
    consensus::DataSourceLifeCycle,
    mocks::{MockHeader, MockPayload, MockTransaction, MockTypes},
};
use crate::availability::{
    BlockInfo, BlockQueryData, LeafQueryData, UpdateAvailabilityData, VidCommonQueryData,
};

/// One decided block with everything a data source stores for it.
#[derive(Clone, Debug)]
pub struct MockBlock {
    pub leaf: LeafQueryData<MockTypes>,
    pub block: BlockQueryData<MockTypes>,
    pub vid_common: VidCommonQueryData<MockTypes>,
    /// One share per storage node.
    pub vid_shares: Vec<VidShare>,
}

impl MockBlock {
    pub fn height(&self) -> u64 {
        self.leaf.leaf().block_header().block_number
    }

    /// Everything node `node` would append for this block on decide.
    pub fn block_info(&self, node: usize) -> BlockInfo<MockTypes> {
        BlockInfo::new(
            self.leaf.clone(),
            Some(self.block.clone()),
            Some(self.vid_common.clone()),
            self.vid_shares.get(node).cloned(),
        )
    }
}

/// A chain of decided blocks at [`NEW_PROTOCOL_VERSION`], starting from genesis.
///
/// Each leaf's `justify_qc` certifies its parent, and each [`LeafQueryData`] carries a QC
/// certifying the leaf itself, so the chain is consistent the way a real decide is. QC signatures
/// are not real; the query service does not check them.
#[derive(Clone, Debug)]
pub struct MockChain {
    blocks: Vec<MockBlock>,
    num_nodes: usize,
    epoch_height: u64,
}

impl MockChain {
    pub async fn new(num_nodes: usize, epoch_height: u64) -> Self {
        Self {
            blocks: vec![genesis(epoch_height).await],
            num_nodes,
            epoch_height,
        }
    }

    pub fn blocks(&self) -> &[MockBlock] {
        &self.blocks
    }

    pub fn tip(&self) -> &MockBlock {
        self.blocks.last().expect("chain always has genesis")
    }

    /// Extend the chain by one block containing `txs`, one second after the tip.
    pub async fn push(
        &mut self,
        txs: impl IntoIterator<Item = MockTransaction> + Send,
    ) -> &MockBlock {
        let timestamp = self.tip().leaf.leaf().block_header().timestamp + 1;
        self.push_at(txs, timestamp).await
    }

    /// Extend the chain by one block containing `txs`, with header timestamp `timestamp` seconds.
    pub async fn push_at(
        &mut self,
        txs: impl IntoIterator<Item = MockTransaction> + Send,
        timestamp: u64,
    ) -> &MockBlock {
        let (payload, metadata) = <MockPayload as BlockPayload<MockTypes>>::from_transactions(
            txs,
            &Default::default(),
            &Default::default(),
        )
        .await
        .unwrap();
        let (payload_commitment, vid_common, vid_shares) =
            disperse(&payload, &metadata, self.num_nodes);

        let parent = self.tip().leaf.leaf();
        let height = parent.block_header().block_number + 1;
        let header = MockHeader {
            block_number: height,
            payload_commitment,
            builder_commitment: <MockPayload as BlockPayload<MockTypes>>::builder_commitment(
                &payload, &metadata,
            ),
            metadata,
            timestamp,
            timestamp_millis: timestamp * 1000,
            random: 0,
            version: NEW_PROTOCOL_VERSION,
        };
        let leaf = Leaf2::from_quorum_proposal(&QuorumProposalWrapper {
            proposal: QuorumProposal2 {
                block_header: header.clone(),
                view_number: parent.view_number() + 1,
                epoch: Some(self.epoch(height)),
                justify_qc: self.qc_for(parent),
                next_epoch_justify_qc: None,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                state_cert: None,
            },
        });
        let qc = self.qc_for(&leaf);

        self.blocks.push(MockBlock {
            leaf: LeafQueryData::new(leaf, qc).unwrap(),
            block: BlockQueryData::new(header.clone(), payload),
            vid_common: VidCommonQueryData::new(header, vid_common),
            vid_shares,
        });
        self.tip()
    }

    /// Extend the chain by `n` empty blocks.
    pub async fn push_empty(&mut self, n: usize) {
        for _ in 0..n {
            self.push([]).await;
        }
    }

    /// The event consensus sends node `node` when the blocks at `heights` are decided together.
    ///
    /// As the coordinator sends it, leaves are newest first and carry their payloads, `cert1`
    /// certifies the newest leaf and `cert2` finalizes it. Genesis is the exception: consensus
    /// neither disperses it nor finalizes it with a cert2, so it carries no VID share and its
    /// decide has no cert2.
    pub fn decide_event(
        &self,
        heights: RangeInclusive<usize>,
        node: usize,
    ) -> CoordinatorEvent<MockTypes> {
        let blocks = &self.blocks[heights];
        let newest = blocks.last().expect("decide at least one block");
        let leaf_infos = blocks
            .iter()
            .rev()
            .map(|block| {
                let vid_share = (block.height() > 0).then(|| self.vid_disperse_share(block, node));
                let mut leaf = block.leaf.leaf().clone();
                leaf.fill_block_payload_unchecked(block.block.payload().clone());
                LeafInfo::new(leaf, Arc::new(Default::default()), None, vid_share, None)
            })
            .collect();
        CoordinatorEvent::NewDecide {
            leaf_infos,
            cert1: newest.leaf.qc().clone(),
            cert2: self.cert2(newest),
        }
    }

    /// The cert2 finalizing `block`, or `None` for genesis, which consensus decides without one.
    pub fn cert2(&self, block: &MockBlock) -> Option<Certificate2<MockTypes>> {
        (block.height() > 0).then(|| self.cert2_for(block.leaf.leaf()))
    }

    fn vid_disperse_share(&self, block: &MockBlock, node: usize) -> VidDisperseShare<MockTypes> {
        let (VidCommitment::V2(payload_commitment), VidCommon::V2(common), VidShare::V2(share)) = (
            block.leaf.payload_hash(),
            block.vid_common.common().clone(),
            block.vid_shares[node].clone(),
        ) else {
            panic!("chain disperses with AvidmGf2");
        };
        let epoch = Some(self.epoch(block.height()));
        VidDisperseShare::V2(AvidmGf2DisperseShare {
            view_number: block.leaf.leaf().view_number(),
            epoch,
            target_epoch: epoch,
            payload_commitment,
            share,
            recipient_key: BLSPubKey::generated_from_seed_indexed([0; 32], node as u64).0,
            common,
        })
    }

    fn cert2_for(&self, leaf: &Leaf2<MockTypes>) -> Certificate2<MockTypes> {
        let height = leaf.block_header().block_number;
        let data = Vote2Data {
            leaf_commit: leaf.commit(),
            epoch: self.epoch(height),
            block_number: height,
        };
        let commit = data.commit();
        Certificate2::new(data, commit, leaf.view_number(), None, PhantomData)
    }

    fn epoch(&self, height: u64) -> EpochNumber {
        EpochNumber::new(epoch_from_block_number(height, self.epoch_height))
    }

    fn qc_for(&self, leaf: &Leaf2<MockTypes>) -> QuorumCertificate2<MockTypes> {
        let height = leaf.block_header().block_number;
        let data = QuorumData2 {
            leaf_commit: leaf.commit(),
            epoch: Some(self.epoch(height)),
            block_number: Some(height),
        };
        QuorumCertificate2::new(data, data.commit(), leaf.view_number(), None, PhantomData)
    }
}

/// Storage nodes the chain disperses VID to.
pub const NUM_NODES: usize = 2;

pub const EPOCH_HEIGHT: u64 = 10;

/// A data source following a [`MockChain`]: each block pushed onto the chain is decided and
/// appended to the data source, as a node following consensus would.
pub struct ChainNode<D: DataSourceLifeCycle> {
    chain: MockChain,
    data_source: D,
    storage: D::Storage,
}

impl<D: DataSourceLifeCycle + UpdateAvailabilityData<MockTypes>> ChainNode<D> {
    /// A node that has decided genesis.
    pub async fn new() -> Self {
        let storage = D::create(0).await;
        let data_source = D::connect(&storage).await;
        Self::with_data_source(storage, data_source).await
    }

    /// A node with a leaf-only data source that has decided genesis.
    pub async fn leaf_only() -> Self {
        let storage = D::create(0).await;
        let data_source = D::leaf_only_ds(&storage).await;
        Self::with_data_source(storage, data_source).await
    }

    pub fn data_source(&self) -> D {
        self.data_source.clone()
    }

    pub fn storage(&self) -> &D::Storage {
        &self.storage
    }

    pub fn chain(&self) -> &MockChain {
        &self.chain
    }

    /// Decide one block containing `txs`, one second after the tip.
    pub async fn push(
        &mut self,
        txs: impl IntoIterator<Item = MockTransaction> + Send,
    ) -> MockBlock {
        let block = self.chain.push(txs).await.clone();
        self.decide(&block).await;
        block
    }

    /// Decide one block containing `txs`, with header timestamp `timestamp` seconds.
    pub async fn push_at(
        &mut self,
        txs: impl IntoIterator<Item = MockTransaction> + Send,
        timestamp: u64,
    ) -> MockBlock {
        let block = self.chain.push_at(txs, timestamp).await.clone();
        self.decide(&block).await;
        block
    }

    /// Decide `n` empty blocks.
    pub async fn push_empty(&mut self, n: usize) {
        for _ in 0..n {
            self.push([]).await;
        }
    }

    async fn with_data_source(storage: D::Storage, data_source: D) -> Self {
        let mut node = Self {
            chain: MockChain::new(NUM_NODES, EPOCH_HEIGHT).await,
            data_source,
            storage,
        };
        let genesis = node.chain.tip().clone();
        node.decide(&genesis).await;
        node
    }

    async fn decide(&mut self, block: &MockBlock) {
        self.data_source.append(block.block_info(0)).await.unwrap();
    }
}

async fn genesis(epoch_height: u64) -> MockBlock {
    let leaf = Leaf2::<MockTypes>::genesis(
        &Default::default(),
        &Default::default(),
        NEW_PROTOCOL_VERSION,
    )
    .await;
    let header = leaf.block_header().clone();
    let payload = MockPayload::genesis();

    // The genesis header commits to VID over `GENESIS_VID_NUM_STORAGE_NODES`, so disperse to
    // exactly that many nodes for the commitment to match.
    let (payload_commitment, vid_common, vid_shares) =
        disperse(&payload, &header.metadata, GENESIS_VID_NUM_STORAGE_NODES);
    assert_eq!(payload_commitment, header.payload_commitment);

    let data = QuorumData2 {
        leaf_commit: leaf.commit(),
        epoch: Some(EpochNumber::new(epoch_from_block_number(0, epoch_height))),
        block_number: Some(0),
    };
    let qc = QuorumCertificate2::new(
        data,
        data.commit(),
        ViewNumber::genesis(),
        None,
        PhantomData,
    );

    MockBlock {
        leaf: LeafQueryData::new(leaf, qc).unwrap(),
        block: BlockQueryData::new(header.clone(), payload),
        vid_common: VidCommonQueryData::new(header, vid_common),
        vid_shares,
    }
}

/// AvidmGf2 dispersal of `payload` over `num_nodes` equally weighted nodes.
fn disperse(
    payload: &MockPayload,
    metadata: &<MockPayload as BlockPayload<MockTypes>>::Metadata,
    num_nodes: usize,
) -> (VidCommitment, VidCommon, Vec<VidShare>) {
    let bytes = payload.encode();
    let param = init_avidm_gf2_param(num_nodes).unwrap();
    let ns_table = parse_ns_table(bytes.len(), &metadata.encode());
    let (commit, common, shares) =
        AvidmGf2Scheme::ns_disperse(&param, &vec![1; num_nodes], &bytes, ns_table).unwrap();
    (
        VidCommitment::V2(commit),
        VidCommon::V2(common),
        shares.into_iter().map(VidShare::V2).collect(),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{
        availability::{AvailabilityDataSource, UpdateAvailabilityData},
        node::NodeDataSource,
        testing::{
            consensus::{DataSourceLifeCycle, MockSqlDataSource},
            mocks::mock_transaction,
        },
    };

    #[tokio::test]
    async fn test_chain_is_linked() {
        let mut chain = MockChain::new(3, 10).await;
        chain.push_empty(2).await;
        chain.push([mock_transaction(vec![1, 2, 3])]).await;
        chain.push_at([], 100).await;

        for (parent, child) in chain.blocks().iter().zip(&chain.blocks()[1..]) {
            assert_eq!(child.height(), parent.height() + 1);
            assert_eq!(
                child.leaf.leaf().parent_commitment(),
                parent.leaf.leaf().commit()
            );
            assert_eq!(child.vid_shares.len(), 3);
        }
        assert_eq!(chain.blocks()[3].block.num_transactions(), 1);
        assert_eq!(chain.tip().leaf.header().timestamp, 100);
    }

    #[tokio::test]
    async fn test_chain_round_trips_through_storage() {
        let storage = MockSqlDataSource::create(0).await;
        let ds = <MockSqlDataSource as DataSourceLifeCycle>::connect(&storage).await;

        let mut chain = MockChain::new(2, 10).await;
        chain.push([mock_transaction(vec![1, 2, 3])]).await;
        chain.push_empty(1).await;
        for block in chain.blocks() {
            ds.append(block.block_info(1)).await.unwrap();
        }

        for block in chain.blocks() {
            let height = block.height() as usize;
            assert_eq!(ds.get_leaf(height).await.await, block.leaf);
            assert_eq!(ds.get_block(height).await.await, block.block);
            assert_eq!(ds.get_vid_common(height).await.await, block.vid_common);
        }
        assert_eq!(chain.blocks()[1].block.num_transactions(), 1);
        assert_eq!(
            ds.vid_share(1).await.unwrap(),
            chain.blocks()[1].vid_shares[1]
        );
    }
}
