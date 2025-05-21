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

use std::ops::Range;

use hotshot::traits::{
    election::static_committee::StaticCommittee, implementations::MemoryNetwork, NodeImplementation,
};
use hotshot_example_types::{
    block_types::{TestBlockHeader, TestBlockPayload, TestTransaction},
    state_types::{TestInstanceState, TestValidatedState},
    storage_types::TestStorage,
};
use hotshot_types::{
    data::{QuorumProposal, ViewNumber},
    signature_key::{BLSPubKey, SchnorrPubKey},
    traits::node_implementation::{NodeType, Versions},
};
use jf_merkle_tree::{
    prelude::{MerkleProof, Sha3Digest, Sha3Node},
    universal_merkle_tree::UniversalMerkleTree,
    ForgetableMerkleTreeScheme, ForgetableUniversalMerkleTreeScheme,
};
use serde::{Deserialize, Serialize};
use vbs::version::StaticVersion;

use crate::{
    availability::{QueryableHeader, QueryablePayload},
    explorer::traits::{ExplorerHeader, ExplorerTransaction},
    merklized_state::MerklizedState,
    types::HeightIndexed,
};

pub type MockHeader = TestBlockHeader;
pub type MockPayload = TestBlockPayload;
pub type MockTransaction = TestTransaction;

pub fn mock_transaction(payload: Vec<u8>) -> MockTransaction {
    TestTransaction::new(payload)
}

impl QueryableHeader<MockTypes> for MockHeader {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

impl ExplorerHeader<MockTypes> for MockHeader {
    type BalanceAmount = i128;
    type WalletAddress = [u8; 32];
    type ProposerId = [u8; 32];
    type NamespaceId = u64;

    fn proposer_id(&self) -> Self::ProposerId {
        [0; 32]
    }

    fn fee_info_account(&self) -> Self::WalletAddress {
        [0; 32]
    }

    fn fee_info_balance(&self) -> Self::BalanceAmount {
        0
    }

    fn reward_balance(&self) -> Self::BalanceAmount {
        0
    }

    fn namespace_ids(&self) -> Vec<Self::NamespaceId> {
        vec![0]
    }
}

impl ExplorerTransaction for MockTransaction {
    type NamespaceId = u64;

    fn namespace_id(&self) -> Self::NamespaceId {
        0
    }

    fn payload_size(&self) -> u64 {
        self.bytes().len() as u64
    }
}

impl HeightIndexed for MockHeader {
    fn height(&self) -> u64 {
        self.block_number
    }
}

impl<Types: NodeType> QueryablePayload<Types> for MockPayload {
    type TransactionIndex = usize;
    type Iter<'a> = Range<usize>;
    type InclusionProof = ();

    fn len(&self, _meta: &Self::Metadata) -> usize {
        self.transactions.len()
    }

    fn iter(&self, meta: &Self::Metadata) -> Self::Iter<'_> {
        0..<TestBlockPayload as QueryablePayload<Types>>::len(self, meta)
    }

    fn transaction_with_proof(
        &self,
        _meta: &Self::Metadata,
        index: &Self::TransactionIndex,
    ) -> Option<(Self::Transaction, Self::InclusionProof)> {
        self.transactions.get(*index).cloned().map(|tx| (tx, ()))
    }
}

#[derive(
    Copy, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MockTypes;

impl NodeType for MockTypes {
    type View = ViewNumber;
    type Epoch = ViewNumber;
    type BlockHeader = MockHeader;
    type BlockPayload = MockPayload;
    type SignatureKey = BLSPubKey;
    type Transaction = MockTransaction;
    type InstanceState = TestInstanceState;
    type ValidatedState = TestValidatedState;
    type Membership = StaticCommittee<Self>;
    type BuilderSignatureKey = BLSPubKey;
    type StateSignatureKey = SchnorrPubKey;
}

#[derive(Clone, Debug, Copy)]
pub struct MockVersions {}

impl Versions for MockVersions {
    type Base = StaticVersion<0, 1>;
    type Upgrade = StaticVersion<0, 2>;
    const UPGRADE_HASH: [u8; 32] = [
        1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0,
        0, 0,
    ];
    type Epochs = StaticVersion<0, 4>;
}

/// A type alias for the mock base version
pub type MockBase = <MockVersions as Versions>::Base;

pub type MockMembership = StaticCommittee<MockTypes>;
pub type MockQuorumProposal = QuorumProposal<MockTypes>;
pub type MockNetwork = MemoryNetwork<BLSPubKey>;

pub type MockStorage = TestStorage<MockTypes>;

#[derive(
    Copy, Clone, Debug, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct MockNodeImpl;

impl NodeImplementation<MockTypes> for MockNodeImpl {
    type Network = MockNetwork;
    type Storage = MockStorage;
}

pub type MockMerkleTree = UniversalMerkleTree<usize, Sha3Digest, usize, 8, Sha3Node>;

impl MerklizedState<MockTypes, 8> for MockMerkleTree {
    type Key = usize;
    type Entry = usize;
    type T = Sha3Node;
    type Commit = Self::Commitment;
    type Digest = Sha3Digest;

    fn state_type() -> &'static str {
        "test_tree"
    }

    fn header_state_commitment_field() -> &'static str {
        "test_merkle_tree_root"
    }

    fn tree_height() -> usize {
        12
    }

    fn insert_path(
        &mut self,
        key: Self::Key,
        proof: &MerkleProof<Self::Entry, Self::Key, Self::T, 8>,
    ) -> anyhow::Result<()> {
        match proof.elem() {
            Some(elem) => self.remember(key, elem, proof)?,
            None => self.non_membership_remember(key, proof)?,
        }
        Ok(())
    }
}
