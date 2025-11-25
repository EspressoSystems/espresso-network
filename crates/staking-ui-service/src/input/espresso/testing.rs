#![cfg(any(test, feature = "testing"))]

use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use alloy::{
    network::EthereumWallet, node_bindings::Anvil, primitives::FixedBytes,
    providers::ProviderBuilder,
};
use async_lock::RwLock;
use bitvec::vec::BitVec;
use espresso_contract_deployer::build_signer;
use espresso_types::{
    ChainConfig, DrbAndHeaderUpgradeVersion, Leaf2, NodeState, PubKey, SeqTypes, SequencerVersions,
    ValidatedState, ValidatorMap, traits::PersistenceOptions, v0_3::Validator,
};
use futures::{Stream, StreamExt, stream};
use hotshot_query_service::data_source::{SqlDataSource, sql::testing::TmpDb};
use hotshot_types::{
    data::{EpochNumber, QuorumProposal2, QuorumProposalWrapper, ViewNumber},
    drb::DrbResult,
    simple_certificate::QuorumCertificate2,
    traits::{
        node_implementation::ConsensusTime,
        signature_key::{SignatureKey, StateSignatureKey},
    },
    utils::{epoch_from_block_number, is_transition_block, transition_block_for_epoch},
};
use jf_signature::schnorr::VerKey;
use rand::{RngCore, SeedableRng, rngs::StdRng};
use sequencer::{
    api::{
        self,
        data_source::testing::TestableSequencerDataSource,
        sql::DataSource,
        test_helpers::{TestNetwork, TestNetworkConfigBuilder},
    },
    testing::TestConfigBuilder,
};
use staking_cli::{
    DEV_MNEMONIC,
    demo::{DelegationConfig, StakingTransactions},
};

use crate::{
    Error, Result,
    error::ensure,
    input::{
        espresso::{
            ActiveNode, ActiveNodeSet, EspressoClient, EspressoPersistence, RewardDistribution,
        },
        l1::testing::ContractDeployment,
    },
    types::{
        common::{Address, ESPTokenAmount},
        global::{ActiveNodeSetDiff, ActiveNodeSetUpdate},
    },
};

type V = SequencerVersions<DrbAndHeaderUpgradeVersion, DrbAndHeaderUpgradeVersion>;

/// An Espresso client pre-loaded with a stake table and a series of leaves.
#[derive(Clone, Debug)]
pub struct MockEspressoClient {
    leaves: Vec<(Leaf2, BitVec)>,
    epoch_height: u64,
    num_nodes: u64,
    unavailable_leaves: HashSet<u64>,
    unavailable_stake_tables: HashSet<u64>,
}

impl EspressoClient for MockEspressoClient {
    async fn wait_for_epochs(&self) -> u64 {
        let leaf = &self.leaves[self.leaves.len() - 1].0;
        *leaf.epoch(self.epoch_height).unwrap()
    }

    async fn epoch_height(&self) -> Result<u64> {
        Ok(self.epoch_height)
    }

    async fn stake_table_for_epoch(&self, epoch: u64) -> Result<ValidatorMap> {
        ensure!(
            !self.unavailable_stake_tables.contains(&epoch),
            Error::not_found().context(format!("mock error: stake table {epoch} is deleted"))
        );

        let mut rng = StdRng::seed_from_u64(epoch);
        let mut seed = [0; 32];
        rng.fill_bytes(&mut seed);
        let nodes: ValidatorMap = (0..self.num_nodes)
            .map(|i| {
                let mut address_bytes = FixedBytes::<32>::default();
                rng.fill_bytes(address_bytes.as_mut_slice());
                let account = Address::from_word(address_bytes);

                let stake_table_key = PubKey::generated_from_seed_indexed(seed, i).0;
                let state_ver_key = VerKey::generated_from_seed_indexed(seed, i).0;

                let stake = ESPTokenAmount::from(1);
                let node = Validator {
                    account,
                    stake_table_key,
                    state_ver_key,
                    stake,
                    commission: 0,
                    // Self-delegate.
                    delegators: [(account, stake)].into_iter().collect(),
                };
                (account, node)
            })
            .collect();
        Ok(nodes)
    }

