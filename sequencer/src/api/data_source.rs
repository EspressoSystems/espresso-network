use std::{collections::HashMap, time::Duration};

use alloy::primitives::Address;
use anyhow::Context;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    config::PublicNetworkConfig,
    v0::traits::{PersistenceOptions, SequencerPersistence},
    v0_3::{
        ChainConfig, RewardAccountProofV1, RewardAccountQueryDataV1, RewardAccountV1, RewardAmount,
        RewardMerkleTreeV1, Validator,
    },
    v0_4::{RewardAccountProofV2, RewardAccountQueryDataV2, RewardAccountV2, RewardMerkleTreeV2},
    FeeAccount, FeeAccountProof, FeeMerkleTree, Leaf2, NodeState, PubKey, Transaction,
};
use futures::future::Future;
use hotshot::types::BLSPubKey;
use hotshot_query_service::{
    availability::{AvailabilityDataSource, VidCommonQueryData},
    data_source::{UpdateDataSource, VersionedDataSource},
    fetching::provider::{AnyProvider, QueryServiceProvider},
    node::NodeDataSource,
    status::StatusDataSource,
};
use hotshot_types::{
    data::{EpochNumber, VidShare, ViewNumber},
    light_client::LCV3StateSignatureRequestBody,
    simple_certificate::LightClientStateUpdateCertificateV2,
    traits::{
        network::ConnectedNetwork,
        node_implementation::{NodeType, Versions},
    },
    PeerConfig,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use tide_disco::Url;

use super::{
    fs,
    options::{Options, Query},
    sql, AccountQueryData, BlocksFrontier,
};
use crate::{persistence, state_cert::StateCertFetchError, SeqTypes, SequencerApiVersion};

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
    async fn get_state_signature(&self, height: u64) -> Option<LCV3StateSignatureRequestBody>;
}

pub(crate) trait NodeStateDataSource {
    fn node_state(&self) -> impl Send + Future<Output = NodeState>;
}

#[derive(Serialize, Deserialize)]
#[serde(bound = "T: NodeType")]
pub struct StakeTableWithEpochNumber<T: NodeType> {
    pub epoch: Option<EpochNumber>,
    pub stake_table: Vec<PeerConfig<T>>,
}

pub(crate) trait StakeTableDataSource<T: NodeType> {
    /// Get the stake table for a given epoch
    fn get_stake_table(
        &self,
        epoch: Option<<T as NodeType>::Epoch>,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<PeerConfig<T>>>>;

    /// Get the stake table for the current epoch if not provided
    fn get_stake_table_current(
        &self,
    ) -> impl Send + Future<Output = anyhow::Result<StakeTableWithEpochNumber<T>>>;

    /// Get all the validators
    fn get_validators(
        &self,
        epoch: <T as NodeType>::Epoch,
    ) -> impl Send + Future<Output = anyhow::Result<IndexMap<Address, Validator<BLSPubKey>>>>;

    fn get_block_reward(
        &self,
        epoch: Option<EpochNumber>,
    ) -> impl Send + Future<Output = anyhow::Result<Option<RewardAmount>>>;
    /// Get the current proposal participation.
    fn current_proposal_participation(
        &self,
    ) -> impl Send + Future<Output = HashMap<BLSPubKey, f64>>;

    /// Get the previous proposal participation.
    fn previous_proposal_participation(
        &self,
    ) -> impl Send + Future<Output = HashMap<BLSPubKey, f64>>;

    fn get_all_validators(
        &self,
        epoch: <T as NodeType>::Epoch,
        offset: u64,
        limit: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<Validator<PubKey>>>>;
}

// Thin wrapper trait to access persistence methods from API handlers
#[async_trait]
pub(crate) trait StateCertDataSource {
    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>>;

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()>;
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
                format!("account {account} not available for height {height}, view {view}"),
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

    fn get_leaf_chain(
        &self,
        height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<Leaf2>>>;

    /// Get the state of the requested `account`.
    ///
    /// The state is fetched from a snapshot at the given height and view, which _must_ correspond!
    /// `height` is provided to simplify lookups for backends where data is not indexed by view.
    /// This function is intended to be used for catchup, so `view` should be no older than the last
    /// decided view.
    fn get_reward_account_v2(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        account: RewardAccountV2,
    ) -> impl Send + Future<Output = anyhow::Result<RewardAccountQueryDataV2>> {
        async move {
            let tree = self
                .get_reward_accounts_v2(instance, height, view, &[account])
                .await?;
            let (proof, balance) = RewardAccountProofV2::prove(&tree, account.into()).context(
                format!("reward account {account} not available for height {height}, view {view}"),
            )?;
            Ok(RewardAccountQueryDataV2 { balance, proof })
        }
    }

    fn get_reward_accounts_v2(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV2],
    ) -> impl Send + Future<Output = anyhow::Result<RewardMerkleTreeV2>>;

    fn get_reward_account_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        account: RewardAccountV1,
    ) -> impl Send + Future<Output = anyhow::Result<RewardAccountQueryDataV1>> {
        async move {
            let tree = self
                .get_reward_accounts_v1(instance, height, view, &[account])
                .await?;
            let (proof, balance) = RewardAccountProofV1::prove(&tree, account.into()).context(
                format!("reward account {account} not available for height {height}, view {view}"),
            )?;
            Ok(RewardAccountQueryDataV1 { balance, proof })
        }
    }

    fn get_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV1],
    ) -> impl Send + Future<Output = anyhow::Result<RewardMerkleTreeV1>>;

    fn get_state_cert(
        &self,
        epoch: u64,
    ) -> impl Send + Future<Output = anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>>>;
}

#[async_trait]
pub trait RequestResponseDataSource<Types: NodeType> {
    async fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common_data: VidCommonQueryData<Types>,
        duration: Duration,
    ) -> anyhow::Result<Vec<VidShare>>;
}

#[async_trait]
pub trait StateCertFetchingDataSource<Types: NodeType> {
    async fn request_state_cert(
        &self,
        epoch: u64,
        timeout: Duration,
    ) -> Result<LightClientStateUpdateCertificateV2<Types>, StateCertFetchError>;
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
