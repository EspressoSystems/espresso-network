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
    v0_3::{StakeTableEvent, Validator},
    BlockMerkleTree, DrbAndHeaderUpgradeVersion, EpochVersion, FeeVersion, Leaf2, NodeState,
    Payload, PrivKey, PubKey, SeqTypes, SequencerVersions, StakeTableHash, StakeTableState,
    Transaction, ValidatorMap, BLOCK_MERKLE_TREE_HEIGHT,
};
use hotshot_contract_adapter::sol_types::StakeTableV2::{Delegated, ValidatorRegistered};
use hotshot_query_service::{
    availability::{LeafHash, LeafId, LeafQueryData},
    node::{BlockHash, BlockId},
};
use hotshot_types::{
    data::{
        vid_commitment, EpochNumber, QuorumProposal2, QuorumProposalWrapper, VidCommitment,
        VidCommon, ViewNumber,
    },
    message::UpgradeLock,
    signature_key::SchnorrPubKey,
    simple_certificate::{NextEpochQuorumCertificate2, QuorumCertificate2, UpgradeCertificate},
    simple_vote::{NextEpochQuorumData2, QuorumData2, UpgradeProposalData, VersionedVoteData},
    stake_table::{supermajority_threshold, StakeTableEntry},
    traits::{
        block_contents::EncodeBytes,
        node_implementation::{ConsensusTime, Versions},
        signature_key::{SignatureKey, StateSignatureKey},
    },
    utils::{epoch_from_block_number, is_epoch_transition, is_ge_epoch_root},
    vid::avidm::init_avidm_param,
    vote::Certificate as _,
};
use jf_merkle_tree_compat::{
    prelude::SHA3MerkleTree, AppendableMerkleTreeScheme, MerkleTreeScheme,
};
use rand::RngCore;
use vbs::version::StaticVersionType;

use crate::{
    client::Client,
    consensus::{
        header::HeaderProof,
        leaf::LeafProof,
        payload::PayloadProof,
        quorum::{Certificate, Quorum},
    },
    state::Genesis,
    storage::LeafRequest,
};

/// Upgrade to epochs during testing.
pub type EnableEpochs = SequencerVersions<LegacyVersion, DrbAndHeaderUpgradeVersion>;

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
    payloads: Vec<Payload>,
    // Use an appendable `MerkleTree` rather than a `BlockMerkleTree` (which is a
    // `LightweightMerkleTree`) so we can look up paths for previously inserted elements.
    merkle_trees: Vec<SHA3MerkleTree<BlockHash<SeqTypes>>>,
    leaf_hashes: HashMap<LeafHash<SeqTypes>, usize>,
    block_hashes: HashMap<BlockHash<SeqTypes>, usize>,
    payload_hashes: HashMap<VidCommitment, usize>,
    missing_leaves: HashSet<usize>,
    invalid_proofs: HashSet<usize>,
    swapped_leaves: HashMap<usize, usize>,
    invalid_payloads: HashSet<usize>,
    quorum: Vec<(PrivKey, Validator<PubKey>)>,
    #[derivative(Default(value = "3"))]
    first_epoch_with_dynamic_stake_table: u64,
    missing_quorums: HashSet<u64>,
    invalid_quorums: HashSet<u64>,
}

impl InnerTestClient {
    fn quorum_for_epoch(&mut self, epoch: u64) -> &[(PrivKey, Validator<PubKey>)] {
        // For testing purposes, we will say that one new node joins the quorum each epoch. The
        // static stake table used before the first epoch with dynamic stake is the same as the
        // stake table used in that epoch.
        let quorum_size = max(epoch, self.first_epoch_with_dynamic_stake_table) as usize;

        while self.quorum.len() < quorum_size {
            let (stake_table_key, priv_key) =
                PubKey::generated_from_seed_indexed(Default::default(), self.quorum.len() as u64);
            let (state_ver_key, _) = SchnorrPubKey::generated_from_seed_indexed(
                Default::default(),
                self.quorum.len() as u64,
            );
            let stake = U256::from(self.quorum.len() + 1) * U256::from(1_000_000_000u128);
            let validator = Validator {
                account: Address::random(),
                stake_table_key,
                state_ver_key,
                stake,
                commission: 1,
                delegators: [(Address::random(), stake)].into_iter().collect(),
            };
            self.quorum.push((priv_key, validator));
        }

        &self.quorum[..quorum_size]
    }

