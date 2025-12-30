#![cfg(any(test, feature = "testing"))]

use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy::primitives::{Address, U256};
use anyhow::{bail, ensure, Context, Result};
use async_lock::Mutex;
use bitvec::vec::BitVec;
use committable::{Commitment, Committable};
use derivative::Derivative;
use espresso_types::{
    v0_3::StakeTableEvent, BlockMerkleTree, EpochVersion, FeeVersion, Leaf2, NodeState, PrivKey,
    PubKey, SeqTypes, SequencerVersions, BLOCK_MERKLE_TREE_HEIGHT,
};
use hotshot_contract_adapter::sol_types::StakeTableV2::{Delegated, ValidatorRegistered};
use hotshot_query_service::{
    availability::{LeafHash, LeafId, LeafQueryData},
    node::{BlockHash, BlockId},
};
use hotshot_types::{
    data::{EpochNumber, QuorumProposal2, QuorumProposalWrapper, VidCommitment, ViewNumber},
    light_client::StateVerKey,
    message::UpgradeLock,
    simple_certificate::{NextEpochQuorumCertificate2, QuorumCertificate2, UpgradeCertificate},
    simple_vote::{NextEpochQuorumData2, QuorumData2, UpgradeProposalData, VersionedVoteData},
    stake_table::{supermajority_threshold, StakeTableEntry},
    traits::{
        node_implementation::{ConsensusTime, Versions},
        signature_key::{SignatureKey, StakeTableEntryType, StateSignatureKey},
    },
    utils::{epoch_from_block_number, is_epoch_transition},
    vote::Certificate as _,
};
use jf_merkle_tree_compat::{
    prelude::SHA3MerkleTree, AppendableMerkleTreeScheme, MerkleTreeScheme,
};
use vbs::version::StaticVersionType;

use crate::{
    client::Client,
    consensus::{
        header::HeaderProof,
        leaf::LeafProof,
        quorum::{Certificate, Quorum},
    },
    state::Genesis,
    storage::LeafRequest,
};

/// Upgrade to epochs during testing.
pub type EnableEpochs = SequencerVersions<LegacyVersion, EpochVersion>;

/// Test without epochs and with legacy HotStuff.
pub type LegacyVersion = FeeVersion;

/// Extract a chain of QCs from a chain of leaves.
///
/// The resulting QC chain will be one shorter than the leaf chain, and will justify the finality of
/// the leaf _preceding_ this leaf chain, since we extract QCs from the justifying QC of each leaf.
pub fn qc_chain_from_leaf_chain<'a>(
    leaves: impl IntoIterator<Item = &'a LeafQueryData<SeqTypes>>,
) -> Vec<Certificate> {
    leaves
        .into_iter()
        .map(|leaf| Certificate::for_parent(leaf.leaf()))
        .collect()
}

/// Construct a valid leaf chain for the given height range.
pub async fn leaf_chain<V: StaticVersionType + 'static>(
    range: impl IntoIterator<Item = u64>,
) -> Vec<LeafQueryData<SeqTypes>> {
    custom_leaf_chain::<SequencerVersions<V, V>>(range, |_| {}).await
}

/// Construct a valid leaf chain for the given height range.
///
/// The chain will upgrade from `V::Base` to `V::Upgrade` at height `upgrade_height`.
pub async fn leaf_chain_with_upgrade<V: Versions>(
    range: impl IntoIterator<Item = u64>,
    upgrade_height: u64,
) -> Vec<LeafQueryData<SeqTypes>> {
    custom_leaf_chain_with_upgrade::<V>(range, upgrade_height, |_| {}).await
}

