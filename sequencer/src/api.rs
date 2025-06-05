use std::{pin::Pin, sync::Arc, time::Duration};

use alloy::primitives::Address;
use anyhow::{bail, Context};
use async_lock::RwLock;
use async_once_cell::Lazy;
use async_trait::async_trait;
use committable::Commitment;
use data_source::{
    CatchupDataSource, RequestResponseDataSource, StakeTableDataSource, StakeTableWithEpochNumber,
    SubmitDataSource,
};
use derivative::Derivative;
use espresso_types::{
    config::PublicNetworkConfig,
    retain_accounts,
    v0::traits::SequencerPersistence,
    v0_1::{RewardAccount, RewardMerkleTree},
    v0_3::{ChainConfig, Validator},
    AccountQueryData, BlockMerkleTree, FeeAccount, FeeMerkleTree, Leaf2, NodeState, PubKey,
    Transaction,
};
use futures::{
    future::{BoxFuture, Future, FutureExt},
    stream::BoxStream,
};
use hotshot::types::BLSPubKey;
use hotshot_events_service::events_source::{
    EventFilterSet, EventsSource, EventsStreamer, StartupInfo,
};
use hotshot_query_service::{
    availability::VidCommonQueryData, data_source::ExtensibleDataSource, VidCommon,
};
use hotshot_types::{
    data::{VidCommitment, VidShare, ViewNumber},
    event::{Event, LegacyEvent},
    light_client::StateSignatureRequestBody,
    network::NetworkConfig,
    traits::{
        network::ConnectedNetwork,
        node_implementation::{NodeType, Versions},
    },
    vid::avidm::{init_avidm_param, AvidMScheme},
    vote::HasViewNumber,
    PeerConfig,
};
use indexmap::IndexMap;
use itertools::Itertools;
use jf_merkle_tree::MerkleTreeScheme;
use rand::Rng;
use request_response::RequestType;
use tokio::time::timeout;

use self::data_source::{HotShotConfigDataSource, NodeStateDataSource, StateSignatureDataSource};
use crate::{
    catchup::{add_fee_accounts_to_state, add_reward_accounts_to_state, CatchupStorage},
    context::Consensus,
    request_response::{
        data_source::retain_reward_accounts,
        request::{Request, Response},
    },
    state_signature::StateSigner,
    SeqTypes, SequencerApiVersion, SequencerContext,
};

pub mod data_source;
pub mod endpoints;
pub mod fs;
pub mod options;
pub mod sql;
mod update;

pub use options::Options;

pub type BlocksFrontier = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;

type BoxLazy<T> = Pin<Arc<Lazy<T, BoxFuture<'static, T>>>>;

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct ConsensusState {
    state_signer: Arc<RwLock<StateSigner<SequencerApiVersion>>>,
    event_streamer: Arc<RwLock<EventsStreamer<SeqTypes>>>,
    node_state: NodeState,
    network_config: NetworkConfig<SeqTypes>,
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, V: Versions>
    From<&SequencerContext<N, P, V>> for ConsensusState
{
    fn from(ctx: &SequencerContext<N, P, V>) -> Self {
        Self {
            state_signer: ctx.state_signer(),
            event_streamer: ctx.event_streamer(),
            node_state: ctx.node_state(),
            network_config: ctx.network_config(),
        }
    }
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
struct ApiState<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, V: Versions> {
    // The consensus state is initialized lazily so we can start the API (and healthcheck endpoints)
    // before consensus has started. Any endpoint that uses consensus state will wait for
    // initialization to finish, but endpoints that do not require a consensus handle can proceed
    // without waiting.
    #[derivative(Debug = "ignore")]
    sequencer_context: BoxLazy<SequencerContext<N, P, V>>,
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, V: Versions> ApiState<N, P, V> {
    fn new(context_init: impl Future<Output = SequencerContext<N, P, V>> + Send + 'static) -> Self {
        Self {
            sequencer_context: Arc::pin(Lazy::from_future(context_init.boxed())),
        }
    }

    async fn state_signer(&self) -> Arc<RwLock<StateSigner<SequencerApiVersion>>> {
        self.sequencer_context
            .as_ref()
            .get()
            .await
            .get_ref()
            .state_signer()
    }

    async fn event_streamer(&self) -> Arc<RwLock<EventsStreamer<SeqTypes>>> {
        self.sequencer_context
            .as_ref()
            .get()
            .await
            .get_ref()
            .event_streamer()
    }

    async fn consensus(&self) -> Arc<RwLock<Consensus<N, P, V>>> {
        self.sequencer_context
            .as_ref()
            .get()
            .await
            .get_ref()
            .consensus()
    }

    async fn network_config(&self) -> NetworkConfig<SeqTypes> {
        self.sequencer_context
            .as_ref()
            .get()
            .await
            .get_ref()
            .network_config()
    }
}

type StorageState<N, P, D, V> = ExtensibleDataSource<D, ApiState<N, P, V>>;

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, V: Versions> EventsSource<SeqTypes>
    for ApiState<N, P, V>
{
    type EventStream = BoxStream<'static, Arc<Event<SeqTypes>>>;
    type LegacyEventStream = BoxStream<'static, Arc<LegacyEvent<SeqTypes>>>;

    async fn get_event_stream(
        &self,
        _filter: Option<EventFilterSet<SeqTypes>>,
    ) -> Self::EventStream {
        self.event_streamer()
            .await
            .read()
            .await
            .get_event_stream(None)
            .await
    }

    async fn get_legacy_event_stream(
        &self,
        _filter: Option<EventFilterSet<SeqTypes>>,
    ) -> Self::LegacyEventStream {
        self.event_streamer()
            .await
            .read()
            .await
            .get_legacy_event_stream(None)
            .await
    }

    async fn get_startup_info(&self) -> StartupInfo<SeqTypes> {
        self.event_streamer()
            .await
            .read()
            .await
            .get_startup_info()
            .await
    }
}

impl<N: ConnectedNetwork<PubKey>, D: Send + Sync, V: Versions, P: SequencerPersistence>
    SubmitDataSource<N, P> for StorageState<N, P, D, V>
{
    async fn submit(&self, tx: Transaction) -> anyhow::Result<()> {
        self.as_ref().submit(tx).await
    }
}

impl<N: ConnectedNetwork<PubKey>, D: Sync, V: Versions, P: SequencerPersistence>
    StakeTableDataSource<SeqTypes> for StorageState<N, P, D, V>
{
    /// Get the stake table for a given epoch
    async fn get_stake_table(
        &self,
        epoch: Option<<SeqTypes as NodeType>::Epoch>,
    ) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>> {
        self.as_ref().get_stake_table(epoch).await
    }

    /// Get the stake table for the current epoch if not provided
    async fn get_stake_table_current(&self) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>> {
        self.as_ref().get_stake_table_current().await
    }

    /// Get all the validators
    async fn get_validators(
        &self,
        epoch: <SeqTypes as NodeType>::Epoch,
    ) -> anyhow::Result<IndexMap<Address, Validator<BLSPubKey>>> {
        self.as_ref().get_validators(epoch).await
    }
}

impl<N: ConnectedNetwork<PubKey>, V: Versions, P: SequencerPersistence>
    StakeTableDataSource<SeqTypes> for ApiState<N, P, V>
{
    /// Get the stake table for a given epoch
    async fn get_stake_table(
        &self,
        epoch: Option<<SeqTypes as NodeType>::Epoch>,
    ) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>> {
        let highest_epoch = self
            .consensus()
            .await
            .read()
            .await
            .cur_epoch()
            .await
            .map(|e| e + 1);
        if epoch > highest_epoch {
            return Err(anyhow::anyhow!(
                "requested stake table for epoch {:?} is beyond the current epoch + 1 {:?}",
                epoch,
                highest_epoch
            ));
        }
        let mem = self
            .consensus()
            .await
            .read()
            .await
            .membership_coordinator
            .stake_table_for_epoch(epoch)
            .await?;

        Ok(mem.stake_table().await.0)
    }

    /// Get the stake table for the current epoch and return it along with the epoch number
    async fn get_stake_table_current(&self) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>> {
        let epoch = self.consensus().await.read().await.cur_epoch().await;

        Ok(StakeTableWithEpochNumber {
            epoch,
            stake_table: self.get_stake_table(epoch).await?,
        })
    }

    /// Get the whole validators map
    async fn get_validators(
        &self,
        epoch: <SeqTypes as NodeType>::Epoch,
    ) -> anyhow::Result<IndexMap<Address, Validator<BLSPubKey>>> {
        let mem = self
            .consensus()
            .await
            .read()
            .await
            .membership_coordinator
            .membership_for_epoch(Some(epoch))
            .await
            .context("membership not found")?;

        let r = mem.coordinator.membership().read().await;
        r.validators(&epoch)
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, D: Sync, V: Versions, P: SequencerPersistence>
    RequestResponseDataSource<SeqTypes> for StorageState<N, P, D, V>
{
    async fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common_data: VidCommonQueryData<SeqTypes>,
        timeout_duration: Duration,
    ) -> anyhow::Result<Vec<VidShare>> {
        self.as_ref()
            .request_vid_shares(block_number, vid_common_data, timeout_duration)
            .await
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, V: Versions, P: SequencerPersistence>
    RequestResponseDataSource<SeqTypes> for ApiState<N, P, V>
{
    async fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common_data: VidCommonQueryData<SeqTypes>,
        duration: Duration,
    ) -> anyhow::Result<Vec<VidShare>> {
        // Get a handle to the request response protocol
        let request_response_protocol = self
            .sequencer_context
            .as_ref()
            .get()
            .await
            .request_response_protocol
            .clone();

        // Get the total VID weight based on the VID common data
        let total_weight = match vid_common_data.common() {
            VidCommon::V0(_) => {
                // TODO: This needs to be done via the stake table
                return Err(anyhow::anyhow!(
                    "V0 total weight calculation not supported yet"
                ));
            },
            VidCommon::V1(v1) => v1.total_weights,
        };

        // Create the AvidM parameters from the total weight
        let avidm_param =
            init_avidm_param(total_weight).with_context(|| "failed to initialize avidm param")?;

        // Get the payload hash for verification
        let VidCommitment::V1(local_payload_hash) = vid_common_data.payload_hash() else {
            bail!("V0 share verification not supported yet");
        };

        // Create a random request id
        let request_id = rand::thread_rng().gen();

        // Request and verify the shares from all other nodes, timing out after `duration` seconds
        let received_shares = Arc::new(parking_lot::Mutex::new(Vec::new()));
        let received_shares_clone = received_shares.clone();
        let request_result: anyhow::Result<_, _> = timeout(
            duration,
            request_response_protocol.request_indefinitely::<_, _, _>(
                Request::VidShare(block_number, request_id),
                RequestType::Broadcast,
                move |_request, response| {
                    let avidm_param = avidm_param.clone();
                    let received_shares = received_shares_clone.clone();
                    async move {
                        // Make sure the response was a V1 share
                        let Response::VidShare(VidShare::V1(received_share)) = response else {
                            bail!("V0 share verification not supported yet");
                        };

                        // Verify the share
                        let Ok(Ok(_)) = AvidMScheme::verify_share(
                            &avidm_param,
                            &local_payload_hash,
                            &received_share,
                        ) else {
                            bail!("share verification failed");
                        };

                        // Add the share to the list of received shares
                        received_shares.lock().push(received_share);

                        bail!("waiting for more shares");

                        #[allow(unreachable_code)]
                        Ok(())
                    }
                },
            ),
        )
        .await;

        // If the request timed out, return the shares we have collected so far
        match request_result {
            Err(_) => {
                // If it timed out, this was successful. Return the shares we have collected so far
                Ok(received_shares
                    .lock()
                    .clone()
                    .into_iter()
                    .map(VidShare::V1)
                    .collect())
            },

            // If it was an error from the inner request, return that error
            Ok(Err(e)) => Err(e).with_context(|| "failed to request vid shares"),

            // If it was successful, this was unexpected.
            Ok(Ok(_)) => bail!("this should not be possible"),
        }
    }
}

impl<N: ConnectedNetwork<PubKey>, V: Versions, P: SequencerPersistence> SubmitDataSource<N, P>
    for ApiState<N, P, V>
{
    async fn submit(&self, tx: Transaction) -> anyhow::Result<()> {
        let handle = self.consensus().await;

        let consensus_read_lock = handle.read().await;

        // Fetch full chain config from the validated state, if present.
        // This is necessary because we support chain config upgrades,
        // so the updated chain config is found in the validated state.
        let cf = consensus_read_lock
            .decided_state()
            .await
            .chain_config
            .resolve();

        // Use the chain config from the validated state if available,
        // otherwise, use the node state's chain config
        // The node state's chain config is the node's base version chain config
        let cf = match cf {
            Some(cf) => cf,
            None => self.node_state().await.chain_config,
        };

        let max_block_size: u64 = cf.max_block_size.into();
        let txn_size = tx.payload().len() as u64;

        // reject transaction bigger than block size
        if txn_size > max_block_size {
            bail!("transaction size ({txn_size}) is greater than max_block_size ({max_block_size})")
        }

        consensus_read_lock.submit_transaction(tx).await?;
        Ok(())
    }
}

impl<N, P, D, V> NodeStateDataSource for StorageState<N, P, D, V>
where
    N: ConnectedNetwork<PubKey>,
    V: Versions,
    P: SequencerPersistence,
    D: Sync,
{
    async fn node_state(&self) -> NodeState {
        self.as_ref().node_state().await
    }
}

impl<
        N: ConnectedNetwork<PubKey>,
        V: Versions,
        P: SequencerPersistence,
        D: CatchupStorage + Send + Sync,
    > CatchupDataSource for StorageState<N, P, D, V>
{
    #[tracing::instrument(skip(self, instance))]
    async fn get_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<FeeMerkleTree> {
        // Check if we have the desired state in memory.
        match self
            .as_ref()
            .get_accounts(instance, height, view, accounts)
            .await
        {
            Ok(accounts) => return Ok(accounts),
            Err(err) => {
                tracing::info!("accounts not in memory, trying storage: {err:#}");
            },
        }

        // Try storage.
        let (tree, leaf) = self
            .inner()
            .get_accounts(instance, height, view, accounts)
            .await
            .context("accounts not in memory, and could not fetch from storage")?;
        // If we successfully fetched accounts from storage, try to add them back into the in-memory
        // state.

        let consensus = self
            .as_ref()
            .consensus()
            .await
            .read()
            .await
            .consensus()
            .clone();
        if let Err(err) =
            add_fee_accounts_to_state::<N, V, P>(&consensus, &view, accounts, &tree, leaf).await
        {
            tracing::warn!(?view, "cannot update fetched account state: {err:#}");
        }
        tracing::info!(?view, "updated with fetched account state");

        Ok(tree)
    }

    #[tracing::instrument(skip(self, instance))]
    async fn get_frontier(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
    ) -> anyhow::Result<BlocksFrontier> {
        // Check if we have the desired state in memory.
        match self.as_ref().get_frontier(instance, height, view).await {
            Ok(frontier) => return Ok(frontier),
            Err(err) => {
                tracing::info!("frontier is not in memory, trying storage: {err:#}");
            },
        }

        // Try storage.
        self.inner().get_frontier(instance, height, view).await
    }

    async fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        // Check if we have the desired state in memory.
        match self.as_ref().get_chain_config(commitment).await {
            Ok(cf) => return Ok(cf),
            Err(err) => {
                tracing::info!("chain config is not in memory, trying storage: {err:#}");
            },
        }

        // Try storage.
        self.inner().get_chain_config(commitment).await
    }
    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        // Check if we have the desired state in memory.
        match self.as_ref().get_leaf_chain(height).await {
            Ok(cf) => return Ok(cf),
            Err(err) => {
                tracing::info!("leaf chain is not in memory, trying storage: {err:#}");
            },
        }

        // Try storage.
        self.inner().get_leaf_chain(height).await
    }

    #[tracing::instrument(skip(self, instance))]
    async fn get_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<RewardMerkleTree> {
        // Check if we have the desired state in memory.
        match self
            .as_ref()
            .get_reward_accounts(instance, height, view, accounts)
            .await
        {
            Ok(accounts) => return Ok(accounts),
            Err(err) => {
                tracing::info!("reward accounts not in memory, trying storage: {err:#}");
            },
        }

        // Try storage.
        let (tree, leaf) = self
            .inner()
            .get_reward_accounts(instance, height, view, accounts)
            .await
            .context("accounts not in memory, and could not fetch from storage")?;

        // If we successfully fetched accounts from storage, try to add them back into the in-memory
        // state.
        let consensus = self
            .as_ref()
            .consensus()
            .await
            .read()
            .await
            .consensus()
            .clone();
        if let Err(err) =
            add_reward_accounts_to_state::<N, V, P>(&consensus, &view, accounts, &tree, leaf).await
        {
            tracing::warn!(?view, "cannot update fetched account state: {err:#}");
        }
        tracing::info!(?view, "updated with fetched account state");

        Ok(tree)
    }
}

impl<N, V, P> NodeStateDataSource for ApiState<N, P, V>
where
    N: ConnectedNetwork<PubKey>,
    V: Versions,
    P: SequencerPersistence,
{
    async fn node_state(&self) -> NodeState {
        self.sequencer_context.as_ref().get().await.node_state()
    }
}

impl<N: ConnectedNetwork<PubKey>, V: Versions, P: SequencerPersistence> CatchupDataSource
    for ApiState<N, P, V>
{
    #[tracing::instrument(skip(self, _instance))]
    async fn get_accounts(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<FeeMerkleTree> {
        let state = self
            .consensus()
            .await
            .read()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view:?}"
            ))?;
        retain_accounts(&state.fee_merkle_tree, accounts.iter().copied())
    }

    #[tracing::instrument(skip(self, _instance))]
    async fn get_frontier(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
    ) -> anyhow::Result<BlocksFrontier> {
        let state = self
            .consensus()
            .await
            .read()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view:?}"
            ))?;
        let tree = &state.block_merkle_tree;
        let frontier = tree.lookup(tree.num_leaves() - 1).expect_ok()?.1;
        Ok(frontier)
    }

    async fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        let state = self.consensus().await.read().await.decided_state().await;
        let chain_config = state.chain_config;

        if chain_config.commit() == commitment {
            chain_config.resolve().context("chain config found")
        } else {
            bail!("chain config not found")
        }
    }

    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        let mut leaves = self
            .consensus()
            .await
            .read()
            .await
            .consensus()
            .read()
            .await
            .undecided_leaves();
        leaves.sort_by_key(|l| l.view_number());
        let (position, mut last_leaf) = leaves
            .iter()
            .find_position(|l| l.height() == height)
            .context(format!("leaf chain not available for {height}"))?;
        let mut chain = vec![last_leaf.clone()];
        for leaf in leaves.iter().skip(position + 1) {
            if leaf.justify_qc().view_number() == last_leaf.view_number() {
                chain.push(leaf.clone());
            } else {
                continue;
            }
            if leaf.view_number() == last_leaf.view_number() + 1 {
                // one away from decide
                last_leaf = leaf;
                break;
            }
            last_leaf = leaf;
        }
        // Make sure we got one more leaf to confirm the decide
        for leaf in leaves
            .iter()
            .skip_while(|l| l.view_number() <= last_leaf.view_number())
        {
            if leaf.justify_qc().view_number() == last_leaf.view_number() {
                chain.push(leaf.clone());
                return Ok(chain);
            }
        }
        bail!(format!("leaf chain not available for {height}"))
    }

    #[tracing::instrument(skip(self, _instance))]
    async fn get_reward_accounts(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<RewardMerkleTree> {
        let state = self
            .consensus()
            .await
            .read()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view:?}"
            ))?;

        retain_reward_accounts(&state.reward_merkle_tree, accounts.iter().copied())
    }
}