    async fn leaf(&self, height: u64) -> Result<Leaf2> {
        ensure!(
            !self.unavailable_leaves.contains(&height),
            Error::not_found().context(format!("mock error: leaf {height} is deleted"))
        );

        let offset = self.leaf_offset(height).ok_or_else(Error::not_found)?;
        Ok(self.leaves[offset].0.clone())
    }

    async fn block_reward(&self, _epoch: u64) -> Result<ESPTokenAmount> {
        // 1 ESP token
        Ok(ESPTokenAmount::from(1_000_000_000_000_000_000u128))
    }

    async fn fetch_all_reward_accounts(
        &self,
        _block: u64,
    ) -> Result<Vec<(Address, ESPTokenAmount)>> {
        Ok(Vec::new())
    }

    fn leaves(&self, from: u64) -> impl Send + Unpin + Stream<Item = (Leaf2, BitVec)> {
        // Yield the leaves we already have in our buffer.
        let leaves = match self.leaf_offset(from) {
            Some(offset) => self.leaves[offset..].to_vec(),
            None => vec![],
        };

        // Block if we get to an error-injected missing leaf.
        let leaves = leaves.into_iter().take_while(|(leaf, _)| {
            if self.unavailable_leaves.contains(&leaf.height()) {
                tracing::warn!(
                    height = leaf.height(),
                    "stopping leaf stream at mock deleted leaf"
                );
                false
            } else {
                true
            }
        });

        // After yielding what we have, block as if waiting for more leaves to arrive.
        stream::iter(leaves).chain(stream::pending())
    }
}

impl MockEspressoClient {
    /// A fixed stake Espresso client with a given number of nodes.
    pub async fn new(num_nodes: usize) -> Self {
        let epoch_height = 100;
        let current_epoch = 2;
        let mut espresso = Self {
            epoch_height,
            num_nodes: num_nodes as u64,
            leaves: vec![],
            unavailable_leaves: Default::default(),
            unavailable_stake_tables: Default::default(),
        };

        // At minimum we need the end of the previous epoch, starting from the transition block.
        let transition_block = transition_block_for_epoch(current_epoch - 1, epoch_height);
        let last_block = epoch_height * (current_epoch - 1);
        for height in transition_block..=last_block {
            espresso.leaves.push((
                espresso.make_leaf(height, ViewNumber::new(height)).await,
                std::iter::repeat_n(true, num_nodes).collect(),
            ));
        }

        espresso
    }

    /// Push a new leaf, making it available for queries and streams.
    ///
    /// `skip_views`, if non-zero, allows the leaf to have a view number that skips one or more
    /// views from the previous leaf. This simulates leader failures, allowing testing of leader
    /// statistics.
    ///
    /// `voters` allows a custom bitmap of signers, for testing voter statistics.
    pub async fn push_leaf(
        &mut self,
        skip_views: u64,
        voters: impl IntoIterator<Item = bool>,
    ) -> (&Leaf2, &BitVec) {
        let parent = self.last_leaf().0;
        self.leaves.push((
            self.make_leaf(parent.height() + 1, parent.view_number() + 1 + skip_views)
                .await,
            voters.into_iter().collect(),
        ));
        self.last_leaf()
    }

    /// The last leaf appended via [`push_leaf`](Self::push_leaf).
    pub fn last_leaf(&self) -> (&Leaf2, &BitVec) {
        let (leaf, signers) = &self.leaves[self.leaves.len() - 1];
        (leaf, signers)
    }

    /// Make a leaf unavailable, so that requests for it return an error.
    pub fn delete_leaf(&mut self, height: u64) {
        // We don't actually delete the leaf, since other functions assume that our vector of leaves
        // is consecutive. We just mark it unavailable so we won't return it.
        self.unavailable_leaves.insert(height);
    }

