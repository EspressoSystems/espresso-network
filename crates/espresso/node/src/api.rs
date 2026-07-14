use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};

use ::light_client::{
    LightClient,
    client::{FallbackClient, QueryServiceClient},
    state::{Genesis, LightClientOptions},
    storage::{LightClientSqliteOptions, SqliteStorage},
};
use alloy::primitives::U256;
use anyhow::{Context, bail, ensure};
use async_lock::RwLock;
use async_once_cell::Lazy;
use async_trait::async_trait;
use committable::Commitment;
use data_source::{
    CatchupDataSource, RequestResponseDataSource, StakeTableDataSource, StakeTableWithEpochNumber,
    StateCertDataSource, StateCertFetchingDataSource, SubmitDataSource,
};
use derivative::Derivative;
use espresso_types::{
    AccountQueryData, AuthenticatedValidatorMap, BlockMerkleTree, FeeAccount, FeeMerkleTree, Leaf2,
    NodeState, PubKey, Transaction,
    config::PublicNetworkConfig,
    retain_accounts,
    traits::EventsPersistenceRead,
    v0::traits::{SequencerPersistence, StateCatchup},
    v0_3::{
        ChainConfig, RegisteredValidator, RewardAccountQueryDataV1, RewardAccountV1, RewardAmount,
        RewardMerkleTreeV1, StakeTableEvent,
    },
    v0_4::{
        PermittedRewardMerkleTreeV2, RewardAccountQueryDataV2, RewardAccountV2, RewardMerkleTreeV2,
    },
};
use futures::{
    future::{BoxFuture, Future, FutureExt},
    stream::BoxStream,
};
use hotshot_contract_adapter::sol_types::EspToken;
use hotshot_events_service::events_source::{
    EventFilterSet, EventsSource, EventsStreamer, StartupInfo,
};
use hotshot_query_service::{
    availability::VidCommonQueryData,
    data_source::ExtensibleDataSource,
    fetching::{self, Provider},
};
use hotshot_types::{
    PeerConfig,
    data::{EpochNumber, VidCommitment, VidCommon, VidShare, ViewNumber},
    event::{Event, LegacyEvent},
    light_client::LCV3StateSignatureRequestBody,
    network::NetworkConfig,
    simple_certificate::LightClientStateUpdateCertificateV2,
    stake_table::HSStakeTable,
    traits::{
        election::{Membership, MembershipSnapshot, NonEpochMembershipSnapshot},
        network::ConnectedNetwork,
    },
    utils::epoch_from_block_number,
    vid::avidm::{AvidMScheme, init_avidm_param},
    vote::HasViewNumber,
};
use itertools::Itertools;
use jf_merkle_tree_compat::MerkleTreeScheme;
use moka::future::Cache;
use rand::Rng;
use request_response::RequestType;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use url::Url;
use vbs::version::Version;

use self::data_source::{HotShotConfigDataSource, NodeStateDataSource, StateSignatureDataSource};
use crate::{
    SeqTypes, SequencerApiVersion, SequencerContext,
    api::data_source::TokenDataSource,
    catchup::{
        CatchupStorage, add_fee_accounts_to_state, add_v1_reward_accounts_to_state,
        add_v2_reward_accounts_to_state,
    },
    consensus_handle::ConsensusHandle,
    context::ConsensusNode,
    request_response::{
        data_source::{retain_v1_reward_accounts, retain_v2_reward_accounts},
        request::{Request, Response},
    },
    state_cert::{StateCertFetchError, validate_state_cert},
    state_signature::StateSigner,
};

pub mod data_source;
pub mod endpoints;
pub mod fs;
pub mod light_client;
pub mod options;
pub mod sql;
pub mod state;
pub mod unlock_schedule;
mod update;

pub use options::Options;

pub type BlocksFrontier = <BlockMerkleTree as MerkleTreeScheme>::MembershipProof;

type BoxLazy<T> = Pin<Arc<Lazy<T, BoxFuture<'static, T>>>>;

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
struct ApiState<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> {
    // The consensus state is initialized lazily so we can start the API (and healthcheck endpoints)
    // before consensus has started. Any endpoint that uses consensus state will wait for
    // initialization to finish, but endpoints that do not require a consensus handle can proceed
    // without waiting.
    #[derivative(Debug = "ignore")]
    sequencer_context: BoxLazy<SequencerContext<N, P>>,

