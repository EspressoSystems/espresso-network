use std::{num::NonZeroUsize, time::Duration};

use anyhow::Context;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    v0::traits::{PersistenceOptions, SequencerPersistence},
    v0_99::ChainConfig,
    FeeAccount, FeeAccountProof, FeeMerkleTree, NodeState, PubKey, Transaction,
};
use futures::future::Future;
use hotshot_query_service::{
    availability::AvailabilityDataSource,
    data_source::{UpdateDataSource, VersionedDataSource},
    fetching::provider::{AnyProvider, QueryServiceProvider},
    node::NodeDataSource,
    status::StatusDataSource,
};
use hotshot_types::{
    data::ViewNumber,
    light_client::StateSignatureRequestBody,
    network::NetworkConfig,
    traits::{network::ConnectedNetwork, node_implementation::Versions},
    HotShotConfig, PeerConfig, ValidatorConfig,
};
use hotshot_types::{
    network::{BuilderType, CombinedNetworkConfig, Libp2pConfig, RandomBuilderConfig},
    traits::node_implementation::NodeType,
};
use serde::{Deserialize, Serialize};
use tide_disco::Url;
use vec1::Vec1;

use super::{
    fs,
    options::{Options, Query},
    sql, AccountQueryData, BlocksFrontier,
};
use crate::{
    persistence::{self},
    SeqTypes, SequencerApiVersion,
};

pub trait DataSourceOptions: PersistenceOptions {
    type DataSource: SequencerDataSource<Options = Self>;

    fn enable_query_module(&self, opt: Options, query: Query) -> Options;
}

impl DataSourceOptions for persistence::sql::Options {
    type DataSource = sql::DataSource;

    fn enable_query_module(&self, opt: Options, query: Query) -> Options {
        opt.query_sql(query, self.clone())
    }
}

impl DataSourceOptions for persistence::fs::Options {
    type DataSource = fs::DataSource;

    fn enable_query_module(&self, opt: Options, query: Query) -> Options {
        opt.query_fs(query, self.clone())
    }
}

/// A data source with sequencer-specific functionality.
///
/// This trait extends the generic [`AvailabilityDataSource`] with some additional data needed to
/// provided sequencer-specific endpoints.
#[async_trait]
pub trait SequencerDataSource:
    AvailabilityDataSource<SeqTypes>
    + NodeDataSource<SeqTypes>
    + StatusDataSource
    + UpdateDataSource<SeqTypes>
    + VersionedDataSource
    + Sized
{
    type Options: DataSourceOptions<DataSource = Self>;

    /// Instantiate a data source from command line options.
    async fn create(opt: Self::Options, provider: Provider, reset: bool) -> anyhow::Result<Self>;
}

/// Provider for fetching missing data for the query service.
pub type Provider = AnyProvider<SeqTypes>;

/// Create a provider for fetching missing data from a list of peer query services.
pub fn provider<V: Versions>(
    peers: impl IntoIterator<Item = Url>,
    bind_version: SequencerApiVersion,
) -> Provider {
    let mut provider = Provider::default();
    for peer in peers {
        tracing::info!("will fetch missing data from {peer}");
        provider = provider.with_provider(QueryServiceProvider::new(peer, bind_version));
    }
    provider
}

pub(crate) trait SubmitDataSource<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> {
    fn submit(&self, tx: Transaction) -> impl Send + Future<Output = anyhow::Result<()>>;
}

pub(crate) trait HotShotConfigDataSource {
    fn get_config(&self) -> impl Send + Future<Output = PublicNetworkConfig>;
}

#[async_trait]
pub(crate) trait StateSignatureDataSource<N: ConnectedNetwork<PubKey>> {
    async fn get_state_signature(&self, height: u64) -> Option<StateSignatureRequestBody>;
}

pub(crate) trait NodeStateDataSource {
    fn node_state(&self) -> impl Send + Future<Output = &NodeState>;
}

pub(crate) trait StakeTableDataSource<T: NodeType> {
    /// Get the stake table for a given epoch
    fn get_stake_table(
        &self,
        epoch: Option<<T as NodeType>::Epoch>,
    ) -> impl Send + Future<Output = Vec<PeerConfig<T::SignatureKey>>>;

    /// Get the stake table for  the current epoch if not provided
    fn get_stake_table_current(
        &self,
    ) -> impl Send + Future<Output = Vec<PeerConfig<T::SignatureKey>>>;
}

pub(crate) trait CatchupDataSource: Sync {
    /// Get the state of the requested `account`.
    ///
    /// The state is fetched from a snapshot at the given height and view, which _must_ correspond!
    /// `height` is provided to simplify lookups for backends where data is not indexed by view.
    /// This function is intended to be used for catchup, so `view` should be no older than the last
    /// decided view.
    fn get_account(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        account: FeeAccount,
    ) -> impl Send + Future<Output = anyhow::Result<AccountQueryData>> {
        async move {
            let tree = self
                .get_accounts(instance, height, view, &[account])
                .await?;
            let (proof, balance) = FeeAccountProof::prove(&tree, account.into()).context(
                format!("account {account} not available for height {height}, view {view:?}"),
            )?;
            Ok(AccountQueryData { balance, proof })
        }
    }