    /// Make a stake table unavailable, so that requests for it return an error.
    pub fn delete_stake_table(&mut self, epoch: u64) {
        self.unavailable_stake_tables.insert(epoch);
    }

    async fn make_leaf(&self, height: u64, view_number: ViewNumber) -> Leaf2 {
        let mut block_header = Leaf2::genesis::<V>(&Default::default(), &NodeState::mock())
            .await
            .block_header()
            .clone();
        *block_header.height_mut() = height;
        let epoch = epoch_from_block_number(height, self.epoch_height);
        let proposal: QuorumProposalWrapper<SeqTypes> = QuorumProposal2 {
            block_header,
            view_number,
            epoch: Some(EpochNumber::new(epoch)),
            justify_qc: QuorumCertificate2::genesis::<V>(&Default::default(), &NodeState::mock())
                .await,
            next_epoch_justify_qc: None,
            upgrade_certificate: None,
            view_change_evidence: None,
            next_drb_result: is_transition_block(height, self.epoch_height)
                .then_some(fake_drb_result(epoch + 1)),
            state_cert: None,
        }
        .into();

        Leaf2::from_quorum_proposal(&proposal)
    }

    // Find the index of leaf `height` in `leaves`, if it exists.
    fn leaf_offset(&self, height: u64) -> Option<usize> {
        let first = &self.leaves.first()?.0;
        if height < first.height() {
            return None;
        }
        let offset = (height - first.height()) as usize;
        if offset >= self.leaves.len() {
            return None;
        }
        Some(offset)
    }

    /// Synchronous, infallible version of [`EspressoClient::current_epoch`].
    pub fn current_epoch(&self) -> u64 {
        let leaf = &self.leaves[self.leaves.len() - 1].0;
        *leaf.epoch(self.epoch_height).unwrap()
    }

    /// Synchronous, infallible version of [`EspressoClient::epoch_height`].
    pub fn epoch_height(&self) -> u64 {
        self.epoch_height
    }
}

pub fn fake_drb_result(epoch: u64) -> DrbResult {
    let mut drb_result = [0; 32];
    drb_result[0..8].copy_from_slice(&epoch.to_le_bytes());
    drb_result
}

/// Easy-setup storage that just uses memory.
#[derive(Clone, Debug, Default)]
pub struct MemoryStorage {
    db: Arc<RwLock<StorageInner>>,
    fail_next: Arc<AtomicBool>,
}

type StorageInner = (Option<ActiveNodeSet>, HashMap<Address, ESPTokenAmount>);

impl MemoryStorage {
    /// Create storage with a preloaded state.
    pub fn new(nodes: ActiveNodeSet, rewards: HashMap<Address, ESPTokenAmount>) -> Self {
        Self {
            db: Arc::new(RwLock::new((Some(nodes), rewards))),
            fail_next: Arc::new(false.into()),
        }
    }

    /// Cause the next operation to fail.
    pub fn fail_next(&mut self) {
        self.fail_next.store(true, Ordering::SeqCst);
    }

    /// A full snapshot of the contents of a mock database.
    ///
    /// This can be used for checking equality between different databases.
    pub async fn cmp_key(&self) -> StorageInner {
        self.db.read().await.clone()
    }

    fn mock_errors(&self) -> Result<()> {
        if self.fail_next.swap(false, Ordering::SeqCst) {
            return Err(Error::internal().context("mock error"));
        }
        Ok(())
    }
}

impl EspressoPersistence for MemoryStorage {
    async fn active_node_set(&self) -> Result<Option<ActiveNodeSet>> {
        self.mock_errors()?;
        Ok(self.db.read().await.0.clone())
    }

    async fn lifetime_rewards(&self, account: Address) -> Result<ESPTokenAmount> {
        self.mock_errors()?;
        let rewards = &self.db.read().await.1;
        // If a reward account is not in the database, it has 0 balance by default.
        Ok(rewards.get(&account).copied().unwrap_or_default())
    }

