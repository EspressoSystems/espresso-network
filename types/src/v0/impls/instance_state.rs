use std::{collections::BTreeMap, sync::Arc};

#[cfg(any(test, feature = "testing"))]
use async_lock::RwLock;
use async_trait::async_trait;
use hotshot::types::BLSPubKey;
use hotshot_types::{
    data::EpochNumber, epoch_membership::EpochMembershipCoordinator, traits::states::InstanceState,
    HotShotConfig,
};
use indexmap::IndexMap;
#[cfg(any(test, feature = "testing"))]
use vbs::version::StaticVersionType;
use vbs::version::Version;

use super::{
    state::ValidatedState,
    traits::MembershipPersistence,
    v0_1::NoStorage,
    v0_3::{IndexedStake, Validator},
    SeqTypes,
};
use crate::v0::{
    traits::StateCatchup, v0_99::ChainConfig, GenesisHeader, L1BlockInfo, L1Client, Timestamp,
    Upgrade, UpgradeMode,
};
#[cfg(any(test, feature = "testing"))]
use crate::EpochCommittees;

/// Represents the immutable state of a node.
///
/// For mutable state, use `ValidatedState`.
#[derive(derive_more::Debug, Clone)]
pub struct NodeState {
    pub node_id: u64,
    pub chain_config: crate::v0_99::ChainConfig,
    pub l1_client: L1Client,
    #[debug("{}", peers.name())]
    pub peers: Arc<dyn StateCatchup>,
    pub genesis_header: GenesisHeader,
    pub genesis_state: ValidatedState,
    pub l1_genesis: Option<L1BlockInfo>,
    #[debug(skip)]
    pub coordinator: EpochMembershipCoordinator<SeqTypes>,
    pub epoch_height: Option<u64>,

    /// Map containing all planned and executed upgrades.
    ///
    /// Currently, only one upgrade can be executed at a time.
    /// For multiple upgrades, the node needs to be restarted after each upgrade.
    ///
    /// This field serves as a record for planned and past upgrades,
    /// listed in the genesis TOML file. It will be very useful if multiple upgrades
    /// are supported in the future.
    pub upgrades: BTreeMap<Version, Upgrade>,
    /// Current version of the sequencer.
    ///
    /// This version is checked to determine if an upgrade is planned,
    /// and which version variant for versioned types  
    /// to use in functions such as genesis.
    /// (example: genesis returns V2 Header if version is 0.2)
    pub current_version: Version,
}

#[async_trait]
impl MembershipPersistence for NoStorage {
    async fn load_stake(
        &self,
        _epoch: EpochNumber,
    ) -> anyhow::Result<Option<IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>>> {
        Ok(None)
    }

    async fn load_latest_stake(&self, _limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        Ok(None)
    }

    async fn store_stake(
        &self,
        _epoch: EpochNumber,
        _stake: IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl NodeState {
    pub fn new(
        node_id: u64,
        chain_config: ChainConfig,
        l1_client: L1Client,
        catchup: impl StateCatchup + 'static,
        current_version: Version,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
    ) -> Self {
        Self {
            node_id,
            chain_config,
            l1_client,
            peers: Arc::new(catchup),
            genesis_header: Default::default(),
            genesis_state: ValidatedState {
                chain_config: chain_config.into(),
                ..Default::default()
            },
            l1_genesis: None,
            upgrades: Default::default(),
            current_version,
            epoch_height: None,
            coordinator,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock() -> Self {
        use vbs::version::StaticVersion;

        let chain_config = ChainConfig::default();
        let l1 = L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
            .expect("Failed to create L1 client");

        let membership = Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            vec![],
            l1.clone(),
            chain_config,
            Arc::new(mock::MockStateCatchup::default()),
            NoStorage,
        )));

        let coordinator = EpochMembershipCoordinator::new(membership, None, 100);
        Self::new(
            0,
            chain_config,
            l1,
            Arc::new(mock::MockStateCatchup::default()),
            StaticVersion::<0, 1>::version(),
            coordinator,
        )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock_v2() -> Self {
        use vbs::version::StaticVersion;

        let chain_config = ChainConfig::default();
        let l1 = L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
            .expect("Failed to create L1 client");

        let membership = Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            vec![],
            l1.clone(),
            chain_config,
            Arc::new(mock::MockStateCatchup::default()),
            NoStorage,
        )));
        let coordinator = EpochMembershipCoordinator::new(membership, None, 100);