    /// Get the state of the requested `accounts`.
    ///
    /// The state is fetched from a snapshot at the given height and view, which _must_ correspond!
    /// `height` is provided to simplify lookups for backends where data is not indexed by view.
    /// This function is intended to be used for catchup, so `view` should be no older than the last
    /// decided view.
    fn get_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[FeeAccount],
    ) -> impl Send + Future<Output = anyhow::Result<FeeMerkleTree>>;

    /// Get the blocks Merkle tree frontier.
    ///
    /// The state is fetched from a snapshot at the given height and view, which _must_ correspond!
    /// `height` is provided to simplify lookups for backends where data is not indexed by view.
    /// This function is intended to be used for catchup, so `view` should be no older than the last
    /// decided view.
    fn get_frontier(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
    ) -> impl Send + Future<Output = anyhow::Result<BlocksFrontier>>;

    fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> impl Send + Future<Output = anyhow::Result<ChainConfig>>;
}

/// This struct defines the public Hotshot validator configuration.
/// Private key and state key pairs are excluded for security reasons.

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicValidatorConfig {
    public_key: PubKey,
    stake_value: u64,
    is_da: bool,
    private_key: String,
    state_public_key: String,
    state_key_pair: String,
}

impl From<ValidatorConfig<PubKey>> for PublicValidatorConfig {
    fn from(v: ValidatorConfig<PubKey>) -> Self {
        let ValidatorConfig::<PubKey> {
            public_key,
            private_key: _,
            stake_value,
            state_key_pair,
            is_da,
        } = v;

        let state_public_key = state_key_pair.ver_key();

        Self {
            public_key,
            stake_value,
            is_da,
            state_public_key: state_public_key.to_string(),
            private_key: "*****".into(),
            state_key_pair: "*****".into(),
        }
    }
}

/// This struct defines the public Hotshot configuration parameters.
/// Our config module features a GET endpoint accessible via the route `/hotshot` to display the hotshot config parameters.
/// Hotshot config has sensitive information like private keys and such fields are excluded from this struct.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicHotShotConfig {
    start_threshold: (u64, u64),
    num_nodes_with_stake: NonZeroUsize,
    known_nodes_with_stake: Vec<PeerConfig<PubKey>>,
    known_da_nodes: Vec<PeerConfig<PubKey>>,
    da_staked_committee_size: usize,
    fixed_leader_for_gpuvid: usize,
    next_view_timeout: u64,
    view_sync_timeout: Duration,
    num_bootstrap: usize,
    builder_timeout: Duration,
    data_request_delay: Duration,
    builder_urls: Vec1<Url>,
    start_proposing_view: u64,
    stop_proposing_view: u64,
    start_voting_view: u64,
    stop_voting_view: u64,
    start_proposing_time: u64,
    stop_proposing_time: u64,
    start_voting_time: u64,
    stop_voting_time: u64,
    epoch_height: u64,
}

impl From<HotShotConfig<PubKey>> for PublicHotShotConfig {
    fn from(v: HotShotConfig<PubKey>) -> Self {
        // Destructure all fields from HotShotConfig to return an error
        // if new fields are added to HotShotConfig. This makes sure that we handle
        // all fields appropriately and do not miss any updates.
        let HotShotConfig::<PubKey> {
            start_threshold,
            num_nodes_with_stake,
            known_nodes_with_stake,
            known_da_nodes,
            da_staked_committee_size,
            fixed_leader_for_gpuvid,
            next_view_timeout,
            view_sync_timeout,
            num_bootstrap,
            builder_timeout,
            data_request_delay,
            builder_urls,
            start_proposing_view,
            stop_proposing_view,
            start_voting_view,
            stop_voting_view,
            start_proposing_time,
            stop_proposing_time,
            start_voting_time,
            stop_voting_time,
            epoch_height,
        } = v;

        Self {
            start_threshold,
            num_nodes_with_stake,
            known_nodes_with_stake,
            known_da_nodes,
            da_staked_committee_size,
            fixed_leader_for_gpuvid,
            next_view_timeout,
            view_sync_timeout,
            num_bootstrap,
            builder_timeout,
            data_request_delay,
            builder_urls,
            start_proposing_view,
            stop_proposing_view,
            start_voting_view,
            stop_voting_view,
            start_proposing_time,
            stop_proposing_time,
            start_voting_time,
            stop_voting_time,
            epoch_height,
        }
    }
}