impl<N: ConnectedNetwork<PubKey>, D: Sync, V: Versions, P: SequencerPersistence>
    HotShotConfigDataSource for StorageState<N, P, D, V>
{
    async fn get_config(&self) -> PublicNetworkConfig {
        self.as_ref().network_config().await.into()
    }
}

impl<N: ConnectedNetwork<PubKey>, V: Versions, P: SequencerPersistence> HotShotConfigDataSource
    for ApiState<N, P, V>
{
    async fn get_config(&self) -> PublicNetworkConfig {
        self.network_config().await.into()
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, D: Sync, V: Versions, P: SequencerPersistence>
    StateSignatureDataSource<N> for StorageState<N, P, D, V>
{
    async fn get_state_signature(&self, height: u64) -> Option<StateSignatureRequestBody> {
        self.as_ref().get_state_signature(height).await
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, V: Versions, P: SequencerPersistence> StateSignatureDataSource<N>
    for ApiState<N, P, V>
{
    async fn get_state_signature(&self, height: u64) -> Option<StateSignatureRequestBody> {
        self.state_signer()
            .await
            .read()
            .await
            .get_state_signature(height)
            .await
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers {
    use std::time::Duration;

    use alloy::{
        network::EthereumWallet,
        primitives::{Address, U256},
        providers::{ext::AnvilApi, ProviderBuilder},
    };
    use committable::Committable;
    use espresso_contract_deployer::{
        builder::DeployerArgsBuilder, network_config::light_client_genesis_from_stake_table,
        Contract, Contracts,
    };
    use espresso_types::{
        v0::traits::{NullEventConsumer, PersistenceOptions, StateCatchup},
        EpochVersion, MockSequencerVersions, NamespaceId, ValidatedState,
    };
    use futures::{
        future::{join_all, FutureExt},
        stream::StreamExt,
    };
    use hotshot::types::{Event, EventType};
    use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
    use hotshot_types::{
        event::LeafInfo,
        traits::{metrics::NoMetrics, node_implementation::ConsensusTime},
    };
    use itertools::izip;
    use jf_merkle_tree::{MerkleCommitment, MerkleTreeScheme};
    use portpicker::pick_unused_port;
    use sequencer_utils::test_utils::setup_test;
    use staking_cli::demo::{setup_stake_table_contract_for_test, DelegationConfig};
    use surf_disco::Client;
    use tempfile::TempDir;
    use tide_disco::{error::ServerError, Api, App, Error, StatusCode};
    use tokio::{spawn, task::JoinHandle, time::sleep};
    use url::Url;
    use vbs::version::{StaticVersion, StaticVersionType};

    use super::*;
    use crate::{
        catchup::NullStateCatchup,
        network,
        persistence::no_storage,
        testing::{run_legacy_builder, wait_for_decide_on_handle, TestConfig, TestConfigBuilder},
    };

    pub const STAKE_TABLE_CAPACITY_FOR_TEST: usize = 10;

    pub struct TestNetwork<P: PersistenceOptions, const NUM_NODES: usize, V: Versions> {
        pub server: SequencerContext<network::Memory, P::Persistence, V>,
        pub peers: Vec<SequencerContext<network::Memory, P::Persistence, V>>,
        pub cfg: TestConfig<{ NUM_NODES }>,
        // todo (abdul): remove this when fs storage is removed
        pub temp_dir: Option<TempDir>,
    }

    pub struct TestNetworkConfig<const NUM_NODES: usize, P, C>
    where
        P: PersistenceOptions,
        C: StateCatchup + 'static,
    {
        state: [ValidatedState; NUM_NODES],
        persistence: [P; NUM_NODES],
        catchup: [C; NUM_NODES],
        network_config: TestConfig<{ NUM_NODES }>,
        api_config: Options,
    }

    impl<const NUM_NODES: usize, P, C> TestNetworkConfig<{ NUM_NODES }, P, C>
    where
        P: PersistenceOptions,
        C: StateCatchup + 'static,
    {
        pub fn states(&self) -> [ValidatedState; NUM_NODES] {
            self.state.clone()
        }
    }
    #[derive(Clone)]
    pub struct TestNetworkConfigBuilder<const NUM_NODES: usize, P, C>
    where
        P: PersistenceOptions,
        C: StateCatchup + 'static,
    {
        state: [ValidatedState; NUM_NODES],
        persistence: Option<[P; NUM_NODES]>,
        catchup: Option<[C; NUM_NODES]>,
        api_config: Option<Options>,
        network_config: Option<TestConfig<{ NUM_NODES }>>,
    }

    impl Default for TestNetworkConfigBuilder<5, no_storage::Options, NullStateCatchup> {
        fn default() -> Self {
            TestNetworkConfigBuilder {
                state: std::array::from_fn(|_| ValidatedState::default()),
                persistence: Some([no_storage::Options; 5]),
                catchup: Some(std::array::from_fn(|_| NullStateCatchup::default())),
                network_config: None,
                api_config: None,
            }
        }
    }

    impl<const NUM_NODES: usize>
        TestNetworkConfigBuilder<{ NUM_NODES }, no_storage::Options, NullStateCatchup>
    {
        pub fn with_num_nodes(
        ) -> TestNetworkConfigBuilder<{ NUM_NODES }, no_storage::Options, NullStateCatchup>
        {
            TestNetworkConfigBuilder {
                state: std::array::from_fn(|_| ValidatedState::default()),
                persistence: Some([no_storage::Options; { NUM_NODES }]),
                catchup: Some(std::array::from_fn(|_| NullStateCatchup::default())),
                network_config: None,
                api_config: None,
            }
        }
    }

    impl<const NUM_NODES: usize, P, C> TestNetworkConfigBuilder<{ NUM_NODES }, P, C>
    where
        P: PersistenceOptions,
        C: StateCatchup + 'static,
    {
        pub fn states(mut self, state: [ValidatedState; NUM_NODES]) -> Self {
            self.state = state;
            self
        }

        pub fn persistences<NP: PersistenceOptions>(
            self,
            persistence: [NP; NUM_NODES],
        ) -> TestNetworkConfigBuilder<{ NUM_NODES }, NP, C> {
            TestNetworkConfigBuilder {
                state: self.state,
                catchup: self.catchup,
                network_config: self.network_config,
                api_config: self.api_config,
                persistence: Some(persistence),
            }
        }

        pub fn api_config(mut self, api_config: Options) -> Self {
            self.api_config = Some(api_config);
            self
        }

        pub fn catchups<NC: StateCatchup + 'static>(
            self,
            catchup: [NC; NUM_NODES],
        ) -> TestNetworkConfigBuilder<{ NUM_NODES }, P, NC> {
            TestNetworkConfigBuilder {
                state: self.state,
                catchup: Some(catchup),
                network_config: self.network_config,
                api_config: self.api_config,
                persistence: self.persistence,
            }
        }

        pub fn network_config(mut self, network_config: TestConfig<{ NUM_NODES }>) -> Self {
            self.network_config = Some(network_config);
            self
        }

        /// Setup for POS testing. Deploys contracts and adds the
        /// stake table address to state. Must be called before `build()`.
        pub async fn pos_hook<V: Versions>(
            self,
            delegation_config: DelegationConfig,
            stake_table_version: StakeTableContractVersion,
        ) -> anyhow::Result<Self> {
            if <V as Versions>::Upgrade::VERSION < EpochVersion::VERSION
                && <V as Versions>::Base::VERSION < EpochVersion::VERSION
            {
                panic!("given version does not require pos deployment");
            };

            let network_config = self
                .network_config
                .as_ref()
                .expect("network_config is required");

            let l1_url = network_config.l1_url();
            let signer = network_config.signer();
            let deployer = ProviderBuilder::new()
                .wallet(EthereumWallet::from(signer.clone()))
                .on_http(l1_url.clone());

            let blocks_per_epoch = network_config.hotshot_config().epoch_height;
            let epoch_start_block = network_config.hotshot_config().epoch_start_block;
            let (genesis_state, genesis_stake) = light_client_genesis_from_stake_table(
                &network_config.hotshot_config().hotshot_stake_table(),
                STAKE_TABLE_CAPACITY_FOR_TEST,
            )
            .unwrap();

            let mut contracts = Contracts::new();
            let args = DeployerArgsBuilder::default()
                .deployer(deployer.clone())
                .mock_light_client(true)
                .genesis_lc_state(genesis_state)
                .genesis_st_state(genesis_stake)
                .blocks_per_epoch(blocks_per_epoch)
                .epoch_start_block(epoch_start_block)
                .build()
                .unwrap();

            match stake_table_version {
                StakeTableContractVersion::V1 => {
                    args.deploy_to_stake_table_v1(&mut contracts).await
                },
                StakeTableContractVersion::V2 => args.deploy_all(&mut contracts).await,
            }
            .context("failed to deploy contracts")?;

            let stake_table_address = contracts
                .address(Contract::StakeTableProxy)
                .expect("StakeTableProxy address not found");
            let token_addr = contracts
                .address(Contract::EspTokenProxy)
                .expect("EspTokenProxy address not found");
            setup_stake_table_contract_for_test(
                l1_url.clone(),
                &deployer,
                stake_table_address,
                token_addr,
                network_config.staking_priv_keys(),
                delegation_config,
            )
            .await
            .expect("stake table setup failed");

            // enable interval mining with a 1s interval.
            // This ensures that blocks are finalized every second, even when there are no transactions.
            // It's useful for testing stake table updates,
            // which rely on the finalized L1 block number.
            if let Some(anvil) = network_config.anvil() {
                anvil
                    .anvil_set_interval_mining(1)
                    .await
                    .expect("interval mining");
            }

            // Add stake table address to `ChainConfig` (held in state),
            // avoiding overwrite other values. Base fee is set to `0` to avoid
            // unnecessary catchup of `FeeState`.
            let state = self.state[0].clone();
            let chain_config = if let Some(cf) = state.chain_config.resolve() {
                ChainConfig {
                    base_fee: 0.into(),
                    stake_table_contract: Some(stake_table_address),
                    ..cf
                }
            } else {
                ChainConfig {
                    base_fee: 0.into(),
                    stake_table_contract: Some(stake_table_address),
                    ..Default::default()
                }
            };

            let state = ValidatedState {
                chain_config: chain_config.into(),
                ..state
            };
            Ok(self.states(std::array::from_fn(|_| state.clone())))
        }

        pub fn build(self) -> TestNetworkConfig<{ NUM_NODES }, P, C> {
            TestNetworkConfig {
                state: self.state,
                persistence: self.persistence.unwrap(),
                catchup: self.catchup.unwrap(),
                network_config: self.network_config.unwrap(),
                api_config: self.api_config.unwrap(),
            }
        }
    }

    impl<P: PersistenceOptions, const NUM_NODES: usize, V: Versions> TestNetwork<P, { NUM_NODES }, V> {
        pub async fn new<C: StateCatchup + 'static>(
            cfg: TestNetworkConfig<{ NUM_NODES }, P, C>,
            bind_version: V,
        ) -> Self {
            let mut cfg = cfg;
            let mut builder_tasks = Vec::new();

            let chain_config = cfg.state[0].chain_config.resolve();
            if chain_config.is_none() {
                tracing::warn!("Chain config is not set, using default max_block_size");
            }
            let (task, builder_url) = run_legacy_builder::<{ NUM_NODES }>(
                cfg.network_config.builder_port(),
                chain_config.map(|c| *c.max_block_size),
            )
            .await;
            builder_tasks.push(task);
            cfg.network_config
                .set_builder_urls(vec1::vec1![builder_url.clone()]);

            // add default storage if none is provided as query module is now required
            let mut opt = cfg.api_config.clone();
            let temp_dir = if opt.storage_fs.is_none() && opt.storage_sql.is_none() {
                let temp_dir = tempfile::tempdir().unwrap();
                opt = opt.query_fs(
                    Default::default(),
                    crate::persistence::fs::Options::new(temp_dir.path().to_path_buf()),
                );
                Some(temp_dir)
            } else {
                None
            };

            let mut nodes = join_all(
                izip!(cfg.state, cfg.persistence, cfg.catchup)
                    .enumerate()
                    .map(|(i, (state, persistence, state_peers))| {
                        let opt = opt.clone();
                        let cfg = &cfg.network_config;
                        let upgrades_map = cfg.upgrades();
                        async move {
                            if i == 0 {
                                opt.serve(|metrics, consumer, storage| {
                                    let cfg = cfg.clone();
                                    async move {
                                        Ok(cfg
                                            .init_node(
                                                0,
                                                state,
                                                persistence,
                                                Some(state_peers),
                                                storage,
                                                &*metrics,
                                                STAKE_TABLE_CAPACITY_FOR_TEST,
                                                consumer,
                                                bind_version,
                                                upgrades_map,
                                            )
                                            .await)
                                    }
                                    .boxed()
                                })
                                .await
                                .unwrap()
                            } else {
                                cfg.init_node(
                                    i,
                                    state,
                                    persistence,
                                    Some(state_peers),
                                    None,
                                    &NoMetrics,
                                    STAKE_TABLE_CAPACITY_FOR_TEST,
                                    NullEventConsumer,
                                    bind_version,
                                    upgrades_map,
                                )
                                .await
                            }
                        }
                    }),
            )
            .await;

            let handle_0 = &nodes[0];

            // Hook the builder(s) up to the event stream from the first node
            for builder_task in builder_tasks {
                builder_task.start(Box::new(handle_0.event_stream().await));
            }

            for ctx in &nodes {
                ctx.start_consensus().await;
            }

            let server = nodes.remove(0);
            let peers = nodes;

            Self {
                server,
                peers,
                cfg: cfg.network_config,
                temp_dir,
            }
        }

        pub async fn stop_consensus(&mut self) {
            self.server.shutdown_consensus().await;

            for ctx in &mut self.peers {
                ctx.shutdown_consensus().await;
            }
        }
    }

    /// Test the status API with custom options.
    ///
    /// The `opt` function can be used to modify the [`Options`] which are used to start the server.
    /// By default, the options are the minimal required to run this test (configuring a port and
    /// enabling the status API). `opt` may add additional functionality (e.g. adding a query module
    /// to test a different initialization path) but should not remove or modify the existing
    /// functionality (e.g. removing the status module or changing the port).
    pub async fn status_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_test();

        let port = pick_unused_port().expect("No ports free");
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

        let options = opt(Options::with_port(port));
        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let _network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        client.connect(None).await;

        // The status API is well tested in the query service repo. Here we are just smoke testing
        // that we set it up correctly. Wait for a (non-genesis) block to be sequenced and then
        // check the success rate metrics.
        while client
            .get::<u64>("status/block-height")
            .send()
            .await
            .unwrap()
            <= 1
        {
            sleep(Duration::from_secs(1)).await;
        }
        let success_rate = client
            .get::<f64>("status/success-rate")
            .send()
            .await
            .unwrap();
        // If metrics are populating correctly, we should get a finite number. If not, we might get
        // NaN or infinity due to division by 0.
        assert!(success_rate.is_finite(), "{success_rate}");
        // We know at least some views have been successful, since we finalized a block.
        assert!(success_rate > 0.0, "{success_rate}");
    }

    /// Test the submit API with custom options.
    ///
    /// The `opt` function can be used to modify the [`Options`] which are used to start the server.
    /// By default, the options are the minimal required to run this test (configuring a port and
    /// enabling the submit API). `opt` may add additional functionality (e.g. adding a query module
    /// to test a different initialization path) but should not remove or modify the existing
    /// functionality (e.g. removing the submit module or changing the port).
    pub async fn submit_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_test();

        let txn = Transaction::new(NamespaceId::from(1_u32), vec![1, 2, 3, 4]);

        let port = pick_unused_port().expect("No ports free");

        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

        let options = opt(Options::with_port(port).submit(Default::default()));
        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        let mut events = network.server.event_stream().await;

        client.connect(None).await;

        let hash = client
            .post("submit/submit")
            .body_json(&txn)
            .unwrap()
            .send()
            .await
            .unwrap();
        assert_eq!(txn.commit(), hash);

        // Wait for a Decide event containing transaction matching the one we sent
        wait_for_decide_on_handle(&mut events, &txn).await;
    }

    /// Test the state signature API.
    pub async fn state_signature_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_test();

        let port = pick_unused_port().expect("No ports free");

        let url = format!("http://localhost:{port}").parse().unwrap();

        let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

        let options = opt(Options::with_port(port));
        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        let mut height: u64;
        // Wait for block >=2 appears
        // It's waiting for an extra second to make sure that the signature is generated
        loop {
            height = network.server.decided_leaf().await.height();
            sleep(std::time::Duration::from_secs(1)).await;
            if height >= 2 {
                break;
            }
        }
        // we cannot verify the signature now, because we don't know the stake table
        client
            .get::<StateSignatureRequestBody>(&format!("state-signature/block/{}", height))
            .send()
            .await
            .unwrap();
    }

    /// Test the catchup API with custom options.
    ///
    /// The `opt` function can be used to modify the [`Options`] which are used to start the server.
    /// By default, the options are the minimal required to run this test (configuring a port and
    /// enabling the catchup API). `opt` may add additional functionality (e.g. adding a query module
    /// to test a different initialization path) but should not remove or modify the existing
    /// functionality (e.g. removing the catchup module or changing the port).
    pub async fn catchup_test_helper(opt: impl FnOnce(Options) -> Options) {
        setup_test();

        let port = pick_unused_port().expect("No ports free");
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);

        let options = opt(Options::with_port(port));
        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        client.connect(None).await;

        // Wait for a few blocks to be decided.
        let mut events = network.server.event_stream().await;
        loop {
            if let Event {
                event: EventType::Decide { leaf_chain, .. },
                ..
            } = events.next().await.unwrap()
            {
                if leaf_chain
                    .iter()
                    .any(|LeafInfo { leaf, .. }| leaf.block_header().height() > 2)
                {
                    break;
                }
            }
        }

        // Stop consensus running on the node so we freeze the decided and undecided states.
        // We'll let it go out of scope here since it's a write lock.
        {
            network.server.shutdown_consensus().await;
        }

        // Undecided fee state: absent account.
        let leaf = network.server.decided_leaf().await;
        let height = leaf.height() + 1;
        let view = leaf.view_number() + 1;
        let res = client
            .get::<AccountQueryData>(&format!(
                "catchup/{height}/{}/account/{:x}",
                view.u64(),
                Address::default()
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(res.balance, U256::ZERO);
        assert_eq!(
            res.proof
                .verify(
                    &network
                        .server
                        .state(view)
                        .await
                        .unwrap()
                        .fee_merkle_tree
                        .commitment()
                )
                .unwrap(),
            U256::ZERO,
        );

        // Undecided block state.
        let res = client
            .get::<BlocksFrontier>(&format!("catchup/{height}/{}/blocks", view.u64()))
            .send()
            .await
            .unwrap();
        let root = &network
            .server
            .state(view)
            .await
            .unwrap()
            .block_merkle_tree
            .commitment();
        BlockMerkleTree::verify(root.digest(), root.size() - 1, res)
            .unwrap()
            .unwrap();
    }

    pub async fn spawn_dishonest_peer_catchup_api() -> anyhow::Result<(Url, JoinHandle<()>)> {
        let toml = toml::from_str::<toml::Value>(include_str!("../api/catchup.toml")).unwrap();
        let mut api =
            Api::<(), hotshot_query_service::Error, SequencerApiVersion>::new(toml).unwrap();

        api.get("account", |_req, _state: &()| {
            async move {
                Result::<AccountQueryData, _>::Err(hotshot_query_service::Error::catch_all(
                    StatusCode::BAD_REQUEST,
                    "no account found".to_string(),
                ))
            }
            .boxed()
        })?
        .get("blocks", |_req, _state| {
            async move {
                Result::<BlocksFrontier, _>::Err(hotshot_query_service::Error::catch_all(
                    StatusCode::BAD_REQUEST,
                    "no block found".to_string(),
                ))
            }
            .boxed()
        })?
        .get("chainconfig", |_req, _state| {
            async move {
                Result::<ChainConfig, _>::Ok(ChainConfig {
                    max_block_size: 300.into(),
                    base_fee: 1.into(),
                    fee_recipient: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"
                        .parse()
                        .unwrap(),
                    ..Default::default()
                })
            }
            .boxed()
        })?
        .get("leafchain", |_req, _state| {
            async move {
                Result::<Vec<Leaf2>, _>::Err(hotshot_query_service::Error::catch_all(
                    StatusCode::BAD_REQUEST,
                    "No leafchain found".to_string(),
                ))
            }
            .boxed()
        })?;

        let mut app = App::<_, hotshot_query_service::Error>::with_state(());
        app.with_version(env!("CARGO_PKG_VERSION").parse().unwrap());

        app.register_module::<_, _>("catchup", api).unwrap();

        let port = pick_unused_port().expect("no free port");
        let url: Url = Url::parse(&format!("http://localhost:{port}")).unwrap();

        let handle = spawn({
            let url = url.clone();
            async move {
                let _ = app.serve(url, SequencerApiVersion::instance()).await;
            }
        });

        Ok((url, handle))
    }
}

#[cfg(test)]
#[espresso_macros::generic_tests]
mod api_tests {
    use std::fmt::Debug;

    use committable::Committable;
    use data_source::testing::TestableSequencerDataSource;
    use espresso_types::{
        traits::{EventConsumer, PersistenceOptions},
        Header, Leaf2, MockSequencerVersions, NamespaceId, NamespaceProofQueryData, ValidatedState,
    };
    use futures::{future, stream::StreamExt};
    use hotshot_example_types::node_types::{EpochsTestVersions, TestVersions};
    use hotshot_query_service::availability::{
        AvailabilityDataSource, BlockQueryData, StateCertQueryData, VidCommonQueryData,
    };
    use hotshot_types::{
        data::{
            ns_table::parse_ns_table, vid_disperse::VidDisperseShare2, DaProposal2, EpochNumber,
            QuorumProposal2, QuorumProposalWrapper, VidCommitment,
        },
        event::LeafInfo,
        message::Proposal,
        simple_certificate::QuorumCertificate2,
        traits::{node_implementation::ConsensusTime, signature_key::SignatureKey, EncodeBytes},
        utils::EpochTransitionIndicator,
        vid::avidm::{init_avidm_param, AvidMScheme},
    };
    use portpicker::pick_unused_port;
    use sequencer_utils::test_utils::setup_test;
    use surf_disco::Client;
    use test_helpers::{
        catchup_test_helper, state_signature_test_helper, status_test_helper, submit_test_helper,
        TestNetwork, TestNetworkConfigBuilder,
    };
    use tide_disco::error::ServerError;
    use vbs::version::StaticVersion;

    use super::{update::ApiEventConsumer, *};
    use crate::{
        network,
        persistence::no_storage::NoStorage,
        testing::{wait_for_decide_on_handle, TestConfigBuilder},
    };

    #[tokio::test(flavor = "multi_thread")]
    pub(crate) async fn submit_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        submit_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[tokio::test(flavor = "multi_thread")]
    pub(crate) async fn status_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        status_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[tokio::test(flavor = "multi_thread")]
    pub(crate) async fn state_signature_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        state_signature_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[tokio::test(flavor = "multi_thread")]
    pub(crate) async fn test_namespace_query<D: TestableSequencerDataSource>() {
        setup_test();

        // Arbitrary transaction, arbitrary namespace ID
        let ns_id = NamespaceId::from(42_u32);
        let txn = Transaction::new(ns_id, vec![1, 2, 3, 4]);

        // Start query service.
        let port = pick_unused_port().expect("No ports free");
        let storage = D::create_storage().await;
        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(D::options(&storage, Options::with_port(port)).submit(Default::default()))
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        let mut events = network.server.event_stream().await;

        // Connect client.
        let client: Client<ServerError, StaticVersion<0, 1>> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        client.connect(None).await;

        let hash = client
            .post("submit/submit")
            .body_json(&txn)
            .unwrap()
            .send()
            .await
            .unwrap();
        assert_eq!(txn.commit(), hash);

        // Wait for a Decide event containing transaction matching the one we sent
        let block_height = wait_for_decide_on_handle(&mut events, &txn).await as usize;
        tracing::info!(block_height, "transaction sequenced");

        // Wait for the query service to update to this block height.
        client
            .socket(&format!("availability/stream/blocks/{block_height}"))
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();

        let mut found_txn = false;
        let mut found_empty_block = false;
        for block_num in 0..=block_height {
            let header: Header = client
                .get(&format!("availability/header/{block_num}"))
                .send()
                .await
                .unwrap();
            let ns_query_res: NamespaceProofQueryData = client
                .get(&format!("availability/block/{block_num}/namespace/{ns_id}"))
                .send()
                .await
                .unwrap();

            // Verify namespace proof if present
            if let Some(ns_proof) = ns_query_res.proof {
                let vid_common: VidCommonQueryData<SeqTypes> = client
                    .get(&format!("availability/vid/common/{block_num}"))
                    .send()
                    .await
                    .unwrap();
                ns_proof
                    .verify(
                        header.ns_table(),
                        &header.payload_commitment(),
                        vid_common.common(),
                    )
                    .unwrap();
            } else {
                // Namespace proof should be present if ns_id exists in ns_table
                assert!(header.ns_table().find_ns_id(&ns_id).is_none());
                assert!(ns_query_res.transactions.is_empty());
            }

            found_empty_block = found_empty_block || ns_query_res.transactions.is_empty();

            for txn in ns_query_res.transactions {
                if txn.commit() == hash {
                    // Ensure that we validate an inclusion proof
                    found_txn = true;
                }
            }
        }
        assert!(found_txn);
        assert!(found_empty_block);
    }

    #[tokio::test(flavor = "multi_thread")]
    pub(crate) async fn catchup_test_with_query_module<D: TestableSequencerDataSource>() {
        let storage = D::create_storage().await;
        catchup_test_helper(|opt| D::options(&storage, opt)).await
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_non_consecutive_decide_with_failing_event_consumer<D>()
    where
        D: TestableSequencerDataSource + Debug + 'static,
    {
        #[derive(Clone, Copy, Debug)]
        struct FailConsumer;

        #[async_trait]
        impl EventConsumer for FailConsumer {
            async fn handle_event(&self, _: &Event<SeqTypes>) -> anyhow::Result<()> {
                bail!("mock error injection");
            }
        }

        setup_test();
        let (pubkey, privkey) = PubKey::generated_from_seed_indexed([0; 32], 1);

        let storage = D::create_storage().await;
        let persistence = D::persistence_options(&storage).create().await.unwrap();
        let data_source: Arc<StorageState<network::Memory, NoStorage, _, MockSequencerVersions>> =
            Arc::new(StorageState::new(
                D::create(D::persistence_options(&storage), Default::default(), false)
                    .await
                    .unwrap(),
                ApiState::new(future::pending()),
            ));

        // Create two non-consecutive leaf chains.
        let mut chain1 = vec![];

        let genesis = Leaf2::genesis::<TestVersions>(&Default::default(), &NodeState::mock()).await;
        let payload = genesis.block_payload().unwrap();
        let payload_bytes_arc = payload.encode();

        let avidm_param = init_avidm_param(2).unwrap();
        let weights = vec![1u32; 2];

        let ns_table = parse_ns_table(payload.byte_len().as_usize(), &payload.ns_table().encode());
        let (payload_commitment, shares) =
            AvidMScheme::ns_disperse(&avidm_param, &weights, &payload_bytes_arc, ns_table).unwrap();

        let mut quorum_proposal = QuorumProposalWrapper::<SeqTypes> {
            proposal: QuorumProposal2::<SeqTypes> {
                block_header: genesis.block_header().clone(),
                view_number: ViewNumber::genesis(),
                justify_qc: QuorumCertificate2::genesis::<MockSequencerVersions>(
                    &ValidatedState::default(),
                    &NodeState::mock(),
                )
                .await,
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                epoch: None,
                state_cert: None,
            },
        };
        let mut qc = QuorumCertificate2::genesis::<MockSequencerVersions>(
            &ValidatedState::default(),
            &NodeState::mock(),
        )
        .await;

        let mut justify_qc = qc.clone();
        for i in 0..5 {
            *quorum_proposal.proposal.block_header.height_mut() = i;
            quorum_proposal.proposal.view_number = ViewNumber::new(i);
            quorum_proposal.proposal.justify_qc = justify_qc;
            let leaf = Leaf2::from_quorum_proposal(&quorum_proposal);
            qc.view_number = leaf.view_number();
            qc.data.leaf_commit = Committable::commit(&leaf);
            justify_qc = qc.clone();
            chain1.push((leaf.clone(), qc.clone()));

            // Include a quorum proposal for each leaf.
            let quorum_proposal_signature =
                PubKey::sign(&privkey, &bincode::serialize(&quorum_proposal).unwrap())
                    .expect("Failed to sign quorum_proposal");
            persistence
                .append_quorum_proposal2(&Proposal {
                    data: quorum_proposal.clone(),
                    signature: quorum_proposal_signature,
                    _pd: Default::default(),
                })
                .await
                .unwrap();

            // Include VID information for each leaf.
            let share = VidDisperseShare2::<SeqTypes> {
                view_number: leaf.view_number(),
                payload_commitment,
                share: shares[0].clone(),
                recipient_key: pubkey,
                epoch: Some(EpochNumber::new(0)),
                target_epoch: Some(EpochNumber::new(0)),
                common: avidm_param.clone(),
            };
            persistence
                .append_vid2(&share.to_proposal(&privkey).unwrap())
                .await
                .unwrap();

            // Include payload information for each leaf.
            let block_payload_signature =
                PubKey::sign(&privkey, &payload_bytes_arc).expect("Failed to sign block payload");
            let da_proposal_inner = DaProposal2::<SeqTypes> {
                encoded_transactions: payload_bytes_arc.clone(),
                metadata: payload.ns_table().clone(),
                view_number: leaf.view_number(),
                epoch: Some(EpochNumber::new(0)),
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            };
            let da_proposal = Proposal {
                data: da_proposal_inner,
                signature: block_payload_signature,
                _pd: Default::default(),
            };
            persistence
                .append_da2(&da_proposal, VidCommitment::V1(payload_commitment))
                .await
                .unwrap();
        }
        // Split into two chains.
        let mut chain2 = chain1.split_off(2);
        // Make non-consecutive (i.e. we skip a leaf).
        chain2.remove(0);

        // Decide 2 leaves, but fail in event processing.
        let leaf_chain = chain1
            .iter()
            .map(|(leaf, qc)| (leaf_info(leaf.clone()), qc.clone()))
            .collect::<Vec<_>>();
        tracing::info!("decide with event handling failure");
        persistence
            .append_decided_leaves(
                ViewNumber::new(1),
                leaf_chain.iter().map(|(leaf, qc)| (leaf, qc.clone())),
                &FailConsumer,
            )
            .await
            .unwrap();

        // Now decide remaining leaves successfully. We should now process a decide event for all
        // the leaves.
        let consumer = ApiEventConsumer::from(data_source.clone());
        let leaf_chain = chain2
            .iter()
            .map(|(leaf, qc)| (leaf_info(leaf.clone()), qc.clone()))
            .collect::<Vec<_>>();
        tracing::info!("decide successfully");
        persistence
            .append_decided_leaves(
                ViewNumber::new(4),
                leaf_chain.iter().map(|(leaf, qc)| (leaf, qc.clone())),
                &consumer,
            )
            .await
            .unwrap();

        // Check that the leaves were moved to archive storage, along with payload and VID
        // information.
        for (leaf, qc) in chain1.iter().chain(&chain2) {
            tracing::info!(height = leaf.height(), "check archive");
            let qd = data_source.get_leaf(leaf.height() as usize).await.await;
            let stored_leaf: Leaf2 = qd.leaf().clone();
            let stored_qc = qd.qc().clone();
            assert_eq!(&stored_leaf, leaf);
            assert_eq!(&stored_qc, qc);

            data_source
                .get_block(leaf.height() as usize)
                .await
                .try_resolve()
                .ok()
                .unwrap();
            data_source
                .get_vid_common(leaf.height() as usize)
                .await
                .try_resolve()
                .ok()
                .unwrap();

            // Check that all data has been garbage collected for the decided views.
            assert!(persistence
                .load_da_proposal(leaf.view_number())
                .await
                .unwrap()
                .is_none());
            assert!(persistence
                .load_vid_share(leaf.view_number())
                .await
                .unwrap()
                .is_none());
            assert!(persistence
                .load_quorum_proposal(leaf.view_number())
                .await
                .is_err());
        }

        // Check that data has _not_ been garbage collected for the missing view.
        assert!(persistence
            .load_da_proposal(ViewNumber::new(2))
            .await
            .unwrap()
            .is_some());
        assert!(persistence
            .load_vid_share(ViewNumber::new(2))
            .await
            .unwrap()
            .is_some());
        persistence
            .load_quorum_proposal(ViewNumber::new(2))
            .await
            .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    pub async fn test_decide_missing_data<D>()
    where
        D: TestableSequencerDataSource + Debug + 'static,
    {
        setup_test();

        let storage = D::create_storage().await;
        let persistence = D::persistence_options(&storage).create().await.unwrap();
        let data_source: Arc<StorageState<network::Memory, NoStorage, _, MockSequencerVersions>> =
            Arc::new(StorageState::new(
                D::create(D::persistence_options(&storage), Default::default(), false)
                    .await
                    .unwrap(),
                ApiState::new(future::pending()),
            ));
        let consumer = ApiEventConsumer::from(data_source.clone());

        let mut qc = QuorumCertificate2::genesis::<MockSequencerVersions>(
            &ValidatedState::default(),
            &NodeState::mock(),
        )
        .await;
        let leaf =
            Leaf2::genesis::<TestVersions>(&ValidatedState::default(), &NodeState::mock()).await;

        // Append the genesis leaf. We don't use this for the test, because the update function will
        // automatically fill in the missing data for genesis. We just append this to get into a
        // consistent state to then append the leaf from view 1, which will have missing data.
        tracing::info!(?leaf, ?qc, "decide genesis leaf");
        persistence
            .append_decided_leaves(
                leaf.view_number(),
                [(&leaf_info(leaf.clone()), qc.clone())],
                &consumer,
            )
            .await
            .unwrap();

        // Create another leaf, with missing data.
        let mut block_header = leaf.block_header().clone();
        *block_header.height_mut() += 1;
        let qp = QuorumProposalWrapper {
            proposal: QuorumProposal2 {
                block_header,
                view_number: leaf.view_number() + 1,
                justify_qc: qc.clone(),
                upgrade_certificate: None,
                view_change_evidence: None,
                next_drb_result: None,
                next_epoch_justify_qc: None,
                epoch: None,
                state_cert: None,
            },
        };

        let leaf = Leaf2::from_quorum_proposal(&qp);
        qc.view_number = leaf.view_number();
        qc.data.leaf_commit = Committable::commit(&leaf);

        // Decide a leaf without the corresponding payload or VID.
        tracing::info!(?leaf, ?qc, "append leaf 1");
        persistence
            .append_decided_leaves(
                leaf.view_number(),
                [(&leaf_info(leaf.clone()), qc)],
                &consumer,
            )
            .await
            .unwrap();

        // Check that we still processed the leaf.
        assert_eq!(leaf, data_source.get_leaf(1).await.await.leaf().clone());
        assert!(data_source.get_vid_common(1).await.is_pending());
        assert!(data_source.get_block(1).await.is_pending());
    }

    fn leaf_info(leaf: Leaf2) -> LeafInfo<SeqTypes> {
        LeafInfo {
            leaf,
            vid_share: None,
            state: Default::default(),
            delta: None,
            state_cert: None,
        }
    }

    #[ignore]
    #[tokio::test(flavor = "multi_thread")]
    pub(crate) async fn test_state_cert_query<D: TestableSequencerDataSource>() {
        setup_test();

        const TEST_EPOCH_HEIGHT: u64 = 10;
        const TEST_EPOCHS: u64 = 3;

        // Start query service.
        let port = pick_unused_port().expect("No ports free");
        let storage = D::create_storage().await;
        let network_config = TestConfigBuilder::default()
            .epoch_height(TEST_EPOCH_HEIGHT)
            .build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(D::options(&storage, Options::with_port(port)).submit(Default::default()))
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, EpochsTestVersions {}).await;
        let mut events = network.server.event_stream().await;

        // Wait until 3 epochs have passed.
        loop {
            let event = events.next().await.unwrap();
            tracing::info!("Received event from handle: {event:?}");

            if let hotshot::types::EventType::Decide { leaf_chain, .. } = event.event {
                println!(
                    "Decide event received: {:?}",
                    leaf_chain.first().unwrap().leaf.height()
                );
                if leaf_chain
                    .first()
                    .is_some_and(|leaf| leaf.leaf.height() >= TEST_EPOCHS * TEST_EPOCH_HEIGHT)
                {
                    break;
                } else {
                    // Keep waiting
                }
            }
        }

        // Connect client.
        let client: Client<ServerError, StaticVersion<0, 1>> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        client.connect(None).await;

        // Get the state cert for the 3rd epoch.
        for i in 0..TEST_EPOCHS {
            let state_cert = client
                .get::<StateCertQueryData<SeqTypes>>(&format!("availability/state-cert/{i}"))
                .send()
                .await
                .unwrap()
                .0;
            tracing::info!("state_cert: {:?}", state_cert);
            assert_eq!(state_cert.epoch.u64(), i);
            assert_eq!(
                state_cert.light_client_state.block_height,
                (i + 1) * TEST_EPOCH_HEIGHT - 5
            );
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        collections::{HashMap, HashSet},
        time::Duration,
    };

    use alloy::primitives::U256;
    use committable::{Commitment, Committable};
    use espresso_types::{
        config::PublicHotShotConfig,
        traits::NullEventConsumer,
        v0_1::{block_reward, RewardAmount, COMMISSION_BASIS_POINTS},
        v0_3::StakeTableFetcher,
        validators_from_l1_events, EpochVersion, FeeAmount, FeeVersion, Header, L1ClientOptions,
        MockSequencerVersions, SequencerVersions, ValidatedState,
    };
    use futures::{
        future::{self, join_all},
        stream::{StreamExt, TryStreamExt},
    };
    use hotshot::types::EventType;
    use hotshot_example_types::node_types::EpochsTestVersions;
    use hotshot_query_service::{
        availability::{BlockQueryData, LeafQueryData, VidCommonQueryData},
        data_source::{sql::Config, storage::SqlStorage, VersionedDataSource},
        types::HeightIndexed,
    };
    use hotshot_types::{
        data::EpochNumber,
        event::LeafInfo,
        traits::{election::Membership, metrics::NoMetrics, node_implementation::ConsensusTime},
        utils::epoch_from_block_number,
        ValidatorConfig,
    };
    use jf_merkle_tree::prelude::{MerkleProof, Sha3Node};
    use portpicker::pick_unused_port;
    use rand::seq::SliceRandom;
    use sequencer_utils::test_utils::setup_test;
    use staking_cli::demo::DelegationConfig;
    use surf_disco::Client;
    use test_helpers::{
        catchup_test_helper, state_signature_test_helper, status_test_helper, submit_test_helper,
        TestNetwork, TestNetworkConfigBuilder,
    };
    use tide_disco::{app::AppHealth, error::ServerError, healthcheck::HealthStatus};
    use tokio::time::sleep;
    use vbs::version::{StaticVersion, StaticVersionType};

    use self::{
        data_source::testing::TestableSequencerDataSource, options::HotshotEvents,
        sql::DataSource as SqlDataSource,
    };
    use super::*;
    use crate::{
        api::{
            options::Query,
            sql::{impl_testable_data_source::tmp_options, reconstruct_state},
        },
        catchup::{NullStateCatchup, StatePeers},
        persistence::no_storage,
        testing::{TestConfig, TestConfigBuilder},
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn test_healthcheck() {
        setup_test();

        let port = pick_unused_port().expect("No ports free");
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url);
        let options = Options::with_port(port);
        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::<5, _, NullStateCatchup>::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let _network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        client.connect(None).await;
        let health = client.get::<AppHealth>("healthcheck").send().await.unwrap();
        assert_eq!(health.status, HealthStatus::Available);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn status_test_without_query_module() {
        status_test_helper(|opt| opt).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn submit_test_without_query_module() {
        submit_test_helper(|opt| opt).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn state_signature_test_without_query_module() {
        state_signature_test_helper(|opt| opt).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn catchup_test_without_query_module() {
        catchup_test_helper(|opt| opt).await
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn slow_test_merklized_state_api() {
        setup_test();

        let port = pick_unused_port().expect("No ports free");

        let storage = SqlDataSource::create_storage().await;

        let options = SqlDataSource::options(&storage, Options::with_port(port));

        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let mut network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerApiVersion> = Client::new(url);

        client.connect(Some(Duration::from_secs(15))).await;

        // Wait until some blocks have been decided.
        tracing::info!("waiting for blocks");
        let blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(4)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        // sleep for few seconds so that state data is upserted
        tracing::info!("waiting for state to be inserted");
        sleep(Duration::from_secs(5)).await;
        network.stop_consensus().await;

        for block in blocks {
            let i = block.height();
            tracing::info!(i, "get block state");
            let path = client
                .get::<MerkleProof<Commitment<Header>, u64, Sha3Node, 3>>(&format!(
                    "block-state/{}/{i}",
                    i + 1
                ))
                .send()
                .await
                .unwrap();
            assert_eq!(*path.elem().unwrap(), block.hash());

            tracing::info!(i, "get fee state");
            let account = TestConfig::<5>::builder_key().fee_account();
            let path = client
                .get::<MerkleProof<FeeAmount, FeeAccount, Sha3Node, 256>>(&format!(
                    "fee-state/{}/{}",
                    i + 1,
                    account
                ))
                .send()
                .await
                .unwrap();
            assert_eq!(*path.index(), account);
            assert!(*path.elem().unwrap() > 0.into(), "{:?}", path.elem());
        }

        // testing fee_balance api
        let account = TestConfig::<5>::builder_key().fee_account();
        let amount = client
            .get::<Option<FeeAmount>>(&format!("fee-state/fee-balance/latest/{}", account))
            .send()
            .await
            .unwrap()
            .unwrap();
        let expected = U256::MAX;
        assert_eq!(expected, amount.0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_leaf_only_data_source() {
        setup_test();

        let port = pick_unused_port().expect("No ports free");

        let storage = SqlDataSource::create_storage().await;
        let options =
            SqlDataSource::leaf_only_ds_options(&storage, Options::with_port(port)).unwrap();

        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let _network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        let url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, SequencerApiVersion> = Client::new(url);

        tracing::info!("waiting for blocks");
        client.connect(Some(Duration::from_secs(15))).await;
        // Wait until some blocks have been decided.

        let account = TestConfig::<5>::builder_key().fee_account();

        let _headers = client
            .socket("availability/stream/headers/0")
            .subscribe::<Header>()
            .await
            .unwrap()
            .take(10)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        for i in 1..5 {
            let leaf = client
                .get::<LeafQueryData<SeqTypes>>(&format!("availability/leaf/{i}"))
                .send()
                .await
                .unwrap();

            assert_eq!(leaf.height(), i);

            let header = client
                .get::<Header>(&format!("availability/header/{i}"))
                .send()
                .await
                .unwrap();

            assert_eq!(header.height(), i);

            let vid = client
                .get::<VidCommonQueryData<SeqTypes>>(&format!("availability/vid/common/{i}"))
                .send()
                .await
                .unwrap();

            assert_eq!(vid.height(), i);

            client
                .get::<MerkleProof<Commitment<Header>, u64, Sha3Node, 3>>(&format!(
                    "block-state/{i}/{}",
                    i - 1
                ))
                .send()
                .await
                .unwrap();

            client
                .get::<MerkleProof<FeeAmount, FeeAccount, Sha3Node, 256>>(&format!(
                    "fee-state/{}/{}",
                    i + 1,
                    account
                ))
                .send()
                .await
                .unwrap();
        }

        // This would fail even though we have processed atleast 10 leaves
        // this is because light weight nodes only support leaves, headers and VID
        client
            .get::<BlockQueryData<SeqTypes>>("availability/block/1")
            .send()
            .await
            .unwrap_err();
    }

    async fn run_catchup_test(url_suffix: &str) {
        setup_test();

        // Start a sequencer network, using the query service for catchup.
        let port = pick_unused_port().expect("No ports free");
        const NUM_NODES: usize = 5;

        let url: url::Url = format!("http://localhost:{port}{url_suffix}")
            .parse()
            .unwrap();

        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(Options::with_port(port))
            .network_config(TestConfigBuilder::default().build())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![url.clone()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .build();
        let mut network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        // Wait for replica 0 to reach a (non-genesis) decide, before disconnecting it.
        let mut events = network.peers[0].event_stream().await;
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            if leaf_chain[0].leaf.height() > 0 {
                break;
            }
        }

        // Shut down and restart replica 0. We don't just stop consensus and restart it; we fully
        // drop the node and recreate it so it loses all of its temporary state and starts off from
        // genesis. It should be able to catch up by listening to proposals and then rebuild its
        // state from its peers.
        tracing::info!("shutting down node");
        network.peers.remove(0);

        // Wait for a few blocks to pass while the node is down, so it falls behind.
        network
            .server
            .event_stream()
            .await
            .filter(|event| future::ready(matches!(event.event, EventType::Decide { .. })))
            .take(3)
            .collect::<Vec<_>>()
            .await;

        tracing::info!("restarting node");
        let node = network
            .cfg
            .init_node(
                1,
                ValidatedState::default(),
                no_storage::Options,
                Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![url],
                    Default::default(),
                    &NoMetrics,
                )),
                None,
                &NoMetrics,
                test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                NullEventConsumer,
                MockSequencerVersions::new(),
                Default::default(),
            )
            .await;
        let mut events = node.event_stream().await;

        // Wait for a (non-genesis) block proposed by each node, to prove that the lagging node has
        // caught up and all nodes are in sync.
        let mut proposers = [false; NUM_NODES];
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
                let height = leaf.height();
                let leaf_builder = (leaf.view_number().u64() as usize) % NUM_NODES;
                if height == 0 {
                    continue;
                }

                tracing::info!(
                    "waiting for blocks from {proposers:?}, block {height} is from {leaf_builder}",
                );
                proposers[leaf_builder] = true;
            }

            if proposers.iter().all(|has_proposed| *has_proposed) {
                break;
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_catchup() {
        run_catchup_test("").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_catchup_v0() {
        run_catchup_test("/v0").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_catchup_v1() {
        run_catchup_test("/v1").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_catchup_no_state_peers() {
        setup_test();

        // Start a sequencer network, using the query service for catchup.
        let port = pick_unused_port().expect("No ports free");
        const NUM_NODES: usize = 5;
        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(Options::with_port(port))
            .network_config(TestConfigBuilder::default().build())
            .build();
        let mut network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        // Wait for replica 0 to reach a (non-genesis) decide, before disconnecting it.
        let mut events = network.peers[0].event_stream().await;
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            if leaf_chain[0].leaf.height() > 0 {
                break;
            }
        }

        // Shut down and restart replica 0. We don't just stop consensus and restart it; we fully
        // drop the node and recreate it so it loses all of its temporary state and starts off from
        // genesis. It should be able to catch up by listening to proposals and then rebuild its
        // state from its peers.
        tracing::info!("shutting down node");
        network.peers.remove(0);

        // Wait for a few blocks to pass while the node is down, so it falls behind.
        network
            .server
            .event_stream()
            .await
            .filter(|event| future::ready(matches!(event.event, EventType::Decide { .. })))
            .take(3)
            .collect::<Vec<_>>()
            .await;

        tracing::info!("restarting node");
        let node = network
            .cfg
            .init_node(
                1,
                ValidatedState::default(),
                no_storage::Options,
                None::<NullStateCatchup>,
                None,
                &NoMetrics,
                test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                NullEventConsumer,
                MockSequencerVersions::new(),
                Default::default(),
            )
            .await;
        let mut events = node.event_stream().await;

        // Wait for a (non-genesis) block proposed by each node, to prove that the lagging node has
        // caught up and all nodes are in sync.
        let mut proposers = [false; NUM_NODES];
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
                let height = leaf.height();
                let leaf_builder = (leaf.view_number().u64() as usize) % NUM_NODES;
                if height == 0 {
                    continue;
                }

                tracing::info!(
                    "waiting for blocks from {proposers:?}, block {height} is from {leaf_builder}",
                );
                proposers[leaf_builder] = true;
            }

            if proposers.iter().all(|has_proposed| *has_proposed) {
                break;
            }
        }
    }

    #[ignore]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_catchup_epochs_no_state_peers() {
        setup_test();

        // Start a sequencer network, using the query service for catchup.
        let port = pick_unused_port().expect("No ports free");
        const EPOCH_HEIGHT: u64 = 5;
        let network_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .build();
        const NUM_NODES: usize = 5;
        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(Options::with_port(port))
            .network_config(network_config)
            .build();
        let mut network = TestNetwork::new(config, EpochsTestVersions {}).await;

        // Wait for replica 0 to decide in the third epoch.
        let mut events = network.peers[0].event_stream().await;
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            tracing::error!("got decide height {}", leaf_chain[0].leaf.height());

            if leaf_chain[0].leaf.height() > EPOCH_HEIGHT * 3 {
                tracing::error!("decided past one epoch");
                break;
            }
        }

        // Shut down and restart replica 0. We don't just stop consensus and restart it; we fully
        // drop the node and recreate it so it loses all of its temporary state and starts off from
        // genesis. It should be able to catch up by listening to proposals and then rebuild its
        // state from its peers.
        tracing::info!("shutting down node");
        network.peers.remove(0);

        // Wait for a few blocks to pass while the node is down, so it falls behind.
        network
            .server
            .event_stream()
            .await
            .filter(|event| future::ready(matches!(event.event, EventType::Decide { .. })))
            .take(3)
            .collect::<Vec<_>>()
            .await;

        tracing::error!("restarting node");
        let node = network
            .cfg
            .init_node(
                1,
                ValidatedState::default(),
                no_storage::Options,
                None::<NullStateCatchup>,
                None,
                &NoMetrics,
                test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                NullEventConsumer,
                MockSequencerVersions::new(),
                Default::default(),
            )
            .await;
        let mut events = node.event_stream().await;

        // Wait for a (non-genesis) block proposed by each node, to prove that the lagging node has
        // caught up and all nodes are in sync.
        let mut proposers = [false; NUM_NODES];
        loop {
            let event = events.next().await.unwrap();
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            for LeafInfo { leaf, .. } in leaf_chain.iter().rev() {
                let height = leaf.height();
                let leaf_builder = (leaf.view_number().u64() as usize) % NUM_NODES;
                if height == 0 {
                    continue;
                }

                tracing::info!(
                    "waiting for blocks from {proposers:?}, block {height} is from {leaf_builder}",
                );
                proposers[leaf_builder] = true;
            }

            if proposers.iter().all(|has_proposed| *has_proposed) {
                break;
            }
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_chain_config_from_instance() {
        // This test uses a ValidatedState which only has the default chain config commitment.
        // The NodeState has the full chain config.
        // Both chain config commitments will match, so the ValidatedState should have the full chain config after a non-genesis block is decided.
        setup_test();

        let port = pick_unused_port().expect("No ports free");

        let chain_config: ChainConfig = ChainConfig::default();

        let state = ValidatedState {
            chain_config: chain_config.commit().into(),
            ..Default::default()
        };

        let states = std::array::from_fn(|_| state.clone());

        let config = TestNetworkConfigBuilder::default()
            .api_config(Options::with_port(port))
            .states(states)
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .network_config(TestConfigBuilder::default().build())
            .build();

        let mut network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        // Wait for few blocks to be decided.
        network
            .server
            .event_stream()
            .await
            .filter(|event| future::ready(matches!(event.event, EventType::Decide { .. })))
            .take(3)
            .collect::<Vec<_>>()
            .await;

        for peer in &network.peers {
            let state = peer.consensus().read().await.decided_state().await;

            assert_eq!(state.chain_config.resolve().unwrap(), chain_config)
        }

        network.server.shut_down().await;
        drop(network);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_chain_config_catchup() {
        // This test uses a ValidatedState with a non-default chain config
        // so it will be different from the NodeState chain config used by the TestNetwork.
        // However, for this test to work, at least one node should have a full chain config
        // to allow other nodes to catch up.

        setup_test();

        let port = pick_unused_port().expect("No ports free");

        let cf = ChainConfig {
            max_block_size: 300.into(),
            base_fee: 1.into(),
            ..Default::default()
        };

        // State1 contains only the chain config commitment
        let state1 = ValidatedState {
            chain_config: cf.commit().into(),
            ..Default::default()
        };

        //state 2 contains the full chain config
        let state2 = ValidatedState {
            chain_config: cf.into(),
            ..Default::default()
        };

        let mut states = std::array::from_fn(|_| state1.clone());
        // only one node has the full chain config
        // all the other nodes should do a catchup to get the full chain config from peer 0
        states[0] = state2;

        const NUM_NODES: usize = 5;
        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(Options::from(options::Http {
                port,
                max_connections: None,
            }))
            .states(states)
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .network_config(TestConfigBuilder::default().build())
            .build();

        let mut network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        // Wait for a few blocks to be decided.
        network
            .server
            .event_stream()
            .await
            .filter(|event| future::ready(matches!(event.event, EventType::Decide { .. })))
            .take(3)
            .collect::<Vec<_>>()
            .await;

        for peer in &network.peers {
            let state = peer.consensus().read().await.decided_state().await;

            assert_eq!(state.chain_config.resolve().unwrap(), cf)
        }

        network.server.shut_down().await;
        drop(network);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pos_upgrade_view_based() {
        type PosUpgrade = SequencerVersions<FeeVersion, EpochVersion>;
        test_upgrade_helper::<PosUpgrade>(PosUpgrade::new()).await;
    }

    async fn test_upgrade_helper<V: Versions>(version: V) {
        setup_test();
        // wait this number of views beyond the configured first view
        // before asserting anything.
        let wait_extra_views = 10;
        // Number of nodes running in the test network.
        const NUM_NODES: usize = 5;
        let upgrade_version = <V as Versions>::Upgrade::VERSION;
        let port = pick_unused_port().expect("No ports free");

        let test_config = TestConfigBuilder::default()
            .epoch_height(200)
            .epoch_start_block(321)
            .set_upgrades(upgrade_version)
            .await
            .build();

        let chain_config_upgrade = test_config.get_upgrade_map().chain_config(upgrade_version);
        tracing::debug!(?chain_config_upgrade);

        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(Options::from(options::Http {
                port,
                max_connections: None,
            }))
            .catchups(std::array::from_fn(|_| {
                StatePeers::<SequencerApiVersion>::from_urls(
                    vec![format!("http://localhost:{port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .network_config(test_config)
            .build();

        let mut network = TestNetwork::new(config, version).await;
        let mut events = network.server.event_stream().await;

        // First loop to get an `UpgradeProposal`. Note that the
        // actual upgrade will take several to many subsequent views for
        // voting and finally the actual upgrade.
        let upgrade = loop {
            let event = events.next().await.unwrap();
            match event.event {
                EventType::UpgradeProposal { proposal, .. } => {
                    tracing::info!(?proposal, "proposal");
                    let upgrade = proposal.data.upgrade_proposal;
                    let new_version = upgrade.new_version;
                    tracing::info!(?new_version, "upgrade proposal new version");
                    assert_eq!(new_version, <V as Versions>::Upgrade::VERSION);
                    break upgrade;
                },
                _ => continue,
            }
        };

        let wanted_view = upgrade.new_version_first_view + wait_extra_views;
        // Loop until we get the `new_version_first_view`, then test the upgrade.
        loop {
            let event = events.next().await.unwrap();
            let view_number = event.view_number;

            tracing::debug!(?view_number, ?upgrade.new_version_first_view, "upgrade_new_view");
            if view_number > wanted_view {
                let states: Vec<_> = network
                    .peers
                    .iter()
                    .map(|peer| async { peer.consensus().read().await.decided_state().await })
                    .collect();

                let configs: Option<Vec<ChainConfig>> = join_all(states)
                    .await
                    .iter()
                    .map(|state| state.chain_config.resolve())
                    .collect();

                tracing::debug!(?configs, "`ChainConfig`s for nodes");
                if let Some(configs) = configs {
                    for config in configs {
                        assert_eq!(config, chain_config_upgrade);
                    }
                    break; // if assertion did not panic, the test was successful, so we exit the loop
                }
            }
            sleep(Duration::from_millis(200)).await;
        }

        network.server.shut_down().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    pub(crate) async fn test_restart() {
        setup_test();

        const NUM_NODES: usize = 5;
        // Initialize nodes.
        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        let port = pick_unused_port().unwrap();
        let config = TestNetworkConfigBuilder::default()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(port),
            ))
            .persistences(persistence.clone())
            .network_config(TestConfigBuilder::default().build())
            .build();
        let mut network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        // Connect client.
        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        client.connect(None).await;
        tracing::info!(port, "server running");

        // Wait until some blocks have been decided.
        client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(3)
            .collect::<Vec<_>>()
            .await;

        // Shut down the consensus nodes.
        tracing::info!("shutting down nodes");
        network.stop_consensus().await;

        // Get the block height we reached.
        let height = client
            .get::<usize>("status/block-height")
            .send()
            .await
            .unwrap();
        tracing::info!("decided {height} blocks before shutting down");

        // Get the decided chain, so we can check consistency after the restart.
        let chain: Vec<LeafQueryData<SeqTypes>> = client
            .socket("availability/stream/leaves/0")
            .subscribe()
            .await
            .unwrap()
            .take(height)
            .try_collect()
            .await
            .unwrap();
        let decided_view = chain.last().unwrap().leaf().view_number();

        // Get the most recent state, for catchup.

        let state = network.server.decided_state().await;
        tracing::info!(?decided_view, ?state, "consensus state");

        // Fully shut down the API servers.
        drop(network);

        // Start up again, resuming from the last decided leaf.
        let port = pick_unused_port().expect("No ports free");

        let config = TestNetworkConfigBuilder::default()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(port),
            ))
            .persistences(persistence)
            .catchups(std::array::from_fn(|_| {
                // Catchup using node 0 as a peer. Node 0 was running the archival state service
                // before the restart, so it should be able to resume without catching up by loading
                // state from storage.
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .network_config(TestConfigBuilder::default().build())
            .build();
        let _network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        let client: Client<ServerError, StaticVersion<0, 1>> =
            Client::new(format!("http://localhost:{port}").parse().unwrap());
        client.connect(None).await;
        tracing::info!(port, "server running");

        // Make sure we can decide new blocks after the restart.
        tracing::info!("waiting for decide, height {height}");
        let new_leaf: LeafQueryData<SeqTypes> = client
            .socket(&format!("availability/stream/leaves/{height}"))
            .subscribe()
            .await
            .unwrap()
            .next()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(new_leaf.height(), height as u64);
        assert_eq!(
            new_leaf.leaf().parent_commitment(),
            chain[height - 1].hash()
        );

        // Ensure the new chain is consistent with the old chain.
        let new_chain: Vec<LeafQueryData<SeqTypes>> = client
            .socket("availability/stream/leaves/0")
            .subscribe()
            .await
            .unwrap()
            .take(height)
            .try_collect()
            .await
            .unwrap();
        assert_eq!(chain, new_chain);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_config() {
        setup_test();

        let port = pick_unused_port().expect("No ports free");
        let url: surf_disco::Url = format!("http://localhost:{port}").parse().unwrap();
        let client: Client<ServerError, StaticVersion<0, 1>> = Client::new(url.clone());

        let options = Options::with_port(port).config(Default::default());
        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let network = TestNetwork::new(config, MockSequencerVersions::new()).await;
        client.connect(None).await;

        // Fetch a network config from the API server. The first peer URL is bogus, to test the
        // failure/retry case.
        let peers = StatePeers::<StaticVersion<0, 1>>::from_urls(
            vec!["https://notarealnode.network".parse().unwrap(), url],
            Default::default(),
            &NoMetrics,
        );

        // Fetch the config from node 1, a different node than the one running the service.
        let validator =
            ValidatorConfig::generated_from_seed_indexed([0; 32], 1, U256::from(1), false);
        let config = peers.fetch_config(validator.clone()).await.unwrap();

        // Check the node-specific information in the recovered config is correct.
        assert_eq!(config.node_index, 1);

        // Check the public information is also correct (with respect to the node that actually
        // served the config, for public keys).
        pretty_assertions::assert_eq!(
            serde_json::to_value(PublicHotShotConfig::from(config.config)).unwrap(),
            serde_json::to_value(PublicHotShotConfig::from(
                network.cfg.hotshot_config().clone()
            ))
            .unwrap()
        );
    }

    async fn run_hotshot_event_streaming_test(url_suffix: &str) {
        setup_test();

        let hotshot_event_streaming_port =
            pick_unused_port().expect("No ports free for hotshot event streaming");
        let query_service_port = pick_unused_port().expect("No ports free for query service");

        let url = format!("http://localhost:{hotshot_event_streaming_port}{url_suffix}")
            .parse()
            .unwrap();

        let hotshot_events = HotshotEvents {
            events_service_port: hotshot_event_streaming_port,
        };

        let client: Client<ServerError, SequencerApiVersion> = Client::new(url);

        let options = Options::with_port(query_service_port).hotshot_events(hotshot_events);

        let network_config = TestConfigBuilder::default().build();
        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config)
            .build();
        let _network = TestNetwork::new(config, MockSequencerVersions::new()).await;

        let mut subscribed_events = client
            .socket("hotshot-events/events")
            .subscribe::<Event<SeqTypes>>()
            .await
            .unwrap();

        let total_count = 5;
        // wait for these events to receive on client 1
        let mut receive_count = 0;
        loop {
            let event = subscribed_events.next().await.unwrap();
            tracing::info!(
                "Received event in hotshot event streaming Client 1: {:?}",
                event
            );
            receive_count += 1;
            if receive_count > total_count {
                tracing::info!("Client Received at least desired events, exiting loop");
                break;
            }
        }
        assert_eq!(receive_count, total_count + 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_hotshot_event_streaming_v0() {
        run_hotshot_event_streaming_test("/v0").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_hotshot_event_streaming_v1() {
        run_hotshot_event_streaming_test("/v1").await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_hotshot_event_streaming() {
        run_hotshot_event_streaming_test("").await;
    }

    // TODO when `EpochVersion` becomes base version we can merge this
    // w/ above test.
    #[tokio::test(flavor = "multi_thread")]
    async fn test_hotshot_event_streaming_epoch_progression() {
        setup_test();
        let epoch_height = 35;
        let wanted_epochs = 4;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(epoch_height)
            .build();

        let hotshot_event_streaming_port =
            pick_unused_port().expect("No ports free for hotshot event streaming");
        let hotshot_url = format!("http://localhost:{hotshot_event_streaming_port}")
            .parse()
            .unwrap();

        let query_service_port = pick_unused_port().expect("No ports free for query service");

        let hotshot_events = HotshotEvents {
            events_service_port: hotshot_event_streaming_port,
        };

        let client: Client<ServerError, SequencerApiVersion> = Client::new(hotshot_url);
        let options = Options::with_port(query_service_port).hotshot_events(hotshot_events);

        let config = TestNetworkConfigBuilder::default()
            .api_config(options)
            .network_config(network_config.clone())
            .pos_hook::<PosVersion>(DelegationConfig::VariableAmounts, Default::default())
            .await
            .expect("Pos Deployment")
            .build();

        let _network = TestNetwork::new(config, PosVersion::new()).await;

        let mut subscribed_events = client
            .socket("hotshot-events/events")
            .subscribe::<Event<SeqTypes>>()
            .await
            .unwrap();

        let wanted_views = epoch_height * wanted_epochs;

        let mut views = HashSet::new();
        let mut epochs = HashSet::new();
        for _ in 0..=600 {
            let event = subscribed_events.next().await.unwrap();
            let event = event.unwrap();
            let view_number = event.view_number;
            views.insert(view_number.u64());

            if let hotshot::types::EventType::Decide { qc, .. } = event.event {
                assert!(qc.data.epoch.is_some(), "epochs are live");
                assert!(qc.data.block_number.is_some());

                let epoch = qc.data.epoch.unwrap().u64();
                epochs.insert(epoch);

                tracing::debug!(
                    "Got decide: epoch: {:?}, block: {:?} ",
                    epoch,
                    qc.data.block_number
                );

                let expected_epoch =
                    epoch_from_block_number(qc.data.block_number.unwrap(), epoch_height);
                tracing::debug!("expected epoch: {}, qc epoch: {}", expected_epoch, epoch);

                assert_eq!(expected_epoch, epoch);
            }
            if views.contains(&wanted_views) {
                tracing::info!("Client Received at least desired views, exiting loop");
                break;
            }
        }

        // prevent false positive when we overflow the range
        assert!(views.contains(&wanted_views), "Views are not progressing");
        assert!(
            epochs.contains(&wanted_epochs),
            "Epochs are not progressing"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pos_rewards_basic() -> anyhow::Result<()> {
        // Basic PoS rewards test:
        // - Sets up a single validator and a single delegator (the node itself).
        // - Sets the number of blocks in each epoch to 20.
        // - Rewards begin applying from block 41 (i.e., the start of the 3rd epoch).
        // - Since the validator is also the delegator, it receives the full reward.
        // - Verifies that the reward at block height 60 matches the expected amount.
        setup_test();
        let epoch_height = 20;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(epoch_height)
            .build();

        let api_port = pick_unused_port().expect("No ports free for query service");

        const NUM_NODES: usize = 1;
        // Initialize nodes.
        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config.clone())
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .pos_hook::<PosVersion>(DelegationConfig::VariableAmounts, Default::default())
            .await
            .unwrap()
            .build();

        let _network = TestNetwork::new(config, PosVersion::new()).await;
        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());

        // first two epochs will be 1 and 2
        // rewards are distributed starting third epoch
        // third epoch starts from block 40 as epoch height is 20
        // wait for atleast 65 blocks
        let _blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(65)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let staking_priv_keys = network_config.staking_priv_keys();
        let account = staking_priv_keys[0].0.clone();
        let address = account.address();

        let block_height = 60;

        // get the validator address balance at block height 60
        let amount = client
            .get::<Option<RewardAmount>>(&format!(
                "reward-state/reward-balance/{block_height}/{address}"
            ))
            .send()
            .await
            .unwrap()
            .unwrap();

        tracing::info!("amount={amount:?}");

        let epoch_start_block = 40;
        // The validator gets all the block reward so we can calculate the expected amount
        let expected_amount = block_reward().0 * (U256::from(block_height - epoch_start_block));

        assert_eq!(amount.0, expected_amount, "reward amount don't match");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_cumulative_pos_rewards() -> anyhow::Result<()> {
        // This test registers 5 validators and multiple delegators for each validator.
        // One of the delegators is also a validator.
        // The test verifies that the cumulative reward at each block height equals the total block reward,
        // which is a constant.

        setup_test();
        let epoch_height = 20;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(epoch_height)
            .build();

        let api_port = pick_unused_port().expect("No ports free for query service");

        const NUM_NODES: usize = 5;
        // Initialize nodes.
        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config)
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .pos_hook::<PosVersion>(DelegationConfig::MultipleDelegators, Default::default())
            .await
            .unwrap()
            .build();

        let _network = TestNetwork::new(config, PosVersion::new()).await;
        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());

        // wait for atleast 75 blocks
        let _blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(75)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        // We are going to check cumulative blocks from block height 40 to 67
        // Basically epoch 3 and epoch 4 as epoch height is 20
        // get all the validators
        let validators = client
            .get::<IndexMap<Address, Validator<BLSPubKey>>>("node/validators/3")
            .send()
            .await
            .expect("failed to get validator");

        // insert all the address in a map
        // We will query the reward-balance at each block height for all the addresses
        // We don't know which validator was the leader because we don't have access to Membership
        let mut addresses = HashSet::new();
        for v in validators.values() {
            addresses.insert(v.account);
            addresses.extend(v.clone().delegators.keys().collect::<Vec<_>>());
        }
        // get all the validators
        let validators = client
            .get::<IndexMap<Address, Validator<BLSPubKey>>>("node/validators/4")
            .send()
            .await
            .expect("failed to get validator");
        for v in validators.values() {
            addresses.insert(v.account);
            addresses.extend(v.clone().delegators.keys().collect::<Vec<_>>());
        }

        let mut prev_cumulative_amount = U256::ZERO;
        // Check Cumulative rewards for epoch 3
        // i.e block height 41 to 59
        for block in 41..=67 {
            let mut cumulative_amount = U256::ZERO;
            for address in addresses.clone() {
                let amount = client
                    .get::<Option<RewardAmount>>(&format!(
                        "reward-state/reward-balance/{block}/{address}"
                    ))
                    .send()
                    .await
                    .ok()
                    .flatten();

                if let Some(amount) = amount {
                    tracing::info!("address={address}, amount= {amount}");
                    cumulative_amount += amount.0;
                };
            }

            // assert cumulative reward is equal to block reward
            assert_eq!(cumulative_amount - prev_cumulative_amount, block_reward().0);
            tracing::info!("cumulative_amount is correct for block={block}");
            prev_cumulative_amount = cumulative_amount;
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_stake_table_duplicate_events_from_contract() -> anyhow::Result<()> {
        // TODO(abdul): This test currently uses TestNetwork only for contract deployment and for L1 block number.
        // Once the stake table deployment logic is refactored and isolated, TestNetwork here will be unnecessary

        setup_test();
        let epoch_height = 20;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(epoch_height)
            .build();

        let api_port = pick_unused_port().expect("No ports free for query service");

        const NUM_NODES: usize = 5;
        // Initialize nodes.
        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let l1_url = network_config.l1_url();
        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config)
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .pos_hook::<PosVersion>(DelegationConfig::MultipleDelegators, Default::default())
            .await
            .unwrap()
            .build();

        let network = TestNetwork::new(config, PosVersion::new()).await;

        let mut prev_st = None;
        let state = network.server.decided_state().await;
        let chain_config = state.chain_config.resolve().expect("resolve chain config");
        let stake_table = chain_config.stake_table_contract.unwrap();

        let l1_client = L1ClientOptions::default()
            .connect(vec![l1_url])
            .expect("failed to connect to l1");

        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());

        let mut headers = client
            .socket("availability/stream/headers/0")
            .subscribe::<Header>()
            .await
            .unwrap();

        let mut target_bh = 0;
        while let Some(header) = headers.next().await {
            let header = header.unwrap();
            if header.height() == 0 {
                continue;
            }
            let l1_block = header.l1_finalized().expect("l1 block not found");

            let events = StakeTableFetcher::fetch_events_from_contract(
                l1_client.clone(),
                stake_table,
                None,
                l1_block.number(),
            )
            .await
            .expect("failed to get stake table from contract");
            let sorted_events = events.sort_events().expect("failed to sort");

            let mut sorted_dedup_removed = sorted_events.clone();
            sorted_dedup_removed.dedup();

            assert_eq!(
                sorted_events.len(),
                sorted_dedup_removed.len(),
                "duplicates found"
            );

            // This also checks if there is a duplicate registration
            let stake_table =
                validators_from_l1_events(sorted_events.into_iter().map(|(_, e)| e)).unwrap();
            if let Some(prev_st) = prev_st {
                assert_eq!(stake_table, prev_st);
            }

            prev_st = Some(stake_table);

            if target_bh == 100 {
                break;
            }

            target_bh = header.height();
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_rewards() -> anyhow::Result<()> {
        // The test registers multiple delegators for each validator
        // It verifies that no rewards are distributed in the first two epochs
        // and that rewards are correctly allocated starting from the third epoch.
        // also checks that the total stake of delegators matches the stake of the validator
        // and that the calculated rewards match those obtained via the merklized state api
        setup_test();
        const EPOCH_HEIGHT: u64 = 20;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .build();

        let api_port = pick_unused_port().expect("No ports free for query service");

        const NUM_NODES: usize = 7;

        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config)
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .pos_hook::<PosVersion>(DelegationConfig::MultipleDelegators, Default::default())
            .await
            .unwrap()
            .build();

        let network = TestNetwork::new(config, PosVersion::new()).await;
        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());

        // Wait for 3 epochs to allow rewards distribution to take effect.
        let mut events = network.peers[0].event_stream().await;
        while let Some(event) = events.next().await {
            if let EventType::Decide { leaf_chain, .. } = event.event {
                let height = leaf_chain[0].leaf.height();
                tracing::info!("Node 0 decided at height: {}", height);
                if height > EPOCH_HEIGHT * 3 {
                    break;
                }
            }
        }

        // Verify that there are no validators for epoch # 1 and epoch # 2
        {
            client
                .get::<IndexMap<Address, Validator<BLSPubKey>>>("node/validators/1")
                .send()
                .await
                .unwrap()
                .is_empty();

            client
                .get::<IndexMap<Address, Validator<BLSPubKey>>>("node/validators/2")
                .send()
                .await
                .unwrap()
                .is_empty();
        }

        // Get the epoch # 3 validators
        let validators = client
            .get::<IndexMap<Address, Validator<BLSPubKey>>>("node/validators/3")
            .send()
            .await
            .expect("validators");

        assert!(!validators.is_empty());

        // Collect addresses to track rewards for all participants.
        let mut addresses = HashSet::new();
        for v in validators.values() {
            addresses.insert(v.account);
            addresses.extend(v.clone().delegators.keys().collect::<Vec<_>>());
        }

        // Verify no rewards are distributed in the first two epochs.
        for block in 0..=EPOCH_HEIGHT * 2 {
            for address in addresses.clone() {
                let amount = client
                    .get::<Option<RewardAmount>>(&format!(
                        "reward-state/reward-balance/{block}/{address}"
                    ))
                    .send()
                    .await
                    .ok()
                    .flatten();
                assert!(amount.is_none(), "amount is not none for block {block}")
            }
        }

        // Collect leaves for epoch 3 to 5 to verify reward calculations.
        let leaves = client
            .socket("availability/stream/leaves/41")
            .subscribe::<LeafQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take((EPOCH_HEIGHT * 3).try_into().unwrap())
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        let node_state = network.server.node_state();
        let coordinator = node_state.coordinator;

        let mut rewards_map = HashMap::new();

        for leaf in leaves {
            let block = leaf.height();
            tracing::info!("verify rewards for block={block:?}");
            let membership = coordinator.membership().read().await;
            let epoch = epoch_from_block_number(block, EPOCH_HEIGHT);
            let epoch_number = EpochNumber::new(epoch);
            let leader = membership
                .leader(leaf.leaf().view_number(), Some(epoch_number))
                .expect("leader");
            let leader_eth_address = membership.address(&epoch_number, leader).expect("address");
            drop(membership);

            let validators = client
                .get::<IndexMap<Address, Validator<BLSPubKey>>>(&format!("node/validators/{epoch}"))
                .send()
                .await
                .expect("validators");

            let leader_validator = validators
                .get(&leader_eth_address)
                .expect("leader not found");

            // Verify that the sum of delegator stakes equals the validator's total stake.
            for validator in validators.values() {
                let delegator_stake_sum: U256 = validator.delegators.values().cloned().sum();

                assert_eq!(delegator_stake_sum, validator.stake);
            }

            let computed_rewards = leader_validator
                .compute_rewards()
                .expect("reward computation");

            // Verify that the leader commission amount is within the tolerated range.
            // Due to potential rounding errors in decimal calculations for delegator rewards,
            // the actual distributed commission
            // amount may differ very slightly from the calculated value.
            // this asserts that it is within 10wei tolerance level.
            // 10 wei is 10* 10E-18
            let total_reward = block_reward().0;
            let leader_commission_basis_points = U256::from(leader_validator.commission);
            let calculated_leader_commission_reward = leader_commission_basis_points
                .checked_mul(total_reward)
                .context("overflow")?
                .checked_div(U256::from(COMMISSION_BASIS_POINTS))
                .context("overflow")?;

            assert!(
                computed_rewards.leader_commission().0 - calculated_leader_commission_reward
                    <= U256::from(10_u64)
            );

            // Aggregate reward amounts by address in the map.
            // This is necessary because there can be two entries for a leader address:
            // - One entry for commission rewards.
            // - Another for delegator rewards when the leader is delegating.
            // Also, rewards are accumulated for the same addresses
            let leader_commission = *computed_rewards.leader_commission();
            for (address, amount) in computed_rewards.delegators().clone() {
                rewards_map
                    .entry(address)
                    .and_modify(|entry| *entry += amount)
                    .or_insert(amount);
            }

            // add leader commission reward
            rewards_map
                .entry(leader_eth_address)
                .and_modify(|entry| *entry += leader_commission)
                .or_insert(leader_commission);

            // assert that the reward matches to what is in the reward merkle tree
            for (address, calculated_amount) in rewards_map.iter() {
                let amount_from_api = client
                    .get::<Option<RewardAmount>>(&format!(
                        "reward-state/reward-balance/{block}/{address}"
                    ))
                    .send()
                    .await
                    .ok()
                    .flatten()
                    .expect("amount");
                assert_eq!(amount_from_api, *calculated_amount)
            }
        }

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_node_stake_table_api() {
        setup_test();
        let epoch_height = 20;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(epoch_height)
            .build();

        let api_port = pick_unused_port().expect("No ports free for query service");

        const NUM_NODES: usize = 2;
        // Initialize nodes.
        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config)
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .pos_hook::<PosVersion>(DelegationConfig::MultipleDelegators, Default::default())
            .await
            .unwrap()
            .build();

        let _network = TestNetwork::new(config, PosVersion::new()).await;

        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{api_port}").parse().unwrap());

        // wait for atleast 2 epochs
        let _blocks = client
            .socket("availability/stream/blocks/0")
            .subscribe::<BlockQueryData<SeqTypes>>()
            .await
            .unwrap()
            .take(40)
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        for i in 1..=3 {
            let _st = client
                .get::<Vec<PeerConfig<SeqTypes>>>(&format!("node/stake-table/{}", i as u64))
                .send()
                .await
                .expect("failed to get stake table");
        }

        let _st = client
            .get::<StakeTableWithEpochNumber<SeqTypes>>("node/stake-table/current")
            .send()
            .await
            .expect("failed to get stake table");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_epoch_stake_table_catchup() {
        setup_test();

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        const EPOCH_HEIGHT: u64 = 10;
        const NUM_NODES: usize = 6;

        let port = pick_unused_port().expect("No ports free");

        let network_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .build();

        // Initialize storage for each node
        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;

        let persistence_options: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        // setup catchup peers
        let catchup_peers = std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                &NoMetrics,
            )
        });
        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(port),
            ))
            .network_config(network_config)
            .persistences(persistence_options.clone())
            .catchups(catchup_peers)
            .pos_hook::<PosVersion>(DelegationConfig::MultipleDelegators, Default::default())
            .await
            .unwrap()
            .build();

        let state = config.states()[0].clone();
        let mut network = TestNetwork::new(config, EpochsTestVersions {}).await;

        // Wait for the peer 0 (node 1) to advance past three epochs
        let mut events = network.peers[0].event_stream().await;
        while let Some(event) = events.next().await {
            if let EventType::Decide { leaf_chain, .. } = event.event {
                let height = leaf_chain[0].leaf.height();
                tracing::info!("Node 0 decided at height: {}", height);
                if height > EPOCH_HEIGHT * 3 {
                    break;
                }
            }
        }

        // Shutdown and remove node 1 to simulate falling behind
        tracing::info!("Shutting down peer 0");
        network.peers.remove(0);

        // Wait for epochs to progress with node 1 offline
        let mut events = network.server.event_stream().await;
        while let Some(event) = events.next().await {
            if let EventType::Decide { leaf_chain, .. } = event.event {
                let height = leaf_chain[0].leaf.height();
                tracing::info!("Server decided at height: {}", height);
                //  until 7 epochs
                if height > EPOCH_HEIGHT * 7 {
                    break;
                }
            }
        }

        // add node 1 to the network with fresh storage
        let storage = SqlDataSource::create_storage().await;
        let options = <SqlDataSource as TestableSequencerDataSource>::persistence_options(&storage);

        tracing::info!("Restarting peer 0");
        let node = network
            .cfg
            .init_node(
                1,
                state,
                options,
                Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )),
                None,
                &NoMetrics,
                test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                NullEventConsumer,
                EpochsTestVersions {},
                Default::default(),
            )
            .await;

        let coordinator = node.node_state().coordinator;

        let server_node_state = network.server.node_state();
        let server_coordinator = server_node_state.coordinator;

        // Verify that the restarted node catches up for each epoch
        for epoch_num in 1..=7 {
            println!("getting stake table for epoch = {epoch_num}");
            let epoch = EpochNumber::new(epoch_num);
            let membership_for_epoch = coordinator.membership_for_epoch(Some(epoch)).await;
            if membership_for_epoch.is_err() {
                coordinator.wait_for_catchup(epoch).await.unwrap();
            }

            println!("have stake table for epoch = {epoch_num}");

            let node_stake_table = coordinator
                .membership()
                .read()
                .await
                .stake_table(Some(epoch));
            let stake_table = server_coordinator
                .membership()
                .read()
                .await
                .stake_table(Some(epoch));

            println!("asserting stake table for epoch = {epoch_num}");

            assert_eq!(
                node_stake_table, stake_table,
                "Stake table mismatch for epoch {epoch_num}",
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_epoch_stake_table_catchup_stress() {
        setup_test();

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        const EPOCH_HEIGHT: u64 = 10;
        const NUM_NODES: usize = 6;

        let port = pick_unused_port().expect("No ports free");

        let network_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .build();

        // Initialize storage for each node
        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;

        let persistence_options: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        // setup catchup peers
        let catchup_peers = std::array::from_fn(|_| {
            StatePeers::<StaticVersion<0, 1>>::from_urls(
                vec![format!("http://localhost:{port}").parse().unwrap()],
                Default::default(),
                &NoMetrics,
            )
        });
        let config = TestNetworkConfigBuilder::<NUM_NODES, _, _>::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(port),
            ))
            .network_config(network_config)
            .persistences(persistence_options.clone())
            .catchups(catchup_peers)
            .pos_hook::<PosVersion>(DelegationConfig::MultipleDelegators, Default::default())
            .await
            .unwrap()
            .build();

        let state = config.states()[0].clone();
        let mut network = TestNetwork::new(config, EpochsTestVersions {}).await;

        // Wait for the peer 0 (node 1) to advance past three epochs
        let mut events = network.peers[0].event_stream().await;
        while let Some(event) = events.next().await {
            if let EventType::Decide { leaf_chain, .. } = event.event {
                let height = leaf_chain[0].leaf.height();
                tracing::info!("Node 0 decided at height: {}", height);
                if height > EPOCH_HEIGHT * 3 {
                    break;
                }
            }
        }

        // Shutdown and remove node 1 to simulate falling behind
        tracing::info!("Shutting down peer 0");
        network.peers.remove(0);

        // Wait for epochs to progress with node 1 offline
        let mut events = network.server.event_stream().await;
        while let Some(event) = events.next().await {
            if let EventType::Decide { leaf_chain, .. } = event.event {
                let height = leaf_chain[0].leaf.height();
                tracing::info!("Server decided at height: {}", height);
                //  until 7 epochs
                if height > EPOCH_HEIGHT * 7 {
                    break;
                }
            }
        }

        // add node 1 to the network with fresh storage
        let storage = SqlDataSource::create_storage().await;
        let options = <SqlDataSource as TestableSequencerDataSource>::persistence_options(&storage);

        tracing::info!("Restarting peer 0");
        let node = network
            .cfg
            .init_node(
                1,
                state,
                options,
                Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )),
                None,
                &NoMetrics,
                test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                NullEventConsumer,
                EpochsTestVersions {},
                Default::default(),
            )
            .await;

        let coordinator = node.node_state().coordinator;

        let server_node_state = network.server.node_state();
        let server_coordinator = server_node_state.coordinator;

        // Trigger catchup for all epochs in quick succession and in random order
        let mut rand_epochs: Vec<_> = (1..=7).collect();
        rand_epochs.shuffle(&mut rand::thread_rng());
        println!("trigger catchup in this order: {rand_epochs:?}");
        for epoch_num in rand_epochs {
            let epoch = EpochNumber::new(epoch_num);
            let _ = coordinator.membership_for_epoch(Some(epoch)).await;
        }

        // Verify that the restarted node catches up for each epoch
        for epoch_num in 1..=7 {
            println!("getting stake table for epoch = {epoch_num}");
            let epoch = EpochNumber::new(epoch_num);
            let _ = coordinator.wait_for_catchup(epoch).await.unwrap();

            println!("have stake table for epoch = {epoch_num}");

            let node_stake_table = coordinator
                .membership()
                .read()
                .await
                .stake_table(Some(epoch));
            let stake_table = server_coordinator
                .membership()
                .read()
                .await
                .stake_table(Some(epoch));

            println!("asserting stake table for epoch = {epoch_num}");

            assert_eq!(
                node_stake_table, stake_table,
                "Stake table mismatch for epoch {epoch_num}",
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_merklized_state_catchup_on_restart() -> anyhow::Result<()> {
        // This test verifies that a query node can catch up on
        // merklized state after being offline for multiple epochs.
        //
        // Steps:
        // 1. Start a test network with 5 sequencer nodes.
        // 2. Start a separate node with the query module enabled, connected to the network.
        //    - This node stores merklized state
        // 3. Shut down the query node after 1 epoch.
        // 4. Allow the network to progress 3 more epochs (query node remains offline).
        // 5. Restart the query node.
        //    - The node is expected to reconstruct or catch up on its own
        setup_test();
        const EPOCH_HEIGHT: u64 = 10;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .build();

        let api_port = pick_unused_port().expect("No ports free for query service");

        tracing::info!("API PORT = {api_port}");
        const NUM_NODES: usize = 5;

        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config)
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .pos_hook::<PosVersion>(
                DelegationConfig::MultipleDelegators,
                hotshot_contract_adapter::stake_table::StakeTableContractVersion::V2,
            )
            .await
            .unwrap()
            .build();
        let state = config.states()[0].clone();
        let mut network = TestNetwork::new(config, PosVersion::new()).await;

        // Remove peer 0 and restart it with the query module enabled.
        // Adding an additional node to the test network is not straight forward,
        // as the keys have already been initialized in the config above.
        // So, we remove this node and re-add it using the same index.
        network.peers.remove(0);
        let node_0_storage = &storage[1];
        let node_0_persistence = persistence[1].clone();
        let node_0_port = pick_unused_port().expect("No ports free for query service");
        tracing::info!("node_0_port {node_0_port}");
        // enable query module with api peers
        let opt = Options::with_port(node_0_port).query_sql(
            Query {
                peers: vec![format!("http://localhost:{api_port}").parse().unwrap()],
            },
            tmp_options(node_0_storage),
        );

        // start the query node so that it builds the merklized state
        let node_0 = opt
            .clone()
            .serve(|metrics, consumer, storage| {
                let cfg = network.cfg.clone();
                let node_0_persistence = node_0_persistence.clone();
                let state = state.clone();
                async move {
                    Ok(cfg
                        .init_node(
                            1,
                            state,
                            node_0_persistence.clone(),
                            Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                                Default::default(),
                                &NoMetrics,
                            )),
                            storage,
                            &*metrics,
                            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                            consumer,
                            PosVersion::new(),
                            Default::default(),
                        )
                        .await)
                }
                .boxed()
            })
            .await
            .unwrap();

        let mut events = network.peers[2].event_stream().await;
        // wait for 1 epoch
        wait_for_epochs(&mut events, EPOCH_HEIGHT, 1).await;

        // shutdown the node for 3 epochs
        drop(node_0);

        // wait for 4 epochs
        wait_for_epochs(&mut events, EPOCH_HEIGHT, 4).await;

        // start the node again.
        let node_0 = opt
            .serve(|metrics, consumer, storage| {
                let cfg = network.cfg.clone();
                async move {
                    Ok(cfg
                        .init_node(
                            1,
                            state,
                            node_0_persistence,
                            Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                                Default::default(),
                                &NoMetrics,
                            )),
                            storage,
                            &*metrics,
                            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                            consumer,
                            PosVersion::new(),
                            Default::default(),
                        )
                        .await)
                }
                .boxed()
            })
            .await
            .unwrap();

        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{node_0_port}").parse().unwrap());
        client.connect(None).await;

        let epoch_6_block = EPOCH_HEIGHT * 6 + 1;

        // check that the node's state has reward accounts
        let mut retries = 0;
        loop {
            sleep(Duration::from_secs(1)).await;
            let state = node_0.decided_state().await;
            let reward_accounts = state
                .reward_merkle_tree
                .clone()
                .into_iter()
                .collect::<Vec<_>>();

            if !reward_accounts.is_empty() {
                tracing::info!("Node's state has reward accounts. {reward_accounts:?}");
                break;
            }
            retries += 1;

            if retries > 120 {
                panic!("max retries reached. failed to catchup reward state");
            }
        }

        // check that the node has stored atleast 6 epochs merklized state in persistence
        loop {
            sleep(Duration::from_secs(1)).await;

            let bh = client
                .get::<u64>("block-state/block-height")
                .send()
                .await
                .expect("block height not found");

            tracing::info!("block state: block height={bh}");
            if bh > epoch_6_block {
                break;
            }
        }

        // shutdown consensus to freeze the state
        node_0.shutdown_consensus().await;
        let decided_leaf = node_0.decided_leaf().await;
        let state = node_0.decided_state().await;

        state
            .block_merkle_tree
            .lookup(decided_leaf.height() - 1)
            .expect_ok()
            .expect("block state not found");

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_state_reconstruction() -> anyhow::Result<()> {
        // This test verifies that a query node can successfully reconstruct its state
        // after being shut down from the database
        //
        // Steps:
        // 1. Start a test network with 5 nodes.
        // 2. Add a query node connected to the network.
        // 3. Let the network run until 3 epochs have passed.
        // 4. Shut down the query node.
        // 5. Attempt to reconstruct its state from storage using:
        //    - No fee/reward accounts
        //    - Only fee accounts
        //    - Only reward accounts
        //    - Both fee and reward accounts
        // 6. Assert that the reconstructed state is correct in all scenarios.

        setup_test();
        const EPOCH_HEIGHT: u64 = 10;

        type PosVersion = SequencerVersions<StaticVersion<0, 3>, StaticVersion<0, 0>>;

        let network_config = TestConfigBuilder::default()
            .epoch_height(EPOCH_HEIGHT)
            .build();

        let api_port = pick_unused_port().expect("No ports free for query service");

        tracing::info!("API PORT = {api_port}");
        const NUM_NODES: usize = 5;

        let storage = join_all((0..NUM_NODES).map(|_| SqlDataSource::create_storage())).await;
        let persistence: [_; NUM_NODES] = storage
            .iter()
            .map(<SqlDataSource as TestableSequencerDataSource>::persistence_options)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let config = TestNetworkConfigBuilder::with_num_nodes()
            .api_config(SqlDataSource::options(
                &storage[0],
                Options::with_port(api_port),
            ))
            .network_config(network_config)
            .persistences(persistence.clone())
            .catchups(std::array::from_fn(|_| {
                StatePeers::<StaticVersion<0, 1>>::from_urls(
                    vec![format!("http://localhost:{api_port}").parse().unwrap()],
                    Default::default(),
                    &NoMetrics,
                )
            }))
            .pos_hook::<PosVersion>(
                DelegationConfig::MultipleDelegators,
                hotshot_contract_adapter::stake_table::StakeTableContractVersion::V2,
            )
            .await
            .unwrap()
            .build();
        let state = config.states()[0].clone();
        let mut network = TestNetwork::new(config, PosVersion::new()).await;
        // Remove peer 0 and restart it with the query module enabled.
        // Adding an additional node to the test network is not straight forward,
        // as the keys have already been initialized in the config above.
        // So, we remove this node and re-add it using the same index.
        network.peers.remove(0);

        let node_0_storage = &storage[1];
        let node_0_persistence = persistence[1].clone();
        let node_0_port = pick_unused_port().expect("No ports free for query service");
        tracing::info!("node_0_port {node_0_port}");
        let opt = Options::with_port(node_0_port).query_sql(
            Query {
                peers: vec![format!("http://localhost:{api_port}").parse().unwrap()],
            },
            tmp_options(node_0_storage),
        );
        let node_0 = opt
            .clone()
            .serve(|metrics, consumer, storage| {
                let cfg = network.cfg.clone();
                let node_0_persistence = node_0_persistence.clone();
                let state = state.clone();
                async move {
                    Ok(cfg
                        .init_node(
                            1,
                            state,
                            node_0_persistence.clone(),
                            Some(StatePeers::<StaticVersion<0, 1>>::from_urls(
                                vec![format!("http://localhost:{api_port}").parse().unwrap()],
                                Default::default(),
                                &NoMetrics,
                            )),
                            storage,
                            &*metrics,
                            test_helpers::STAKE_TABLE_CAPACITY_FOR_TEST,
                            consumer,
                            PosVersion::new(),
                            Default::default(),
                        )
                        .await)
                }
                .boxed()
            })
            .await
            .unwrap();

        let mut events = network.peers[2].event_stream().await;
        // Wait until at least 3 epochs have passed
        wait_for_epochs(&mut events, EPOCH_HEIGHT, 3).await;

        tracing::warn!("shutting down node 0");

        node_0.shutdown_consensus().await;

        let instance = node_0.node_state();
        let state = node_0.decided_state().await;
        let fee_accounts = state
            .fee_merkle_tree
            .clone()
            .into_iter()
            .map(|(acct, _)| acct)
            .collect::<Vec<_>>();
        let reward_accounts = state
            .reward_merkle_tree
            .clone()
            .into_iter()
            .map(|(acct, _)| acct)
            .collect::<Vec<_>>();

        let client: Client<ServerError, SequencerApiVersion> =
            Client::new(format!("http://localhost:{node_0_port}").parse().unwrap());
        client.connect(Some(Duration::from_secs(10))).await;

        // wait 3s to be sure that all the
        // transactions have been committed
        sleep(Duration::from_secs(3)).await;

        tracing::info!("getting node block height");
        let node_block_height = client
            .get::<u64>("node/block-height")
            .send()
            .await
            .context("getting Espresso block height")
            .unwrap();

        tracing::info!("node block height={node_block_height}");

        let leaf_query_data = client
            .get::<LeafQueryData<SeqTypes>>(&format!("availability/leaf/{}", node_block_height - 1))
            .send()
            .await
            .context("error getting leaf")
            .unwrap();

        tracing::info!("leaf={leaf_query_data:?}");
        let leaf = leaf_query_data.leaf();
        let to_view = leaf.view_number() + 1;

        let ds = SqlStorage::connect(Config::try_from(&node_0_persistence).unwrap())
            .await
            .unwrap();
        let mut tx = ds.write().await?;

        let (state, leaf) =
            reconstruct_state(&instance, &mut tx, node_block_height - 1, to_view, &[], &[])
                .await
                .unwrap();
        assert_eq!(leaf.view_number(), to_view);
        assert!(
            state
                .block_merkle_tree
                .lookup(node_block_height - 1)
                .expect_ok()
                .is_ok(),
            "inconsistent block merkle tree"
        );

        // Reconstruct fee state
        let (state, leaf) = reconstruct_state(
            &instance,
            &mut tx,
            node_block_height - 1,
            to_view,
            &fee_accounts,
            &[],
        )
        .await
        .unwrap();

        assert_eq!(leaf.view_number(), to_view);
        assert!(
            state
                .block_merkle_tree
                .lookup(node_block_height - 1)
                .expect_ok()
                .is_ok(),
            "inconsistent block merkle tree"
        );

        for account in &fee_accounts {
            state.fee_merkle_tree.lookup(account).expect_ok().unwrap();
        }

        // Reconstruct reward state

        let (state, leaf) = reconstruct_state(
            &instance,
            &mut tx,
            node_block_height - 1,
            to_view,
            &[],
            &reward_accounts,
        )
        .await
        .unwrap();

        for account in &reward_accounts {
            state
                .reward_merkle_tree
                .lookup(account)
                .expect_ok()
                .unwrap();
        }

        assert_eq!(leaf.view_number(), to_view);
        assert!(
            state
                .block_merkle_tree
                .lookup(node_block_height - 1)
                .expect_ok()
                .is_ok(),
            "inconsistent block merkle tree"
        );
        // Reconstruct reward and fee state

        let (state, leaf) = reconstruct_state(
            &instance,
            &mut tx,
            node_block_height - 1,
            to_view,
            &fee_accounts,
            &reward_accounts,
        )
        .await
        .unwrap();

        assert!(
            state
                .block_merkle_tree
                .lookup(node_block_height - 1)
                .expect_ok()
                .is_ok(),
            "inconsistent block merkle tree"
        );
        assert_eq!(leaf.view_number(), to_view);
        for account in &reward_accounts {
            state
                .reward_merkle_tree
                .lookup(account)
                .expect_ok()
                .unwrap();
        }

        for account in &fee_accounts {
            state.fee_merkle_tree.lookup(account).expect_ok().unwrap();
        }

        Ok(())
    }

    /// Waits until a node has reached the given target epoch (exclusive).
    /// The function returns once the first event indicates an epoch higher than `target_epoch`.
    async fn wait_for_epochs(
        events: &mut (impl futures::Stream<Item = Event<SeqTypes>> + std::marker::Unpin),
        epoch_height: u64,
        target_epoch: u64,
    ) {
        while let Some(event) = events.next().await {
            if let EventType::Decide { leaf_chain, .. } = event.event {
                let leaf = leaf_chain[0].leaf.clone();
                let epoch = leaf.epoch(epoch_height);
                tracing::info!(
                    "Node decided at height: {}, epoch: {:?}",
                    leaf.height(),
                    epoch
                );

                if epoch > Some(EpochNumber::new(target_epoch)) {
                    break;
                }
            }
        }
    }
}