    async fn initialize_lifetime_rewards(
        &mut self,
        initial_rewards: Vec<(Address, ESPTokenAmount)>,
    ) -> Result<()> {
        self.mock_errors()?;

        let (_active_nodes, rewards) = &mut *self.db.write().await;
        rewards.clear();

        for (account, amount) in initial_rewards {
            rewards.insert(account, amount);
        }

        Ok(())
    }

    async fn apply_update(
        &mut self,
        update: ActiveNodeSetUpdate,
        new_rewards: RewardDistribution,
    ) -> Result<()> {
        self.mock_errors()?;

        let (active_nodes, rewards) = &mut *self.db.write().await;

        // Update active node statistics.
        for diff in update.diff {
            tracing::info!(?diff, "apply diff");
            match diff {
                ActiveNodeSetDiff::NewBlock {
                    leader,
                    failed_leaders,
                    voters,
                } => {
                    let active_nodes = active_nodes.as_mut().ok_or_else(Error::not_found)?;
                    active_nodes.espresso_block = update.espresso_block;

                    // Count the leader's successful view.
                    active_nodes.nodes[leader].proposals += 1;
                    active_nodes.nodes[leader].slots += 1;

                    // Count failed leaders' views.
                    for leader in failed_leaders {
                        active_nodes.nodes[leader].slots += 1;
                    }

                    // Count voters.
                    for voter in voters.iter_ones() {
                        active_nodes.nodes[voter].votes += 1;
                    }
                }
                ActiveNodeSetDiff::NewEpoch(addrs) => {
                    *active_nodes = Some(ActiveNodeSet {
                        espresso_block: update.espresso_block,
                        nodes: addrs.into_iter().map(ActiveNode::new).collect(),
                    });
                }
            }
        }

        // Pass out distributed rewards.
        for (account, amount) in new_rewards {
            *rewards.entry(account).or_default() += amount;
        }

        Ok(())
    }
}

pub const EPOCH_HEIGHT: u64 = 20;

pub async fn start_pos_network(
    port: u16,
) -> (
    TestNetwork<impl PersistenceOptions, 1, V>,
    ContractDeployment,
    TmpDb,
) {
    // Deploy PoS contracts.
    let anvil = Anvil::new()
        .block_time(1)
        .args(["--slots-in-an-epoch", "0"])
        .spawn();
    let rpc_url = anvil.endpoint_url();
    let deployment = ContractDeployment::deploy(rpc_url.clone())
        .await
        .expect("Failed to deploy contracts");

    // Configure proof of stake to start immediately.
    let test_config = TestConfigBuilder::<1>::default()
        .epoch_height(EPOCH_HEIGHT)
        .epoch_start_block(EPOCH_HEIGHT / 2)
        .anvil_provider(anvil)
        .build();

    // Set stake table address in consensus config.
    let chain_config = ChainConfig {
        stake_table_contract: Some(deployment.stake_table_addr),
        ..Default::default()
    };
    let state = ValidatedState {
        chain_config: chain_config.into(),
        ..Default::default()
    };

    // Register test nodes so they will be able to participate once the epoch changes.
    let signer = build_signer(DEV_MNEMONIC, 0);
    let provider = ProviderBuilder::new()
        .wallet(EthereumWallet::from(signer.clone()))
        .connect_http(rpc_url.clone());
    let mut txs = StakingTransactions::create(
        rpc_url,
        &provider,
        deployment.stake_table_addr,
        test_config.staking_priv_keys(),
        None,
        DelegationConfig::MultipleDelegators,
    )
    .await
    .unwrap();
    txs.apply_all().await.unwrap();

    // Start network.
    let storage = DataSource::create_storage().await;
    let persistence = <DataSource as TestableSequencerDataSource>::persistence_options(&storage);
    let options =
        SqlDataSource::options(&storage, api::Options::with_port(port)).config(Default::default());
    let config = TestNetworkConfigBuilder::with_num_nodes()
        .api_config(options.clone())
        .persistences([persistence.clone()])
        .network_config(test_config)
        .states([state])
        .build();
    let network = TestNetwork::new(config, V::new()).await;

    (network, deployment, storage)
}