    // we cache `token_supply` for up to an hour, to avoid repeatedly querying the contract for information that rarely changes
    token_supply: Cache<(), U256>,
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> ApiState<N, P> {
    fn new(context_init: impl Future<Output = SequencerContext<N, P>> + Send + 'static) -> Self {
        Self {
            sequencer_context: Arc::pin(Lazy::from_future(context_init.boxed())),
            token_supply: Cache::builder()
                .max_capacity(1)
                .time_to_live(Duration::from_secs(3600))
                .build(),
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

    async fn consensus_handle(&self) -> Arc<ConsensusHandle<SeqTypes, ConsensusNode<N, P>>> {
        self.sequencer_context
            .as_ref()
            .get()
            .await
            .get_ref()
            .consensus_handle()
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

type StorageState<N, P, D> = ExtensibleDataSource<D, ApiState<N, P>>;

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> EventsSource<SeqTypes>
    for ApiState<N, P>
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

impl<N: ConnectedNetwork<PubKey>, D: Send + Sync, P: SequencerPersistence> TokenDataSource<SeqTypes>
    for StorageState<N, P, D>
{
    async fn get_initial_supply_l1(&self) -> anyhow::Result<U256> {
        self.as_ref().get_initial_supply_l1().await
    }

    async fn get_total_supply_l1(&self) -> anyhow::Result<U256> {
        self.as_ref().get_total_supply_l1().await
    }

    async fn get_decided_header(&self) -> espresso_types::Header {
        self.as_ref().get_decided_header().await
    }
}

impl<N: ConnectedNetwork<PubKey>, D: Send + Sync, P: SequencerPersistence> SubmitDataSource<N, P>
    for StorageState<N, P, D>
{
    async fn submit(&self, tx: Transaction) -> anyhow::Result<()> {
        self.as_ref().submit(tx).await
    }
}

impl<N: ConnectedNetwork<PubKey>, D: Sync, P: SequencerPersistence> StakeTableDataSource<SeqTypes>
    for StorageState<N, P, D>
{
    /// Get the stake table for a given epoch
    async fn get_stake_table(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>> {
        self.as_ref().get_stake_table(epoch).await
    }

    /// Get the stake table for the current epoch if not provided
    async fn get_stake_table_current(&self) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>> {
        self.as_ref().get_stake_table_current().await
    }

    /// Get the DA stake table for a given epoch
    async fn get_da_stake_table(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>> {
        self.as_ref().get_da_stake_table(epoch).await
    }

    /// Get the DA stake table for the current epoch if not provided
    async fn get_da_stake_table_current(
        &self,
    ) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>> {
        self.as_ref().get_da_stake_table_current().await
    }

    /// Get all the validators
    async fn get_validators(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<AuthenticatedValidatorMap> {
        self.as_ref().get_validators(epoch).await
    }

    async fn get_block_reward(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Option<RewardAmount>> {
        self.as_ref().get_block_reward(epoch).await
    }
    /// Get all the validator participation for the current epoch
    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64> {
        self.as_ref().current_proposal_participation().await
    }
    /// Get all the validator participation for the previous epoch
    async fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        self.as_ref().proposal_participation(epoch).await
    }
    /// Get all the vote participation for the current epoch
    async fn current_vote_participation(&self) -> HashMap<PubKey, f64> {
        self.as_ref().current_vote_participation().await
    }
    /// Get all the vote participation for a given epoch
    async fn vote_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        self.as_ref().vote_participation(epoch).await
    }

    async fn get_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        self.as_ref().get_all_validators(epoch, offset, limit).await
    }

    async fn stake_table_events(
        &self,
        from_l1_block: u64,
        to_l1_block: u64,
    ) -> anyhow::Result<Vec<StakeTableEvent>> {
        self.as_ref()
            .stake_table_events(from_l1_block, to_l1_block)
            .await
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> TokenDataSource<SeqTypes>
    for ApiState<N, P>
{
    async fn get_initial_supply_l1(&self) -> anyhow::Result<U256> {
        let node_state = self.sequencer_context.as_ref().get().await.node_state();
        let fetcher = node_state.coordinator.membership().fetcher().clone();
        let cached = *fetcher.initial_supply.read().await;
        match cached {
            Some(supply) => Ok(supply),
            None => Ok(fetcher.fetch_and_update_initial_supply().await?),
        }
    }

    async fn get_total_supply_l1(&self) -> anyhow::Result<U256> {
        match self.token_supply.get(&()).await {
            Some(supply) => Ok(supply),
            None => {
                let node_state = self.sequencer_context.as_ref().get().await.node_state();
                let token_contract_address = node_state.token_contract_address().await?;

                let provider = node_state.l1_client.provider;

                let token = EspToken::new(token_contract_address, provider.clone());

                let supply = token
                    .totalSupply()
                    .call()
                    .await
                    .context("Failed to retrieve totalSupply from the contract")?;

                self.token_supply.insert((), supply).await;

                Ok(supply)
            },
        }
    }

    async fn get_decided_header(&self) -> espresso_types::Header {
        self.consensus_handle()
            .await
            .decided_leaf()
            .await
            .block_header()
            .clone()
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> StakeTableDataSource<SeqTypes>
    for ApiState<N, P>
{
    /// Get the stake table for a given epoch
    async fn get_stake_table(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>> {
        let handle = self.consensus_handle().await;
        if let Some(requested) = epoch {
            let first_epoch = handle
                .membership_coordinator()
                .await
                .membership()
                .first_epoch();
            if let Some(first_epoch) = first_epoch
                && requested < first_epoch
            {
                return Err(anyhow::anyhow!(
                    "requested stake table for epoch {requested:?} is below the first epoch \
                     {first_epoch:?}"
                ));
            }
        }
        let highest_epoch = handle.current_epoch().await.map(|e| e + 1);
        if epoch > highest_epoch {
            return Err(anyhow::anyhow!(
                "requested stake table for epoch {epoch:?} is beyond the current epoch + 1 \
                 {highest_epoch:?}"
            ));
        }
        let mem = handle
            .membership_coordinator()
            .await
            .stake_table_for_epoch(epoch)?;

        Ok(mem.stake_table().cloned().collect())
    }

    /// Get the stake table for the current epoch and return it along with the epoch number
    async fn get_stake_table_current(&self) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>> {
        let epoch = self.consensus_handle().await.current_epoch().await;

        Ok(StakeTableWithEpochNumber {
            epoch,
            stake_table: self.get_stake_table(epoch).await?,
        })
    }

    /// Get the DA stake table for a given epoch
    async fn get_da_stake_table(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Vec<PeerConfig<SeqTypes>>> {
        let coordinator = self.consensus_handle().await.membership_coordinator().await;
        Ok(match epoch {
            Some(e) => coordinator
                .membership()
                .snapshot(e)
                .map(|s| s.da_stake_table().cloned().collect())
                .unwrap_or_default(),
            None => coordinator
                .membership()
                .non_epoch_snapshot()
                .da_stake_table()
                .cloned()
                .collect(),
        })
    }

    /// Get the DA stake table for the current epoch and return it along with the epoch number
    async fn get_da_stake_table_current(
        &self,
    ) -> anyhow::Result<StakeTableWithEpochNumber<SeqTypes>> {
        let epoch = self.consensus_handle().await.current_epoch().await;

        Ok(StakeTableWithEpochNumber {
            epoch,
            stake_table: self.get_da_stake_table(epoch).await?,
        })
    }

    async fn get_block_reward(
        &self,
        epoch: Option<EpochNumber>,
    ) -> anyhow::Result<Option<RewardAmount>> {
        let coordinator = self.consensus_handle().await.membership_coordinator().await;

        let membership = coordinator.membership();
        let block_reward = match epoch {
            None => membership.fixed_block_reward(),
            Some(e) => membership.epoch_block_reward(e),
        };

        Ok(block_reward)
    }

    /// Get the whole validators map
    async fn get_validators(&self, e: EpochNumber) -> anyhow::Result<AuthenticatedValidatorMap> {
        Ok(self
            .consensus_handle()
            .await
            .membership_coordinator()
            .await
            .membership_for_epoch(Some(e))
            .context("membership not found")?
            .snapshot()
            .with_context(|| format!("no committee for epoch={e}"))?
            .validators()
            .clone())
    }

    /// Get the current proposal participation.
    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64> {
        self.consensus_handle()
            .await
            .current_proposal_participation()
            .await
    }

    /// Get the proposal participation for a given epoch.
    async fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        self.consensus_handle()
            .await
            .proposal_participation(epoch)
            .await
    }

    /// Get the current vote participation.
    async fn current_vote_participation(&self) -> HashMap<PubKey, f64> {
        self.consensus_handle()
            .await
            .current_vote_participation()
            .await
    }

    /// Get the vote participation for a given epoch.
    async fn vote_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64> {
        self.consensus_handle()
            .await
            .vote_participation(Some(epoch))
            .await
    }

    async fn get_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        let storage = self.consensus_handle().await.storage().await;
        storage.load_all_validators(epoch, offset, limit).await
    }

    async fn stake_table_events(
        &self,
        from_l1_block: u64,
        to_l1_block: u64,
    ) -> anyhow::Result<Vec<StakeTableEvent>> {
        let storage = self.consensus_handle().await.storage().await;
        let (status, events) = storage.load_events(from_l1_block, to_l1_block).await?;
        ensure!(
            status == Some(EventsPersistenceRead::Complete),
            "some events in range [{from_l1_block}, {to_l1_block}] are not available ({status:?})"
        );
        Ok(events.into_iter().map(|(_, event)| event).collect())
    }
}

impl<N: ConnectedNetwork<PubKey>, D: Sync, P: SequencerPersistence>
    RequestResponseDataSource<SeqTypes> for StorageState<N, P, D>
{
    async fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common_data: VidCommonQueryData<SeqTypes>,
        timeout_duration: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>> {
        self.as_ref()
            .request_vid_shares(block_number, vid_common_data, timeout_duration)
            .await
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, D: Sync, P: SequencerPersistence>
    StateCertFetchingDataSource<SeqTypes> for StorageState<N, P, D>
{
    async fn request_state_cert(
        &self,
        epoch: u64,
        timeout: Duration,
    ) -> Result<LightClientStateUpdateCertificateV2<SeqTypes>, StateCertFetchError> {
        self.as_ref().request_state_cert(epoch, timeout).await
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> RequestResponseDataSource<SeqTypes>
    for ApiState<N, P>
{
    async fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common_data: VidCommonQueryData<SeqTypes>,
        duration: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>> {
        // Get a handle to the request response protocol
        let request_response_protocol = self
            .sequencer_context
            .as_ref()
            .get()
            .await
            .request_response_protocol
            .clone();

        async move {
            // Get the total VID weight based on the VID common data
            let total_weight = match vid_common_data.common() {
                VidCommon::V0(_) => {
                    // TODO: This needs to be done via the stake table
                    return Err(anyhow::anyhow!(
                        "V0 total weight calculation not supported yet"
                    ));
                },
                VidCommon::V1(v1) => v1.total_weights,
                VidCommon::V2(v2) => v2.param.total_weights,
            };

            // Create the AvidM parameters from the total weight
            let avidm_param = init_avidm_param(total_weight)
                .with_context(|| "failed to initialize avidm param")?;

            // Get the payload hash for verification
            let VidCommitment::V1(local_payload_hash) = vid_common_data.payload_hash() else {
                bail!("V0 share verification not supported yet");
            };

            // Create a random request id
            let request_id = rand::thread_rng().r#gen();

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
        .boxed()
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> StateCertFetchingDataSource<SeqTypes>
    for ApiState<N, P>
{
    async fn request_state_cert(
        &self,
        epoch: u64,
        timeout: Duration,
    ) -> Result<LightClientStateUpdateCertificateV2<SeqTypes>, StateCertFetchError> {
        tracing::info!("fetching state certificate for epoch={epoch}");
        let handle = self.consensus_handle().await;

        let current_epoch = handle.current_epoch().await;

        // The highest epoch we can have a state certificate for is current_epoch + 1
        // Check if requested epoch is beyond the highest possible epoch
        let highest_epoch = current_epoch.map(|e| e.u64() + 1);

        if Some(epoch) > highest_epoch {
            return Err(StateCertFetchError::Other(anyhow::anyhow!(
                "requested state certificate for epoch {epoch} is beyond the highest possible \
                 epoch {highest_epoch:?}"
            )));
        }

        // Get the stake table for validation
        let coordinator = handle.membership_coordinator().await;
        if let Err(err) = coordinator.stake_table_for_epoch(Some(EpochNumber::new(epoch))) {
            tracing::warn!(
                "Failed to get membership for epoch {epoch}: {err:#}. Waiting for catchup"
            );

            coordinator
                .wait_for_catchup(EpochNumber::new(epoch))
                .await
                .map_err(|e| {
                    StateCertFetchError::Other(
                        anyhow::Error::new(e)
                            .context(format!("failed to catch up for stake table epoch={epoch}")),
                    )
                })?;
        }

        let membership = coordinator
            .stake_table_for_epoch(Some(EpochNumber::new(epoch)))
            .map_err(|e| {
                StateCertFetchError::Other(
                    anyhow::Error::new(e)
                        .context(format!("failed to get stake table for epoch={epoch}")),
                )
            })?;

        let stake_table = HSStakeTable::from_iter(membership.stake_table());

        let state_catchup = self
            .sequencer_context
            .as_ref()
            .get()
            .await
            .node_state()
            .state_catchup
            .clone();

        let result = tokio::time::timeout(timeout, state_catchup.fetch_state_cert(epoch)).await;

        match result {
            Err(_) => Err(StateCertFetchError::FetchError(anyhow::anyhow!(
                "timeout while fetching state cert for epoch {epoch}"
            ))),
            Ok(Ok(cert)) => {
                // Validation errors should be mapped to ValidationError
                validate_state_cert(&cert, &stake_table).map_err(|e| {
                    StateCertFetchError::ValidationError(e.context(format!(
                        "state certificate validation failed for epoch={epoch}"
                    )))
                })?;

                tracing::info!("fetched and validated state certificate for epoch {epoch}");
                Ok(cert)
            },
            Ok(Err(e)) => Err(StateCertFetchError::FetchError(
                e.context(format!("failed to fetch state cert for epoch {epoch}")),
            )),
        }
    }
}

// Thin wrapper implementations that delegate to persistence
#[async_trait]
impl<N: ConnectedNetwork<PubKey>, D: Sync, P: SequencerPersistence> StateCertDataSource
    for StorageState<N, P, D>
{
    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        self.as_ref().get_state_cert_by_epoch(epoch).await
    }

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        self.as_ref().insert_state_cert(epoch, cert).await
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> StateCertDataSource for ApiState<N, P> {
    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>> {
        let storage = self.consensus_handle().await.storage().await;
        storage.get_state_cert_by_epoch(epoch).await
    }

    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        let storage = self.consensus_handle().await.storage().await;
        storage.insert_state_cert(epoch, cert).await
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> SubmitDataSource<N, P>
    for ApiState<N, P>
{
    async fn submit(&self, tx: Transaction) -> anyhow::Result<()> {
        let handle = self.consensus_handle().await;

        // Fetch full chain config from the validated state, if present.
        // This is necessary because we support chain config upgrades,
        // so the updated chain config is found in the validated state.
        let cf = handle
            .decided_state()
            .await
            .and_then(|state| state.chain_config.resolve());

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

        handle.submit_transaction(tx).await?;
        Ok(())
    }
}

impl<N, P, D> NodeStateDataSource for StorageState<N, P, D>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    D: Sync,
{
    async fn node_state(&self) -> NodeState {
        self.as_ref().node_state().await
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, D: CatchupStorage + Send + Sync>
    data_source::DatabaseMetadataSource for StorageState<N, P, D>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    D: data_source::DatabaseMetadataSource + Send + Sync,
{
    async fn get_table_sizes(&self) -> anyhow::Result<Vec<data_source::TableSize>> {
        self.inner().get_table_sizes().await
    }

    async fn get_migration_status(&self) -> anyhow::Result<Vec<data_source::MigrationStatus>> {
        self.inner().get_migration_status().await
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, D: CatchupStorage + Send + Sync>
    data_source::PruningDataSource for StorageState<N, P, D>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    D: data_source::PruningDataSource + Send + Sync,
{
    async fn get_oldest_block(
        &self,
    ) -> anyhow::Result<Option<hotshot_query_service::availability::BlockQueryData<crate::SeqTypes>>>
    {
        self.inner().get_oldest_block().await
    }

    async fn get_oldest_leaf(
        &self,
    ) -> anyhow::Result<Option<hotshot_query_service::availability::LeafQueryData<crate::SeqTypes>>>
    {
        self.inner().get_oldest_leaf().await
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence, D: CatchupStorage + Send + Sync>
    CatchupDataSource for StorageState<N, P, D>
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

        let handle = self.as_ref().consensus_handle().await;
        if let Err(err) = add_fee_accounts_to_state(&*handle, &view, accounts, &tree, leaf).await {
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

    async fn get_cert2(
        &self,
        height: u64,
    ) -> anyhow::Result<Option<espresso_types::Certificate2<SeqTypes>>> {
        self.inner().load_cert2(height).await
    }

    #[tracing::instrument(skip(self, instance))]
    async fn get_reward_accounts_v2(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV2],
    ) -> anyhow::Result<RewardMerkleTreeV2> {
        // Check if we have the desired state in memory.
        match self
            .as_ref()
            .get_reward_accounts_v2(instance, height, view, accounts)
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
            .get_reward_accounts_v2(instance, height, view, accounts)
            .await
            .context("accounts not in memory, and could not fetch from storage")?;

        // If we successfully fetched accounts from storage, try to add them back into the in-memory
        // state.
        let handle = self.as_ref().consensus_handle().await;
        if let Err(err) =
            add_v2_reward_accounts_to_state(&*handle, &view, accounts, &tree, leaf).await
        {
            tracing::warn!(?view, "cannot update fetched account state: {err:#}");
        }
        tracing::info!(?view, "updated with fetched account state");

        Ok(tree)
    }

    #[tracing::instrument(skip(self, instance))]
    async fn get_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<RewardMerkleTreeV1> {
        // Check if we have the desired state in memory.
        match self
            .as_ref()
            .get_reward_accounts_v1(instance, height, view, accounts)
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
            .get_reward_accounts_v1(instance, height, view, accounts)
            .await
            .context("accounts not in memory, and could not fetch from storage")?;

        // If we successfully fetched accounts from storage, try to add them back into the in-memory
        // state.
        let handle = self.as_ref().consensus_handle().await;
        if let Err(err) =
            add_v1_reward_accounts_to_state(&*handle, &view, accounts, &tree, leaf).await
        {
            tracing::warn!(?view, "cannot update fetched account state: {err:#}");
        }
        tracing::info!(?view, "updated with fetched account state");

        Ok(tree)
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
        view: ViewNumber,
    ) -> anyhow::Result<Vec<u8>> {
        self.as_ref().get_reward_merkle_tree_v2(height, view).await
    }

    #[tracing::instrument(skip(self))]
    async fn get_state_cert(
        &self,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        let storage = self.as_ref().consensus_handle().await.storage().await;
        storage
            .get_state_cert_by_epoch(epoch)
            .await?
            .context(format!("state cert for epoch {epoch} not found"))
    }
}

impl<N, P> NodeStateDataSource for ApiState<N, P>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
{
    async fn node_state(&self) -> NodeState {
        self.sequencer_context.as_ref().get().await.node_state()
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> CatchupDataSource for ApiState<N, P> {
    #[tracing::instrument(skip(self, _instance))]
    async fn get_accounts(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<FeeMerkleTree> {
        let state = self
            .consensus_handle()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view}"
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
            .consensus_handle()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view}"
            ))?;
        let tree = &state.block_merkle_tree;
        let frontier = tree.lookup(tree.num_leaves() - 1).expect_ok()?.1;
        Ok(frontier)
    }

    async fn get_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        let state = self
            .consensus_handle()
            .await
            .decided_state()
            .await
            .context("decided state not available")?;
        let chain_config = state.chain_config;

        if chain_config.commit() == commitment {
            chain_config.resolve().context("chain config found")
        } else {
            bail!("chain config not found")
        }
    }

    async fn get_leaf_chain(&self, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        // Builds a legacy 3-chain from undecided leaves in memory. New-protocol heights fall
        // through to the storage path.
        let mut leaves = self.consensus_handle().await.undecided_leaves().await;
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
    async fn get_reward_accounts_v2(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV2],
    ) -> anyhow::Result<RewardMerkleTreeV2> {
        let state = self
            .consensus_handle()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view}"
            ))?;

        retain_v2_reward_accounts(&state.reward_merkle_tree_v2, accounts.iter().copied())
    }

    #[tracing::instrument(skip(self, _instance))]
    async fn get_reward_accounts_v1(
        &self,
        _instance: &NodeState,
        height: u64,
        view: ViewNumber,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<RewardMerkleTreeV1> {
        let state = self
            .consensus_handle()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view}"
            ))?;

        retain_v1_reward_accounts(&state.reward_merkle_tree_v1, accounts.iter().copied())
    }

    async fn get_reward_merkle_tree_v2(
        &self,
        height: u64,
        view: ViewNumber,
    ) -> anyhow::Result<Vec<u8>> {
        let state = self
            .consensus_handle()
            .await
            .state(view)
            .await
            .context(format!(
                "state not available for height {height}, view {view}"
            ))?;

        let tree_data = TryInto::<RewardMerkleTreeV2Data>::try_into(&state.reward_merkle_tree_v2)
            .inspect_err(
            |err| tracing::debug!(%err, height, %view, "cannot serve reward merkle tree"),
        )?;
        let merkle_tree_bytes = bincode::serialize(&tree_data)
            .context("Merkle tree serialization failed; this should never happen.")?;

        Ok(merkle_tree_bytes)
    }

    async fn get_state_cert(
        &self,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        self.get_state_cert_by_epoch(epoch)
            .await?
            .context(format!("state cert not found for epoch {epoch}"))
    }
}

impl<N: ConnectedNetwork<PubKey>, D: Sync, P: SequencerPersistence> HotShotConfigDataSource
    for StorageState<N, P, D>
{
    async fn get_config(&self) -> PublicNetworkConfig {
        self.as_ref().network_config().await.into()
    }
}

impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> HotShotConfigDataSource
    for ApiState<N, P>
{
    async fn get_config(&self) -> PublicNetworkConfig {
        self.network_config().await.into()
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, D: Sync, P: SequencerPersistence> StateSignatureDataSource<N>
    for StorageState<N, P, D>
{
    async fn get_state_signature(&self, height: u64) -> Option<LCV3StateSignatureRequestBody> {
        self.as_ref().get_state_signature(height).await
    }
}

#[async_trait]
impl<N: ConnectedNetwork<PubKey>, P: SequencerPersistence> StateSignatureDataSource<N>
    for ApiState<N, P>
{
    async fn get_state_signature(&self, height: u64) -> Option<LCV3StateSignatureRequestBody> {
        self.state_signer()
            .await
            .read()
            .await
            .get_state_signature(height)
            .await
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Representation of the RewardMerkleTreeV2 as a set of key-value pairs
pub struct RewardMerkleTreeV2Data {
    pub balances: Vec<(RewardAccountV2, RewardAmount)>,
}

impl TryInto<RewardMerkleTreeV2Data> for &RewardMerkleTreeV2 {
    type Error = anyhow::Error;
    // Required method
    fn try_into(self) -> anyhow::Result<RewardMerkleTreeV2Data> {
        let num_leaves = self.num_leaves();

        let balances: Vec<_> = self
            .iter()
            .map(|(account, balance)| (*account, *balance))
            .collect();

        if balances.len() as u64 == num_leaves {
            Ok(RewardMerkleTreeV2Data { balances })
        } else {
            bail!(
                "RewardMerkleTreeV2 is incomplete, some accounts are missing. Balances length: \
                 {}, num_leaves: {num_leaves}.",
                balances.len(),
            );
        }
    }
}

pub(crate) trait RewardMerkleTreeDataSource: Send + Sync + Clone + 'static {
    fn load_v1_reward_account_proof(
        &self,
        _height: u64,
        _account: RewardAccountV1,
    ) -> impl Send + Future<Output = anyhow::Result<RewardAccountQueryDataV1>>;

    fn save_and_gc_reward_tree_v2(
        &self,
        node_state: &NodeState,
        height: u64,
        version: Version,
        merkle_tree: &RewardMerkleTreeV2,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move {
            // The merklized state loop always applies full blocks, so an incomplete
            // tree here indicates something is seriously wrong.
            let tree_data = TryInto::<RewardMerkleTreeV2Data>::try_into(merkle_tree).inspect_err(
                |err| tracing::error!(%err, height, "cannot persist incomplete RewardMerkleTreeV2"),
            )?;
            let serialization =
                bincode::serialize(&tree_data).context("Merkle tree serialization failed")?;
            self.persist_tree(height, serialization).await?;

            // Skip garbage collection in tests
            if cfg!(any(test, feature = "testing")) {
                return Ok(());
            }

            let finalized_hotshot_height = match node_state.finalized_hotshot_height().await {
                Ok(h) => h,
                Err(err) => {
                    tracing::warn!("failed to get finalized hotshot height: {err:#}");
                    return Ok(());
                },
            };

            // trees at heights strictly less than the gc height are deleted
            //
            // keep recent epochs reward trees
            //   - staking-api-service at startup calls `reward-amounts` at
            //     `epoch_start - 1`, which needs the previous epoch's last-block
            //     tree on disk.
            //   - Per epoch reward (EPOCH_REWARD_VERSION+): `fetch_and_calculate`
            //     reads the previous epoch's last block tree to compute the next
            //     epoch's rewards.
            //
            // `finalized_hotshot_height`:  Reward claims
            //   (`reward-claim-input`) target the LightClient L1 finalization
            //   exactly.

            let epoch_height = node_state
                .epoch_height
                .context("reward tree gc requires an epoch height")?;
            // EPOCH_REWARD_VERSION (V5)+ only persists a tree at each epoch boundary,
            // so 5 epochs = 5 trees on disk. Earlier versions persist a tree at
            // every block, so 1 epoch is already epoch_height trees — keeping more
            // would be expensive. We only need 1 epoch for both, but the extra
            // trees are cheap for V5+ so it doesn't make much of a difference.
            let epochs_to_retain = if version >= versions::EPOCH_REWARD_VERSION {
                5
            } else {
                1
            };
            let current_epoch = epoch_from_block_number(height, epoch_height);
            // First block of the oldest epoch we still want to retain.
            let epoch_start_block = current_epoch.saturating_sub(epochs_to_retain) * epoch_height;

            let gc_height = epoch_start_block.min(finalized_hotshot_height);

            if let Err(err) = self.garbage_collect(gc_height).await {
                tracing::info!(gc_height, "failed to garbage collect: {err:#}");
            }

            Ok(())
        }
    }

    fn persist_reward_proofs(
        &self,
        node_state: &NodeState,
        height: u64,
        version: Version,
    ) -> impl Send + Future<Output = anyhow::Result<()>>;

    fn load_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<PermittedRewardMerkleTreeV2>> {
        async move {
            let tree_bytes = self.load_tree(height).await?;

            let tree_data = bincode::deserialize::<RewardMerkleTreeV2Data>(&tree_bytes).context(
                "Failed to deserialize RewardMerkleTreeV2 for height {height} from storage; this \
                 should never happen.",
            )?;

            PermittedRewardMerkleTreeV2::try_from_kv_set(tree_data.balances)
                .await
                .context("Failed to reconstruct reward merkle tree from storage")
        }
    }

    /// Returns the RewardMerkleTreeV2 for height <= requested height
    ///
    /// After V5 the tree is only written at epoch boundaries, so `reward_merkle_tree_v2_data`
    /// has no row for most heights. Within an epoch the tree doesn't change, so the previous
    /// boundary's tree matches the current block's reward root but only if we're actually in
    /// the same epoch. The caller is responsible for checking the returned tree's commitment
    /// against the header at `height`.
    /// if they differ we loaded a tree from an older epoch.
    fn load_latest_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<PermittedRewardMerkleTreeV2>> {
        async move {
            let tree_bytes = self.load_latest_tree(height).await?;

            let tree_data = bincode::deserialize::<RewardMerkleTreeV2Data>(&tree_bytes)
                .context("Failed to deserialize RewardMerkleTreeV2 from storage")?;

            PermittedRewardMerkleTreeV2::try_from_kv_set(tree_data.balances)
                .await
                .context("Failed to reconstruct reward merkle tree from storage")
        }
    }

    fn load_reward_account_proof_v2(
        &self,
        _height: u64,
        _account: RewardAccountV2,
    ) -> impl Send + Future<Output = anyhow::Result<RewardAccountQueryDataV2>> {
        async {
            bail!("load_reward_account_proof_v2 is not supported for this data source");
        }
    }

    fn load_latest_reward_account_proof_v2(
        &self,
        account: RewardAccountV2,
    ) -> impl Send + Future<Output = anyhow::Result<RewardAccountQueryDataV2>> {
        async move {
            let serialized_account = bincode::serialize(&account).context(
                "Failed to serialize RewardAccountV2 for lookup; this should never happen.",
            )?;
            let proof_bytes = self.load_latest_proof(serialized_account).await?;

            bincode::deserialize::<RewardAccountQueryDataV2>(&proof_bytes).context(
                "Failed to deserialize RewardAccountQueryDataV2 for account {account} from \
                 storage; this should never happen.",
            )
        }
    }

    fn persist_tree(
        &self,
        height: u64,
        merkle_tree: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<()>>;

    fn load_tree(&self, height: u64) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>>;

    /// Load the latest serialized reward merkle tree v2 at height `<= height`.
    fn load_latest_tree(&self, height: u64)
    -> impl Send + Future<Output = anyhow::Result<Vec<u8>>>;

    fn persist_proofs(
        &self,
        height: u64,
        proofs: impl Iterator<Item = (Vec<u8>, Vec<u8>)> + Send,
    ) -> impl Send + Future<Output = anyhow::Result<()>>;

    fn load_proof(
        &self,
        height: u64,
        account: Vec<u8>,
        epoch_height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>>;

    fn load_latest_proof(
        &self,
        account: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>>;

    fn proof_exists(&self, height: u64) -> impl Send + Future<Output = bool>;

    /// garbage collects merkle tree data for blocks strictly older than `height`
    fn garbage_collect(&self, height: u64) -> impl Send + Future<Output = anyhow::Result<()>>;
}

impl RewardMerkleTreeDataSource for hotshot_query_service::data_source::MetricsDataSource {
    fn load_v1_reward_account_proof(
        &self,
        _height: u64,
        _account: RewardAccountV1,
    ) -> impl Send + Future<Output = anyhow::Result<RewardAccountQueryDataV1>> {
        async {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn persist_reward_proofs(
        &self,
        _node_state: &NodeState,
        _height: u64,
        _version: Version,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn persist_tree(
        &self,
        _height: u64,
        _merkle_tree: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn load_tree(&self, _height: u64) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn load_latest_tree(
        &self,
        _height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn garbage_collect(&self, _height: u64) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn persist_proofs(
        &self,
        _height: u64,
        _proofs: impl Iterator<Item = (Vec<u8>, Vec<u8>)> + Send,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn load_proof(
        &self,
        _height: u64,
        _account: Vec<u8>,
        _epoch_height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn load_latest_proof(
        &self,
        _account: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move {
            bail!("reward merklized state is not supported for this data source");
        }
    }

    fn proof_exists(&self, _height: u64) -> impl Send + Future<Output = bool> {
        async move { false }
    }
}

impl<T, S> RewardMerkleTreeDataSource
    for hotshot_query_service::data_source::ExtensibleDataSource<T, S>
where
    T: RewardMerkleTreeDataSource,
    S: Send + Sync + Clone + NodeStateDataSource + 'static,
{
    async fn load_v1_reward_account_proof(
        &self,
        height: u64,
        account: RewardAccountV1,
    ) -> anyhow::Result<RewardAccountQueryDataV1> {
        self.inner()
            .load_v1_reward_account_proof(height, account)
            .await
    }

    async fn load_reward_account_proof_v2(
        &self,
        height: u64,
        account: RewardAccountV2,
    ) -> anyhow::Result<RewardAccountQueryDataV2> {
        let epoch_height = self
            .as_ref()
            .node_state()
            .await
            .epoch_height
            .context("epoch height not found")?;
        let serialized_account = bincode::serialize(&account)
            .context("Failed to serialize RewardAccountV2 for lookup; this should never happen.")?;
        let proof_bytes = self
            .inner()
            .load_proof(height, serialized_account, epoch_height)
            .await?;

        bincode::deserialize::<RewardAccountQueryDataV2>(&proof_bytes)
            .context("Failed to deserialize RewardAccountQueryDataV2 from storage")
    }

    fn persist_tree(
        &self,
        height: u64,
        merkle_tree: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move { self.inner().persist_tree(height, merkle_tree).await }
    }

    fn load_tree(&self, height: u64) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { self.inner().load_tree(height).await }
    }

    fn load_latest_tree(
        &self,
        height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { self.inner().load_latest_tree(height).await }
    }

    fn garbage_collect(&self, height: u64) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move { self.inner().garbage_collect(height).await }
    }

    fn persist_proofs(
        &self,
        height: u64,
        proofs: impl Iterator<Item = (Vec<u8>, Vec<u8>)> + Send,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move { self.inner().persist_proofs(height, proofs).await }
    }

    fn load_proof(
        &self,
        height: u64,
        account: Vec<u8>,
        epoch_height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { self.inner().load_proof(height, account, epoch_height).await }
    }

    fn load_latest_proof(
        &self,
        account: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { self.inner().load_latest_proof(account).await }
    }

    fn proof_exists(&self, height: u64) -> impl Send + Future<Output = bool> {
        async move { self.inner().proof_exists(height).await }
    }

    fn persist_reward_proofs(
        &self,
        node_state: &NodeState,
        height: u64,
        version: Version,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move {
            self.inner()
                .persist_reward_proofs(node_state, height, version)
                .await
        }
    }
}

// Implement Reward MerkleTreeDataSource for Arc<D> to allow shared ownership
impl<D> RewardMerkleTreeDataSource for Arc<D>
where
    D: RewardMerkleTreeDataSource,
{
    async fn load_v1_reward_account_proof(
        &self,
        height: u64,
        account: RewardAccountV1,
    ) -> anyhow::Result<RewardAccountQueryDataV1> {
        (**self).load_v1_reward_account_proof(height, account).await
    }

    fn persist_tree(
        &self,
        height: u64,
        merkle_tree: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move { (**self).persist_tree(height, merkle_tree).await }
    }

    fn load_tree(&self, height: u64) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { (**self).load_tree(height).await }
    }

    fn load_latest_tree(
        &self,
        height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { (**self).load_latest_tree(height).await }
    }

    fn load_reward_merkle_tree_v2(
        &self,
        height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<PermittedRewardMerkleTreeV2>> {
        async move { (**self).load_reward_merkle_tree_v2(height).await }
    }

    fn load_reward_account_proof_v2(
        &self,
        height: u64,
        account: RewardAccountV2,
    ) -> impl Send + Future<Output = anyhow::Result<RewardAccountQueryDataV2>> {
        async move { (**self).load_reward_account_proof_v2(height, account).await }
    }

    fn persist_proofs(
        &self,
        height: u64,
        proofs: impl Iterator<Item = (Vec<u8>, Vec<u8>)> + Send,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move { (**self).persist_proofs(height, proofs).await }
    }

    fn persist_reward_proofs(
        &self,
        node_state: &NodeState,
        height: u64,
        version: Version,
    ) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move {
            (**self)
                .persist_reward_proofs(node_state, height, version)
                .await
        }
    }

    fn load_proof(
        &self,
        height: u64,
        account: Vec<u8>,
        epoch_height: u64,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { (**self).load_proof(height, account, epoch_height).await }
    }

    fn proof_exists(&self, height: u64) -> impl Send + Future<Output = bool> {
        async move { (**self).proof_exists(height).await }
    }

    fn load_latest_proof(
        &self,
        account: Vec<u8>,
    ) -> impl Send + Future<Output = anyhow::Result<Vec<u8>>> {
        async move { (**self).load_latest_proof(account).await }
    }

    fn garbage_collect(&self, height: u64) -> impl Send + Future<Output = anyhow::Result<()>> {
        async move { (**self).garbage_collect(height).await }
    }
}

/// [`Provider`] implementation wrapping a lazy [`LightClient`].
///
/// The [`LightClient`] requires a genesis to initialize itself, which we can get from the
/// [`ApiState`]. However, the [`Provider`] instance must be provided to the API data source at
/// initialization time, while the [`ApiState`] is only initialized lazily. This is a provider
/// implementation which is itself initialized lazily: [`Provider::fetch`] calls will time out until
/// the underlying [`ApiState`] is fully initialized, at which point this provider will start
/// serving fetches using the [`LightClient`].
#[derive(Debug)]
struct LightClientProvider {
    light_client: BoxLazy<LightClient<SqliteStorage, FallbackClient<QueryServiceClient>>>,
}

impl LightClientProvider {
    pub async fn new<N, P>(
        peers: impl IntoIterator<Item = Url>,
        state: ApiState<N, P>,
        opt: LightClientOptions,
        db_opt: LightClientSqliteOptions,
    ) -> anyhow::Result<Self>
    where
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
    {
        let db = db_opt
            .connect()
            .await
            .context("creating SQLite database for light client")?;
        let client = FallbackClient::new(peers.into_iter().map(QueryServiceClient::new).collect())?;
        let init_light_client = async move {
            let config = state.network_config().await;
            let chain_id = state.node_state().await.genesis_chain_config.chain_id;
            let epoch_height = config.config.epoch_height;
            let first_epoch =
                epoch_from_block_number(config.config.epoch_start_block, epoch_height);

            let genesis = Genesis {
                epoch_height,

                // Dynamic state starts from the third epoch, since we need the prior epoch's root
                // to have the upgraded header with the stake table hash.
                first_epoch_with_dynamic_stake_table: EpochNumber::new(first_epoch + 2),

                stake_table: config
                    .config
                    .known_nodes_with_stake
                    .into_iter()
                    .map(|peer| peer.stake_table_entry)
                    .collect(),

                chain_id,
            };
            LightClient::from_genesis_with_options(db, client, genesis, opt)
        };
        Ok(Self {
            light_client: Arc::pin(Lazy::from_future(init_light_client.boxed())),
        })
    }
}

#[async_trait]
impl<T> Provider<SeqTypes, T> for LightClientProvider
where
    T: fetching::Request<SeqTypes> + 'static,
    LightClient<SqliteStorage, FallbackClient<QueryServiceClient>>: Provider<SeqTypes, T>,
{
    async fn fetch(&self, req: T) -> Option<T::Response> {
        self.light_client.as_ref().get().await.fetch(req).await
    }
}

#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;

#[cfg(test)]
mod api_tests;

#[cfg(test)]
mod test;