impl PublicHotShotConfig {
    pub fn into_hotshot_config(self) -> HotShotConfig<PubKey> {
        HotShotConfig {
            start_threshold: self.start_threshold,
            num_nodes_with_stake: self.num_nodes_with_stake,
            known_nodes_with_stake: self.known_nodes_with_stake,
            known_da_nodes: self.known_da_nodes,
            da_staked_committee_size: self.da_staked_committee_size,
            fixed_leader_for_gpuvid: self.fixed_leader_for_gpuvid,
            next_view_timeout: self.next_view_timeout,
            view_sync_timeout: self.view_sync_timeout,
            num_bootstrap: self.num_bootstrap,
            builder_timeout: self.builder_timeout,
            data_request_delay: self.data_request_delay,
            builder_urls: self.builder_urls,
            start_proposing_view: self.start_proposing_view,
            stop_proposing_view: self.stop_proposing_view,
            start_voting_view: self.start_voting_view,
            stop_voting_view: self.stop_voting_view,
            start_proposing_time: self.start_proposing_time,
            stop_proposing_time: self.stop_proposing_time,
            start_voting_time: self.start_voting_time,
            stop_voting_time: self.stop_voting_time,
            epoch_height: self.epoch_height,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicNetworkConfig {
    rounds: usize,
    indexed_da: bool,
    transactions_per_round: usize,
    manual_start_password: Option<String>,
    num_bootrap: usize,
    next_view_timeout: u64,
    view_sync_timeout: Duration,
    builder_timeout: Duration,
    data_request_delay: Duration,
    node_index: u64,
    seed: [u8; 32],
    transaction_size: usize,
    key_type_name: String,
    libp2p_config: Option<Libp2pConfig>,
    config: PublicHotShotConfig,
    cdn_marshal_address: Option<String>,
    combined_network_config: Option<CombinedNetworkConfig>,
    commit_sha: String,
    builder: BuilderType,
    random_builder: Option<RandomBuilderConfig>,
}

impl From<NetworkConfig<PubKey>> for PublicNetworkConfig {
    fn from(cfg: NetworkConfig<PubKey>) -> Self {
        Self {
            rounds: cfg.rounds,
            indexed_da: cfg.indexed_da,
            transactions_per_round: cfg.transactions_per_round,
            manual_start_password: Some("*****".into()),
            num_bootrap: cfg.num_bootrap,
            next_view_timeout: cfg.next_view_timeout,
            view_sync_timeout: cfg.view_sync_timeout,
            builder_timeout: cfg.builder_timeout,
            data_request_delay: cfg.data_request_delay,
            node_index: cfg.node_index,
            seed: cfg.seed,
            transaction_size: cfg.transaction_size,
            key_type_name: cfg.key_type_name,
            libp2p_config: cfg.libp2p_config,
            config: cfg.config.into(),
            cdn_marshal_address: cfg.cdn_marshal_address,
            combined_network_config: cfg.combined_network_config,
            commit_sha: cfg.commit_sha,
            builder: cfg.builder,
            random_builder: cfg.random_builder,
        }
    }
}

impl PublicNetworkConfig {
    pub fn into_network_config(
        self,
        my_own_validator_config: ValidatorConfig<PubKey>,
    ) -> anyhow::Result<NetworkConfig<PubKey>> {
        let node_index = self
            .config
            .known_nodes_with_stake
            .iter()
            .position(|peer| peer.stake_table_entry.stake_key == my_own_validator_config.public_key)
            .context(format!(
                "the node {} is not in the stake table",
                my_own_validator_config.public_key
            ))? as u64;

        Ok(NetworkConfig {
            rounds: self.rounds,
            indexed_da: self.indexed_da,
            transactions_per_round: self.transactions_per_round,
            manual_start_password: self.manual_start_password,
            num_bootrap: self.num_bootrap,
            next_view_timeout: self.next_view_timeout,
            view_sync_timeout: self.view_sync_timeout,
            builder_timeout: self.builder_timeout,
            data_request_delay: self.data_request_delay,
            node_index,
            seed: self.seed,
            transaction_size: self.transaction_size,
            key_type_name: self.key_type_name,
            libp2p_config: self.libp2p_config,
            config: self.config.into_hotshot_config(),
            cdn_marshal_address: self.cdn_marshal_address,
            combined_network_config: self.combined_network_config,
            commit_sha: self.commit_sha,
            builder: self.builder,
            random_builder: self.random_builder,
            public_keys: Vec::new(),
        })
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod testing {
    use super::{super::Options, *};

    #[async_trait]
    pub trait TestableSequencerDataSource: SequencerDataSource {
        type Storage: Sync;

        async fn create_storage() -> Self::Storage;
        fn persistence_options(storage: &Self::Storage) -> Self::Options;
        fn leaf_only_ds_options(
            _storage: &Self::Storage,
            _opt: Options,
        ) -> anyhow::Result<Options> {
            anyhow::bail!("not supported")
        }
        fn options(storage: &Self::Storage, opt: Options) -> Options;
    }
}