/// Construct a customized leaf chain for the given height range.
///
/// The chain will upgrade from `V::Base` to `V::Upgrade` at height `upgrade_height`.
pub async fn custom_leaf_chain_with_upgrade<V: Versions>(
    range: impl IntoIterator<Item = u64>,
    upgrade_height: u64,
    map: impl Fn(&mut QuorumProposal2<SeqTypes>),
) -> Vec<LeafQueryData<SeqTypes>> {
    let upgrade_leaf: Leaf2 = Leaf2::genesis::<V>(
        &Default::default(),
        &NodeState::mock()
            .with_genesis_version(V::Upgrade::version())
            .with_current_version(V::Upgrade::version()),
    )
    .await;
    let upgrade_data = UpgradeProposalData {
        old_version: V::Base::version(),
        new_version: V::Upgrade::version(),
        new_version_hash: Default::default(),
        old_version_last_view: ViewNumber::new(upgrade_height - 1),
        new_version_first_view: ViewNumber::new(upgrade_height),
        decide_by: ViewNumber::new(upgrade_height),
    };
    let upgrade_commit = upgrade_data.commit();
    let upgrade_cert = UpgradeCertificate::new(
        upgrade_data,
        upgrade_commit,
        ViewNumber::new(upgrade_height),
        Default::default(),
        Default::default(),
    );

    custom_leaf_chain::<V>(range, |proposal| {
        let height = proposal.block_header.height();
        if height < upgrade_height {
            // All views leading up to the upgrade get a certificate indicating the coming upgrade.
            proposal.upgrade_certificate = Some(upgrade_cert.clone());
        } else {
            // After the upgrade takes effect we stop attaching the upgrade certificate, and we use
            // the upgraded header version.
            proposal.upgrade_certificate = None;
            proposal.block_header = upgrade_leaf.block_header().clone();
            *proposal.block_header.height_mut() = height;
        }
        map(proposal);
    })
    .await
}

/// Construct a customized leaf chain for the given height range.
pub async fn custom_leaf_chain<V: Versions>(
    range: impl IntoIterator<Item = u64>,
    map: impl Fn(&mut QuorumProposal2<SeqTypes>),
) -> Vec<LeafQueryData<SeqTypes>> {
    let node_state = NodeState::mock()
        .with_genesis_version(V::Base::version())
        .with_current_version(V::Base::version());
    let genesis_leaf: Leaf2 = Leaf2::genesis::<V>(&Default::default(), &node_state).await;
    tracing::info!(?genesis_leaf, "leaf chain");

    let mut qc = QuorumCertificate2::genesis::<V>(&Default::default(), &node_state).await;
    let mut quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
        proposal: QuorumProposal2::<SeqTypes> {
            epoch: None,
            block_header: genesis_leaf.block_header().clone(),
            view_number: genesis_leaf.view_number(),
            justify_qc: qc.clone(),
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: None,
            next_epoch_justify_qc: None,
            state_cert: None,
        },
    };

    let mut block_merkle_tree = BlockMerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT);
    let mut leaves = vec![];
    for height in range {
        *quorum_proposal.proposal.block_header.height_mut() = height;
        *quorum_proposal
            .proposal
            .block_header
            .block_merkle_tree_root_mut() = block_merkle_tree.commitment();
        quorum_proposal.proposal.view_number = ViewNumber::new(height);
        map(&mut quorum_proposal.proposal);
        let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);

        qc.view_number = ViewNumber::new(height);
        qc.data.leaf_commit = Committable::commit(&leaf);
        if leaf.block_header().version() >= EpochVersion::version() {
            qc.data.block_number = Some(height);
        }

        block_merkle_tree
            .push(leaf.block_header().commit())
            .unwrap();
        leaves.push(LeafQueryData::new(leaf, qc.clone()).unwrap());
        quorum_proposal.proposal.justify_qc = qc.clone();
    }

    leaves
}

/// Construct a valid leaf chain during which the epoch advances.
pub async fn epoch_change_leaf_chain<V: StaticVersionType + 'static>(
    range: impl IntoIterator<Item = u64>,
    epoch_height: u64,
) -> Vec<LeafQueryData<SeqTypes>> {
    custom_epoch_change_leaf_chain::<V>(range, epoch_height, |_| {}).await
}