    fn stake_table_hash(&mut self, epoch: u64) -> StakeTableHash {
        let quorum = self.quorum_for_epoch(epoch);
        let mut validators = ValidatorMap::default();
        let mut used_bls_keys = HashSet::default();
        let mut used_schnorr_keys = HashSet::default();
        for (_, validator) in quorum {
            validators.insert(validator.account, validator.clone());
            used_bls_keys.insert(validator.stake_table_key);
            used_schnorr_keys.insert(validator.state_ver_key.clone());
        }

        let state = StakeTableState::new(
            validators,
            Default::default(),
            used_bls_keys,
            used_schnorr_keys,
        );
        state.commit()
    }

    async fn leaf(&mut self, height: usize, epoch_height: u64) -> LeafQueryData<SeqTypes> {
        let epoch = epoch_from_block_number(height as u64, epoch_height);
        let (quorum, stake_table): (Vec<_>, Vec<_>) =
            self.quorum_for_epoch(epoch).iter().cloned().unzip();
        let mut stake_entries = vec![];
        let mut total_stake = U256::ZERO;
        for validator in &stake_table {
            stake_entries.push(StakeTableEntry {
                stake_key: validator.stake_table_key,
                stake_amount: validator.stake,
            });
            total_stake += validator.stake;
        }

        let pp = PubKey::public_parameter(&stake_entries, supermajority_threshold(total_stake));

        for i in self.leaves.len()..=height {
            let epoch = EpochNumber::new(epoch_from_block_number(i as u64, epoch_height));
            let view_number = ViewNumber::new(i as u64);

            let version = DrbAndHeaderUpgradeVersion::version();
            let node_state = NodeState::mock_v3().with_genesis_version(version);
            let (justify_qc, mt) = if i == 0 {
                (
                    QuorumCertificate2::genesis::<
                        SequencerVersions<DrbAndHeaderUpgradeVersion, DrbAndHeaderUpgradeVersion>,
                    >(&Default::default(), &node_state)
                    .await,
                    SHA3MerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT),
                )
            } else {
                let parent = &self.leaves[i - 1];
                let mut mt = self.merkle_trees[i - 1].clone();
                mt.push(parent.block_hash()).unwrap();
                (parent.qc().clone(), mt)
            };

            let transactions = vec![Transaction::random(&mut rand::thread_rng())];
            let (payload, ns_table) =
                Payload::from_transactions_sync(transactions, node_state.chain_config).unwrap();
            let payload_comm =
                vid_commitment::<
                    SequencerVersions<DrbAndHeaderUpgradeVersion, DrbAndHeaderUpgradeVersion>,
                >(&payload.encode(), &ns_table.encode(), quorum.len(), version);

            let mut block_header = Leaf2::genesis::<
                SequencerVersions<DrbAndHeaderUpgradeVersion, DrbAndHeaderUpgradeVersion>,
            >(&Default::default(), &node_state)
            .await
            .block_header()
            .clone();
            *block_header.height_mut() = i as u64;
            *block_header.block_merkle_tree_root_mut() = mt.commitment();
            *block_header.payload_commitment_mut() = payload_comm;
            *block_header.ns_table_mut() = ns_table;
            if *epoch + 1 >= self.first_epoch_with_dynamic_stake_table
                && is_ge_epoch_root(block_header.height(), epoch_height)
            {
                assert!(block_header.set_next_stake_table_hash(self.stake_table_hash(*epoch + 1)));
            }
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
                &UpgradeLock::<
                    SeqTypes,
                    SequencerVersions<DrbAndHeaderUpgradeVersion, DrbAndHeaderUpgradeVersion>,
                >::new(),
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
            self.payloads.push(payload);
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
                .map(|(_, validator)| StakeTableEntry {
                    stake_key: validator.stake_table_key,
                    stake_amount: validator.stake,
                })
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

    pub async fn payload(&self, height: usize) -> Payload {
        let mut inner = self.inner.lock().await;
        inner.leaf(height, self.epoch_height).await;
        inner.payloads[height].clone()
    }

    pub async fn return_invalid_payload(&self, for_height: usize) {
        let mut inner = self.inner.lock().await;
        inner.invalid_payloads.insert(for_height);
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
            .map(|(_, validator)| StakeTableEntry {
                stake_key: validator.stake_table_key,
                stake_amount: validator.stake,
            })
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

    pub async fn return_invalid_quorum(&self, epoch: EpochNumber) {
        let mut inner = self.inner.lock().await;
        inner.invalid_quorums.insert(*epoch);
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

        if inner.invalid_quorums.contains(&*epoch) {
            // Return random events to create an invalid stake table.
            let mut events = vec![];
            let mut seed = [0; 32];
            rand::thread_rng().fill_bytes(&mut seed);
            register_validator_events(&mut events, &random_validator());
            return Ok(events);
        }

        let mut events = vec![];
        if *epoch == inner.first_epoch_with_dynamic_stake_table {
            // Generate events such that the first dynamic stake table is the same as the static
            // stake table we started with.
            for (_, validator) in inner.quorum_for_epoch(*epoch) {
                register_validator_events(&mut events, validator);
            }
        } else if *epoch > inner.first_epoch_with_dynamic_stake_table {
            // For each subsequent epoch, just generate one event for the new validator which joined
            // in that epoch.
            let (_, validator) = &inner.quorum_for_epoch(*epoch)[(*epoch as usize) - 1];
            register_validator_events(&mut events, validator);
        }
        Ok(events)
    }

    async fn payload_proof(&self, id: BlockId<SeqTypes>) -> Result<PayloadProof> {
        let mut inner = self.inner.lock().await;

        let height = inner.leaf_height(id.into())?;
        let epoch = epoch_from_block_number(height as u64, self.epoch_height);
        let quorum = inner.quorum_for_epoch(epoch);
        let vid_common = VidCommon::V1(init_avidm_param(quorum.len()).unwrap());

        let payload = if inner.invalid_payloads.contains(&height) {
            Payload::from_transactions_sync(
                [Transaction::random(&mut rand::thread_rng())],
                NodeState::mock_v3().chain_config,
            )
            .unwrap()
            .0
        } else {
            inner.payloads[height].clone()
        };

        Ok(PayloadProof::new(payload, vid_common))
    }
}

fn register_validator_events(events: &mut Vec<StakeTableEvent>, validator: &Validator<PubKey>) {
    events.push(StakeTableEvent::Register(ValidatorRegistered {
        account: validator.account,
        blsVk: validator.stake_table_key.into(),
        schnorrVk: validator.state_ver_key.clone().into(),
        commission: validator.commission,
    }));
    for (&delegator, &amount) in &validator.delegators {
        events.push(StakeTableEvent::Delegate(Delegated {
            delegator,
            validator: validator.account,
            amount,
        }));
    }
}

pub fn random_validator() -> Validator<PubKey> {
    let account = Address::random();
    let mut seed = [0; 32];
    rand::thread_rng().fill_bytes(&mut seed);
    let stake = U256::from(rand::thread_rng().next_u64());
    Validator {
        account,
        stake_table_key: PubKey::generated_from_seed_indexed(seed, 0).0,
        state_ver_key: SchnorrPubKey::generated_from_seed_indexed(seed, 0).0,
        stake,
        commission: 1,
        delegators: [(Address::random(), stake)].into_iter().collect(),
    }
}
