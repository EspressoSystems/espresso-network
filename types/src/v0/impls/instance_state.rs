use std::{collections::BTreeMap, sync::Arc};

use hotshot_types::{traits::states::InstanceState, HotShotConfig};
use vbs::version::Version;
#[cfg(any(test, feature = "testing"))]
use vbs::version::{StaticVersion, StaticVersionType};

use super::{
    state::ValidatedState, traits::StateCatchup, v0_99::ChainConfig, GenesisHeader, L1BlockInfo,
    L1Client, PubKey, Timestamp, Upgrade, UpgradeMode, UpgradeType,
};

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
    pub epoch_height: u64,

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

impl NodeState {
    pub fn new(
        node_id: u64,
        chain_config: ChainConfig,
        l1_client: L1Client,
        catchup: impl StateCatchup + 'static,
        current_version: Version,
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
            epoch_height: 0,
        }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock() -> Self {
        use vbs::version::StaticVersion;

        Self::new(
            0,
            ChainConfig::default(),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            mock::MockStateCatchup::default(),
            StaticVersion::<0, 1>::version(),
        )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock_v2() -> Self {
        use vbs::version::StaticVersion;

        Self::new(
            0,
            ChainConfig::default(),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            mock::MockStateCatchup::default(),
            StaticVersion::<0, 2>::version(),
        )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock_v3() -> Self {
        use vbs::version::StaticVersion;

        Self::new(
            0,
            ChainConfig::default(),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            mock::MockStateCatchup::default(),
            StaticVersion::<0, 3>::version(),
        )
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn mock_v99() -> Self {
        use vbs::version::StaticVersion;

        Self::new(
            0,
            ChainConfig::default(),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            mock::MockStateCatchup::default(),
            StaticVersion::<0, 99>::version(),
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

    /// Given a `version`, get the correct `ChainConfig` from `self.upgrades`.
    pub fn upgrade_chain_config(&self, version: Version) -> Option<ChainConfig> {
        let chain_config = (version > self.current_version).then(|| {
            self.upgrades
                .get(&version)
                .and_then(|upgrade| match upgrade.upgrade_type {
                    UpgradeType::Fee { chain_config } => Some(chain_config),
                    UpgradeType::Epoch { chain_config } => Some(chain_config),
                    UpgradeType::Marketplace { chain_config } => Some(chain_config),
                })
        });
        chain_config?
    }
}

// This allows us to turn on `Default` on InstanceState trait
// which is used in `HotShot` by `TestBuilderImplementation`.
#[cfg(any(test, feature = "testing"))]
impl Default for NodeState {
    fn default() -> Self {
        Self::new(
            1u64,
            ChainConfig::default(),
            L1Client::new(vec!["http://localhost:3331".parse().unwrap()])
                .expect("Failed to create L1 client"),
            mock::MockStateCatchup::default(),
            StaticVersion::<0, 1>::version(),
        )
    }
}

impl InstanceState for NodeState {}

impl Upgrade {
    pub fn set_hotshot_config_parameters(&self, config: &mut HotShotConfig<PubKey>) {
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
        retain_accounts, BackoffParams, BlockMerkleTree, FeeAccount, FeeMerkleCommitment,
        FeeMerkleTree, Leaf2,
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

        fn backoff(&self) -> &BackoffParams {
            &self.backoff
        }

        fn name(&self) -> String {
            "MockStateCatchup".into()
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::{v0::Versions, EpochVersion, FeeVersion, SequencerVersions, ViewBasedUpgrade};

    #[test]
    fn test_upgrade_chain_config_version_02() {
        let mut upgrades = std::collections::BTreeMap::new();
        type MySequencerVersions = SequencerVersions<StaticVersion<0, 1>, FeeVersion>;

        let mode = UpgradeMode::View(ViewBasedUpgrade {
            start_voting_view: None,
            stop_voting_view: None,
            start_proposing_view: 1,
            stop_proposing_view: 10,
        });

        let upgraded_chain_config = ChainConfig {
            max_block_size: 300.into(),
            base_fee: 1.into(),
            ..Default::default()
        };

        let upgrade_type = UpgradeType::Fee {
            chain_config: upgraded_chain_config,
        };

        upgrades.insert(
            <MySequencerVersions as Versions>::Upgrade::VERSION,
            Upgrade { mode, upgrade_type },
        );

        let instance_state = NodeState::mock().with_upgrades(upgrades);

        let chain_config = instance_state.upgrade_chain_config(FeeVersion::version());
        assert_eq!(Some(upgraded_chain_config), chain_config);
    }

    #[test]
    fn test_upgrade_chain_config_version_03() {
        let mut upgrades = std::collections::BTreeMap::new();
        type MySequencerVersions = SequencerVersions<FeeVersion, EpochVersion>;

        let mode = UpgradeMode::View(ViewBasedUpgrade {
            start_voting_view: None,
            stop_voting_view: None,
            start_proposing_view: 1,
            stop_proposing_view: 10,
        });

        let upgraded_chain_config = ChainConfig {
            max_block_size: 300.into(),
            base_fee: 1.into(),
            stake_table_contract: Some(Default::default()),
            ..Default::default()
        };

        let upgrade_type = UpgradeType::Epoch {
            chain_config: upgraded_chain_config,
        };

        upgrades.insert(
            <MySequencerVersions as Versions>::Upgrade::VERSION,
            Upgrade { mode, upgrade_type },
        );

        let instance_state = NodeState::mock_v2().with_upgrades(upgrades);

        let chain_config = instance_state.upgrade_chain_config(EpochVersion::version());
        assert_eq!(Some(upgraded_chain_config), chain_config);
    }
}