/// Construct a customized leaf chain during which the epoch advances.
pub async fn custom_epoch_change_leaf_chain<V: StaticVersionType + 'static>(
    range: impl IntoIterator<Item = u64>,
    epoch_height: u64,
    map: impl Fn(&mut QuorumProposal2<SeqTypes>),
) -> Vec<LeafQueryData<SeqTypes>> {
    custom_leaf_chain::<SequencerVersions<V, V>>(range, |proposal| {
        if is_epoch_transition(proposal.block_header.height(), epoch_height) {
            let data: NextEpochQuorumData2<SeqTypes> = proposal.justify_qc.data.clone().into();
            let commit = data.commit();
            proposal.next_epoch_justify_qc = Some(NextEpochQuorumCertificate2::new(
                data,
                commit,
                proposal.justify_qc.view_number,
                Default::default(),
                Default::default(),
            ));
            map(proposal);
        }
    })
    .await
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AlwaysTrueQuorum;

impl Quorum for AlwaysTrueQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(&self, _: &Certificate) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AlwaysFalseQuorum;

impl Quorum for AlwaysFalseQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(&self, _: &Certificate) -> Result<()> {
        bail!("always false quorum");
    }
}

/// A quorum which verifies that calls to `verify` use the correct version, but does not check
/// signatures.
#[derive(Clone, Debug, Default)]
pub struct VersionCheckQuorum {
    leaves: HashMap<Commitment<Leaf2>, Leaf2>,
}

impl VersionCheckQuorum {
    pub fn new(leaves: impl IntoIterator<Item = Leaf2>) -> Self {
        Self {
            leaves: leaves
                .into_iter()
                .map(|leaf| (leaf.commit(), leaf))
                .collect(),
        }
    }
}

impl Quorum for VersionCheckQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        cert: &Certificate,
    ) -> anyhow::Result<()> {
        let leaf = self
            .leaves
            .get(&cert.leaf_commit())
            .context(format!("unknown leaf {}", cert.leaf_commit()))?;
        ensure!(
            leaf.block_header().version() == V::version(),
            "version mismatch: leaf has version {}, but verifier is using version {}",
            leaf.block_header().version(),
            V::version()
        );
        Ok(())
    }
}

/// A quorum which verifies that epoch change QCs are provided, but does not check signatures.
#[derive(Clone, Debug, Default)]
pub struct EpochChangeQuorum {
    epoch_height: u64,
}

impl EpochChangeQuorum {
    pub fn new(epoch_height: u64) -> Self {
        Self { epoch_height }
    }
}