        Self::new(
            0,
            chain_config,
            l1,
            Arc::new(mock::MockStateCatchup::default()),
            StaticVersion::<0, 2>::version(),
            coordinator,
        )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock_v3() -> Self {
        use vbs::version::StaticVersion;

        let chain_config = ChainConfig::default();
        let l1 = L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
            .expect("Failed to create L1 client");

        let membership = Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            vec![],
            l1.clone(),
            chain_config,
            Arc::new(mock::MockStateCatchup::default()),
            NoStorage,
        )));

        let coordinator = EpochMembershipCoordinator::new(membership, None, 100);
        Self::new(
            0,
            ChainConfig::default(),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            mock::MockStateCatchup::default(),
            StaticVersion::<0, 3>::version(),
            coordinator,
        )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock_v99() -> Self {
        use vbs::version::StaticVersion;
        let chain_config = ChainConfig::default();
        let l1 = L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
            .expect("Failed to create L1 client");

        let membership = Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            vec![],
            l1.clone(),
            chain_config,
            Arc::new(mock::MockStateCatchup::default()),
            NoStorage,
        )));
        let coordinator = EpochMembershipCoordinator::new(membership, None, 100);

        Self::new(
            0,
            chain_config,
            l1,
            Arc::new(mock::MockStateCatchup::default()),
            StaticVersion::<0, 99>::version(),
            coordinator,
        )
    }

    pub fn with_l1(mut self, l1_client: L1Client) -> Self {
        self.l1_client = l1_client;
        self
    }

    pub fn with_genesis(mut self, state: ValidatedState) -> Self {
        self.genesis_state = state;
        self
    }

    pub fn with_chain_config(mut self, cfg: ChainConfig) -> Self {
        self.chain_config = cfg;
        self
    }

    pub fn with_upgrades(mut self, upgrades: BTreeMap<Version, Upgrade>) -> Self {
        self.upgrades = upgrades;
        self
    }

    pub fn with_current_version(mut self, version: Version) -> Self {
        self.current_version = version;
        self
    }

    pub fn with_epoch_height(mut self, epoch_height: u64) -> Self {
        self.epoch_height = Some(epoch_height);
        self
    }
}

// This allows us to turn on `Default` on InstanceState trait
// which is used in `HotShot` by `TestBuilderImplementation`.
#[cfg(any(test, feature = "testing"))]
impl Default for NodeState {
    fn default() -> Self {
        use vbs::version::StaticVersion;
        let chain_config = ChainConfig::default();
        let l1 = L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
            .expect("Failed to create L1 client");

        let membership = Arc::new(RwLock::new(EpochCommittees::new_stake(
            vec![],
            vec![],
            l1.clone(),
            chain_config,
            Arc::new(mock::MockStateCatchup::default()),
            NoStorage,
        )));
        let coordinator = EpochMembershipCoordinator::new(membership, None, 100);

        Self::new(
            1u64,
            chain_config,
            l1,
            Arc::new(mock::MockStateCatchup::default()),
            StaticVersion::<0, 1>::version(),
            coordinator,
        )
    }
}

impl InstanceState for NodeState {}