impl Quorum for EpochChangeQuorum {
    async fn verify_static<V: StaticVersionType + 'static>(
        &self,
        cert: &Certificate,
    ) -> anyhow::Result<()> {
        if V::version() >= EpochVersion::version() {
            cert.verify_next_epoch_qc(self.epoch_height)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct TestClient {
    inner: Arc<Mutex<InnerTestClient>>,
    epoch_height: u64,
}

impl Default for TestClient {
    fn default() -> Self {
        Self {
            inner: Default::default(),
            epoch_height: 100,
        }
    }
}

#[derive(Debug, Derivative)]
#[derivative(Default)]
struct InnerTestClient {
    leaves: Vec<LeafQueryData<SeqTypes>>,
    // Use an appendable `MerkleTree` rather than a `BlockMerkleTree` (which is a
    // `LightweightMerkleTree`) so we can look up paths for previously inserted elements.
    merkle_trees: Vec<SHA3MerkleTree<BlockHash<SeqTypes>>>,
    leaf_hashes: HashMap<LeafHash<SeqTypes>, usize>,
    block_hashes: HashMap<BlockHash<SeqTypes>, usize>,
    payload_hashes: HashMap<VidCommitment, usize>,
    missing_leaves: HashSet<usize>,
    invalid_proofs: HashSet<usize>,
    swapped_leaves: HashMap<usize, usize>,
    quorum: Vec<(PrivKey, StakeTableEntry<PubKey>)>,
    #[derivative(Default(value = "3"))]
    first_epoch_with_dynamic_stake_table: u64,
    missing_quorums: HashSet<u64>,
}

impl InnerTestClient {
    fn quorum_for_epoch(&mut self, epoch: u64) -> &[(PrivKey, StakeTableEntry<PubKey>)] {
        // For testing purposes, we will say that one new node joins the quorum each epoch. The
        // static stake table used before the first epoch with dynamic stake is the same as the
        // stake table used in that epoch.
        let quorum_size = max(epoch, self.first_epoch_with_dynamic_stake_table) as usize;

        while self.quorum.len() < quorum_size {
            let (pub_key, priv_key) =
                PubKey::generated_from_seed_indexed(Default::default(), self.quorum.len() as u64);
            let entry = StakeTableEntry {
                stake_key: pub_key,
                stake_amount: U256::from(self.quorum.len() + 1) * U256::from(1_000_000_000u128),
            };
            self.quorum.push((priv_key, entry));
        }

        &self.quorum[..quorum_size]
    }

    async fn leaf(&mut self, height: usize, epoch_height: u64) -> LeafQueryData<SeqTypes> {
        let epoch = epoch_from_block_number(height as u64, epoch_height);
        let (quorum, stake_table): (Vec<_>, Vec<_>) =
            self.quorum_for_epoch(epoch).iter().cloned().unzip();
        let pp = PubKey::public_parameter(
            stake_table.as_slice(),
            supermajority_threshold(stake_table.iter().map(|entry| entry.stake()).sum()),
        );

        for i in self.leaves.len()..=height {
            let epoch = EpochNumber::new(epoch_from_block_number(i as u64, epoch_height));
            let view_number = ViewNumber::new(i as u64);

            let node_state = NodeState::mock_v3();
            let (justify_qc, mt) =
                if i == 0 {
                    (QuorumCertificate2::genesis::<SequencerVersions<EpochVersion, EpochVersion>>(
                    &Default::default(),
                    &node_state,
                )
                .await, SHA3MerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT))
                } else {
                    let parent = &self.leaves[i - 1];
                    let mut mt = self.merkle_trees[i - 1].clone();
                    mt.push(parent.block_hash()).unwrap();
                    (parent.qc().clone(), mt)
                };
            let mut block_header = Leaf2::genesis::<SequencerVersions<EpochVersion, EpochVersion>>(
                &Default::default(),
                &node_state,
            )
            .await
            .block_header()
            .clone();
            *block_header.height_mut() = i as u64;
            *block_header.block_merkle_tree_root_mut() = mt.commitment();
            let quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
                proposal: QuorumProposal2::<SeqTypes> {
                    epoch: Some(epoch),
                    block_header,
                    view_number,
                    justify_qc,
                    upgrade_certificate: None,
                    view_change_evidence: None,
                    next_drb_result: None,
                    next_epoch_justify_qc: None,
                    state_cert: None,
                },
            };
            let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);
            let quorum_data = QuorumData2 {
                leaf_commit: leaf.commit(),
                epoch: Some(epoch),
                block_number: Some(i as u64),
            };
            let quorum_data_comm = VersionedVoteData::new_infallible(
                quorum_data.clone(),
                view_number,
                &UpgradeLock::<SeqTypes, SequencerVersions<EpochVersion, EpochVersion>>::new(),
            )
            .await
            .commit();
            let signatures = quorum
                .iter()
                .map(|key| PubKey::sign(key, quorum_data_comm.as_ref()).unwrap())
                .collect::<Vec<_>>();
            let assembled = PubKey::assemble(
                &pp,
                &std::iter::repeat_n(true, quorum.len()).collect::<BitVec>(),
                &signatures,
            );
            let qc = QuorumCertificate2::create_signed_certificate(
                quorum_data_comm,
                quorum_data,
                assembled,
                view_number,
            );
            let leaf = LeafQueryData::new(leaf, qc).unwrap();
            self.leaf_hashes.insert(leaf.hash(), i);
            self.block_hashes.insert(leaf.block_hash(), i);
            self.payload_hashes.entry(leaf.payload_hash()).or_insert(i);
            self.leaves.push(leaf);
            self.merkle_trees.push(mt);
        }

        self.leaves[height].clone()
    }

    fn leaf_height(&self, req: LeafRequest) -> Result<usize> {
        match req {
            LeafRequest::Leaf(LeafId::Number(h)) | LeafRequest::Header(BlockId::Number(h)) => Ok(h),
            LeafRequest::Leaf(LeafId::Hash(h)) => self
                .leaf_hashes
                .get(&h)
                .copied()
                .context(format!("missing leaf {h}")),
            LeafRequest::Header(BlockId::Hash(h)) => self
                .block_hashes
                .get(&h)
                .copied()
                .context(format!("missing block {h}")),
            LeafRequest::Header(BlockId::PayloadHash(h)) => self
                .payload_hashes
                .get(&h)
                .copied()
                .context(format!("missing payload {h}")),
        }
    }
}