impl Upgrade {
    pub fn set_hotshot_config_parameters(&self, config: &mut HotShotConfig<SeqTypes>) {
        match &self.mode {
            UpgradeMode::View(v) => {
                config.start_proposing_view = v.start_proposing_view;
                config.stop_proposing_view = v.stop_proposing_view;
                config.start_voting_view = v.start_voting_view.unwrap_or(0);
                config.stop_voting_view = v.stop_voting_view.unwrap_or(u64::MAX);
                config.start_proposing_time = 0;
                config.stop_proposing_time = u64::MAX;
                config.start_voting_time = 0;
                config.stop_voting_time = u64::MAX;
            },
            UpgradeMode::Time(t) => {
                config.start_proposing_time = t.start_proposing_time.unix_timestamp();
                config.stop_proposing_time = t.stop_proposing_time.unix_timestamp();
                config.start_voting_time = t.start_voting_time.unwrap_or_default().unix_timestamp();
                config.stop_voting_time = t
                    .stop_voting_time
                    .unwrap_or(Timestamp::max())
                    .unix_timestamp();
                config.start_proposing_view = 0;
                config.stop_proposing_view = u64::MAX;
                config.start_voting_view = 0;
                config.stop_voting_view = u64::MAX;
            },
        }
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod mock {
    use std::collections::HashMap;

    use async_trait::async_trait;
    use committable::Commitment;
    use hotshot_types::data::ViewNumber;
    use jf_merkle_tree::{ForgetableMerkleTreeScheme, MerkleTreeScheme};

    use super::*;
    use crate::{
        retain_accounts,
        v0_1::{RewardAccount, RewardMerkleCommitment, RewardMerkleTree},
        BackoffParams, BlockMerkleTree, FeeAccount, FeeMerkleCommitment, FeeMerkleTree, Leaf2,
    };

    #[derive(Debug, Clone, Default)]
    pub struct MockStateCatchup {
        backoff: BackoffParams,
        state: HashMap<ViewNumber, Arc<ValidatedState>>,
    }

    impl FromIterator<(ViewNumber, Arc<ValidatedState>)> for MockStateCatchup {
        fn from_iter<I: IntoIterator<Item = (ViewNumber, Arc<ValidatedState>)>>(iter: I) -> Self {
            Self {
                backoff: Default::default(),
                state: iter.into_iter().collect(),
            }
        }
    }

    #[async_trait]
    impl StateCatchup for MockStateCatchup {
        async fn try_fetch_leaves(
            &self,
            _retry: usize,
            _height: u64,
        ) -> anyhow::Result<Vec<Leaf2>> {
            Err(anyhow::anyhow!("todo"))
        }

        async fn try_fetch_accounts(
            &self,
            _retry: usize,
            _instance: &NodeState,
            _height: u64,
            view: ViewNumber,
            fee_merkle_tree_root: FeeMerkleCommitment,
            accounts: &[FeeAccount],
        ) -> anyhow::Result<FeeMerkleTree> {
            let src = &self.state[&view].fee_merkle_tree;
            assert_eq!(src.commitment(), fee_merkle_tree_root);

            tracing::info!("catchup: fetching accounts {accounts:?} for view {view:?}");
            retain_accounts(src, accounts.iter().copied())
        }

        async fn try_remember_blocks_merkle_tree(
            &self,
            _retry: usize,
            _instance: &NodeState,
            _height: u64,
            view: ViewNumber,
            mt: &mut BlockMerkleTree,
        ) -> anyhow::Result<()> {
            tracing::info!("catchup: fetching frontier for view {view:?}");
            let src = &self.state[&view].block_merkle_tree;

            assert_eq!(src.commitment(), mt.commitment());
            assert!(
                src.num_leaves() > 0,
                "catchup should not be triggered when blocks tree is empty"
            );

            let index = src.num_leaves() - 1;
            let (elem, proof) = src.lookup(index).expect_ok().unwrap();
            mt.remember(index, elem, proof.clone())
                .expect("Proof verifies");

            Ok(())
        }

        async fn try_fetch_chain_config(
            &self,
            _retry: usize,
            _commitment: Commitment<ChainConfig>,
        ) -> anyhow::Result<ChainConfig> {
            Ok(ChainConfig::default())
        }

        async fn try_fetch_reward_accounts(
            &self,
            _retry: usize,
            _instance: &NodeState,
            _height: u64,
            _view: ViewNumber,
            _reward_merkle_tree_root: RewardMerkleCommitment,
            _accounts: &[RewardAccount],
        ) -> anyhow::Result<RewardMerkleTree> {
            anyhow::bail!("unimplemented")
        }

        fn backoff(&self) -> &BackoffParams {
            &self.backoff
        }

        fn name(&self) -> String {
            "MockStateCatchup".into()
        }
    }
}