impl TestClient {
    pub async fn genesis(&self) -> Genesis {
        let mut inner = self.inner.lock().await;
        Genesis {
            epoch_height: self.epoch_height,
            stake_table: inner
                .quorum_for_epoch(1)
                .iter()
                .map(|(_, entry)| entry.clone())
                .collect(),
            first_epoch_with_dynamic_stake_table: EpochNumber::new(
                inner.first_epoch_with_dynamic_stake_table,
            ),
        }
    }

    pub async fn leaf(&self, height: usize) -> LeafQueryData<SeqTypes> {
        let mut inner = self.inner.lock().await;
        inner.leaf(height, self.epoch_height).await
    }

    pub async fn remember_leaf(&self, height: usize) -> LeafQueryData<SeqTypes> {
        let mut inner = self.inner.lock().await;
        inner.missing_leaves.remove(&height);
        inner.invalid_proofs.remove(&height);
        inner.swapped_leaves.remove(&height);
        inner.leaf(height, self.epoch_height).await
    }

    pub async fn forget_leaf(&self, height: usize) -> LeafQueryData<SeqTypes> {
        let mut inner = self.inner.lock().await;
        inner.missing_leaves.insert(height);
        inner.leaf(height, self.epoch_height).await
    }

    pub async fn return_invalid_proof(&self, for_height: usize) {
        let mut inner = self.inner.lock().await;
        inner.invalid_proofs.insert(for_height);
    }

    pub async fn return_wrong_leaf(&self, for_height: usize, substitute: usize) {
        let mut inner = self.inner.lock().await;
        inner.swapped_leaves.insert(for_height, substitute);
    }

    pub async fn quorum_for_epoch(&self, epoch: EpochNumber) -> Vec<StakeTableEntry<PubKey>> {
        let mut inner = self.inner.lock().await;
        inner
            .quorum_for_epoch(*epoch)
            .iter()
            .map(|(_, entry)| entry.clone())
            .collect()
    }

    pub async fn remember_quorum(&self, epoch: EpochNumber) {
        let mut inner = self.inner.lock().await;
        inner.missing_quorums.remove(&*epoch);
    }

    pub async fn forget_quorum(&self, epoch: EpochNumber) {
        let mut inner = self.inner.lock().await;
        inner.missing_quorums.insert(*epoch);
    }
}

impl Client for TestClient {
    async fn leaf_proof(
        &self,
        id: impl Into<LeafRequest> + Send,
        finalized: Option<u64>,
    ) -> Result<LeafProof> {
        let mut inner = self.inner.lock().await;

        let mut height = inner.leaf_height(id.into())?;
        ensure!(
            !inner.missing_leaves.contains(&height),
            "missing leaf {height}"
        );
        if inner.invalid_proofs.contains(&height) {
            tracing::info!(height, "return mock incorrect proof");
            return Ok(LeafProof::default());
        }
        if let Some(sub) = inner.swapped_leaves.get(&height) {
            tracing::info!(height, sub, "return wrong leaf");
            height = *sub;
        };

        let leaf = inner.leaf(height, self.epoch_height).await;

        let mut proof = LeafProof::default();
        proof.push(leaf);
        if let Some(finalized) = finalized {
            ensure!(
                finalized > (height as u64),
                "assumed finalized leaf must be after requested leaf"
            );
            if finalized <= (height as u64) + 2 {
                tracing::info!(
                    height,
                    finalized,
                    "path to finalized is shorter than path to QC-chain, using finalized"
                );
                return Ok(proof);
            }
        }

        proof.push(inner.leaf(height + 1, self.epoch_height).await);
        assert!(proof.push(inner.leaf(height + 2, self.epoch_height).await));

        Ok(proof)
    }

    async fn header_proof(&self, root: u64, id: BlockId<SeqTypes>) -> Result<HeaderProof> {
        let mut inner = self.inner.lock().await;

        let root = root as usize;
        let mut height = inner.leaf_height(id.into())?;
        ensure!(
            !inner.missing_leaves.contains(&height),
            "missing leaf {height}"
        );
        if inner.invalid_proofs.contains(&height) {
            tracing::info!(root, height, "return mock invalid proof");
            let leaf = inner.leaf(height, self.epoch_height).await;
            // Construct a proof using a Merkle tree of the wrong height.
            let mt = BlockMerkleTree::from_elems(
                Some(BLOCK_MERKLE_TREE_HEIGHT + 1),
                [leaf.block_hash()],
            )
            .unwrap();
            let proof = mt.lookup(0).expect_ok().unwrap().1;
            return Ok(HeaderProof::new(leaf.header().clone(), proof));
        }
        if let Some(sub) = inner.swapped_leaves.get(&height) {
            tracing::info!(height, sub, "return wrong leaf");
            height = *sub;
        };

        ensure!(height < root);

        let mt = &inner.merkle_trees[root];
        tracing::info!(height, root = %mt.commitment(), "get proof from Merkle tree");
        let proof = mt.lookup(height as u64).expect_ok().unwrap().1;
        let header = inner.leaf(height, self.epoch_height).await.header().clone();
        Ok(HeaderProof::new(header, proof))
    }

    async fn get_leaves_in_range(
        &self,
        start_height: usize,
        end_height: usize,
    ) -> Result<Vec<LeafQueryData<SeqTypes>>> {
        let mut leaves = Vec::new();
        let mut inner = self.inner.lock().await;
        for h in start_height..end_height {
            let height = *inner.swapped_leaves.get(&h).unwrap_or(&h);
            leaves.push(inner.leaf(height, self.epoch_height).await);
        }
        Ok(leaves)
    }

    async fn stake_table_events(&self, epoch: EpochNumber) -> Result<Vec<StakeTableEvent>> {
        let mut inner = self.inner.lock().await;

        ensure!(
            !inner.missing_quorums.contains(&*epoch),
            "missing quorum for epoch {epoch}"
        );

        let mut events = vec![];
        if *epoch == inner.first_epoch_with_dynamic_stake_table {
            // Generate events such that the first dynamic stake table is the same as the static
            // stake table we started with.
            for (i, (_, entry)) in inner.quorum_for_epoch(*epoch).iter().enumerate() {
                register_validator_events(&mut events, i, entry);
            }
        } else if *epoch > inner.first_epoch_with_dynamic_stake_table {
            // For each subsequent epoch, just generate one event for the new validator which joined
            // in that epoch.
            let (_, entry) = &inner.quorum_for_epoch(*epoch)[(*epoch as usize) - 1];
            register_validator_events(&mut events, *epoch as usize, entry);
        }
        Ok(events)
    }
}

fn register_validator_events(
    events: &mut Vec<StakeTableEvent>,
    i: usize,
    entry: &StakeTableEntry<PubKey>,
) {
    let account = Address::random();
    events.push(StakeTableEvent::Register(ValidatorRegistered {
        account,
        blsVk: entry.stake_key.into(),
        schnorrVk: StateVerKey::generated_from_seed_indexed(Default::default(), i as u64)
            .0
            .into(),
        commission: 1,
    }));
    events.push(StakeTableEvent::Delegate(Delegated {
        delegator: Address::random(),
        validator: account,
        amount: entry.stake_amount,
    }));
}
