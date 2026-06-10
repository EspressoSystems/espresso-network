//! This module contains all the traits used for building the sequencer types.
//! It also includes some trait implementations that cannot be implemented in an external crate.
use std::{cmp::max, collections::BTreeMap, fmt::Debug, ops::Range, sync::Arc};

use alloy::primitives::{Address, U256};
use anyhow::{Context, bail, ensure};
use async_trait::async_trait;
use committable::Commitment;
use futures::{FutureExt, TryFutureExt};
use hotshot::{HotShotInitializer, InitializerEpochInfo, types::EventType};
use hotshot_libp2p_networking::network::behaviours::dht::store::persistent::DhtPersistentStorage;
use hotshot_new_protocol::{message::Certificate2, storage::NewProtocolStorage};
use hotshot_types::{
    data::{
        DaProposal, DaProposal2, EpochNumber, QuorumProposal, QuorumProposal2,
        QuorumProposalWrapper, VidCommitment, VidDisperseShare, ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    event::{HotShotAction, LeafInfo},
    message::{Proposal, convert_proposal},
    new_protocol::CoordinatorEvent,
    simple_certificate::{
        CertificatePair, LightClientStateUpdateCertificateV2, NextEpochQuorumCertificate2,
        QuorumCertificate, QuorumCertificate2, UpgradeCertificate,
    },
    simple_vote,
    stake_table::HSStakeTable,
    traits::{
        EncodeBytes, ValidatedState as HotShotState, metrics::Metrics,
        node_implementation::NodeType, signature_key::SignatureKey, storage::Storage,
    },
    utils::{EpochTransitionIndicator, genesis_epoch_from_version},
    vid::avidm_gf2::AvidmGf2Common,
    vote::HasViewNumber,
};
use indexmap::IndexMap;
use serde::{Serialize, de::DeserializeOwned};
use versions::Upgrade;

use super::{
    impls::NodeState,
    utils::BackoffParams,
    v0_3::{EventKey, IndexedStake, StakeTableEvent},
};
use crate::{
    AuthenticatedValidatorMap, BlockMerkleTree, FeeAccount, FeeAccountProof, FeeMerkleCommitment,
    Leaf2, NetworkConfig, Payload, PubKey, SeqTypes,
    v0::impls::{StakeTableHash, ValidatedState},
    v0_3::{
        ChainConfig, RegisteredValidator, RewardAccountProofV1, RewardAccountV1, RewardAmount,
        RewardMerkleCommitmentV1,
    },
    v0_4::{PermittedRewardMerkleTreeV2, RewardAccountV2, RewardMerkleCommitmentV2},
};

#[async_trait]
pub trait StateCatchup: Send + Sync {
    /// Fetch the leaf at the given height without retrying on transient errors.
    async fn try_fetch_leaf(
        &self,
        retry: usize,
        height: u64,
        stake_table: HSStakeTable<SeqTypes>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2>;

    /// Fetch the leaf at the given height, retrying on transient errors.
    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: HSStakeTable<SeqTypes>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        self.backoff()
            .retry(self, |provider, retry| {
                let stake_table_clone = stake_table.clone();
                async move {
                    provider
                        .try_fetch_leaf(retry, height, stake_table_clone, success_threshold)
                        .await
                }
                .boxed()
            })
            .await
    }

    /// Fetch the given list of accounts without retrying on transient errors.
    async fn try_fetch_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>>;

    /// Fetch the given list of accounts, retrying on transient errors.
    async fn fetch_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: Vec<FeeAccount>,
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        self.backoff()
            .retry(self, |provider, retry| {
                let accounts = &accounts;
                async move {
                    provider
                        .try_fetch_accounts(
                            retry,
                            instance,
                            height,
                            view,
                            fee_merkle_tree_root,
                            accounts,
                        )
                        .await
                        .map_err(|err| {
                            err.context(format!(
                                "fetching accounts {accounts:?}, height {height}, view {view}"
                            ))
                        })
                }
                .boxed()
            })
            .await
    }

    /// Fetch and remember the blocks frontier without retrying on transient errors.
    async fn try_remember_blocks_merkle_tree(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()>;

    /// Fetch and remember the blocks frontier, retrying on transient errors.
    async fn remember_blocks_merkle_tree(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        self.backoff()
            .retry(mt, |mt, retry| {
                self.try_remember_blocks_merkle_tree(retry, instance, height, view, mt)
                    .map_err(|err| err.context(format!("fetching frontier using {}", self.name())))
                    .boxed()
            })
            .await
    }

    /// Fetch the chain config without retrying on transient errors.
    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig>;

    /// Fetch the chain config, retrying on transient errors.
    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        self.backoff()
            .retry(self, |provider, retry| {
                provider
                    .try_fetch_chain_config(retry, commitment)
                    .map_err(|err| err.context("fetching chain config"))
                    .boxed()
            })
            .await
    }

    /// Fetch the given reward merkle tree without retrying on transient errors.
    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        retry: usize,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2>;

    async fn fetch_reward_merkle_tree_v2(
        &self,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        self.backoff()
            .retry(self, |provider, retry| {
                let accounts = accounts.clone();
                async move {
                    provider
                        .try_fetch_reward_merkle_tree_v2(
                            retry,
                            height,
                            view,
                            reward_merkle_tree_root,
                            accounts,
                        )
                        .await
                        .map_err(|err| {
                            err.context(format!("fetching reward merkle tree for height {height}"))
                        })
                }
                .boxed()
            })
            .await
    }

    /// Fetch the given list of reward accounts without retrying on transient errors.
    async fn try_fetch_reward_accounts_v1(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<Vec<RewardAccountProofV1>>;

    /// Fetch the given list of reward accounts, retrying on transient errors.
    async fn fetch_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: Vec<RewardAccountV1>,
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        self.backoff()
            .retry(self, |provider, retry| {
                let accounts = &accounts;
                async move {
                    provider
                        .try_fetch_reward_accounts_v1(
                            retry,
                            instance,
                            height,
                            view,
                            reward_merkle_tree_root,
                            accounts,
                        )
                        .await
                        .map_err(|err| {
                            err.context(format!(
                                "fetching v1 reward accounts {accounts:?}, height {height}, view \
                                 {view}"
                            ))
                        })
                }
                .boxed()
            })
            .await
    }

    /// Fetch the state certificate for a given epoch without retrying on transient errors.
    async fn try_fetch_state_cert(
        &self,
        retry: usize,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>>;

    /// Fetch the state certificate for a given epoch, retrying on transient errors.
    async fn fetch_state_cert(
        &self,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        self.backoff()
            .retry(self, |provider, retry| {
                provider
                    .try_fetch_state_cert(retry, epoch)
                    .map_err(|err| err.context(format!("fetching state cert for epoch {epoch}")))
                    .boxed()
            })
            .await
    }

    /// Returns true if the catchup provider is local (e.g. does not make calls to remote resources).
    fn is_local(&self) -> bool;

    /// Returns the backoff parameters for the catchup provider.
    fn backoff(&self) -> &BackoffParams;

    /// Returns the name of the catchup provider.
    fn name(&self) -> String;
}

#[async_trait]
impl<T: StateCatchup + ?Sized> StateCatchup for Arc<T> {
    async fn try_fetch_leaf(
        &self,
        retry: usize,
        height: u64,
        stake_table: HSStakeTable<SeqTypes>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        (**self)
            .try_fetch_leaf(retry, height, stake_table, success_threshold)
            .await
    }

    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: HSStakeTable<SeqTypes>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        (**self)
            .fetch_leaf(height, stake_table, success_threshold)
            .await
    }

    async fn try_fetch_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        (**self)
            .try_fetch_accounts(
                retry,
                instance,
                height,
                view,
                fee_merkle_tree_root,
                accounts,
            )
            .await
    }

    async fn fetch_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: Vec<FeeAccount>,
    ) -> anyhow::Result<Vec<FeeAccountProof>> {
        (**self)
            .fetch_accounts(instance, height, view, fee_merkle_tree_root, accounts)
            .await
    }

    async fn try_remember_blocks_merkle_tree(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        (**self)
            .try_remember_blocks_merkle_tree(retry, instance, height, view, mt)
            .await
    }

    async fn remember_blocks_merkle_tree(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        (**self)
            .remember_blocks_merkle_tree(instance, height, view, mt)
            .await
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        (**self).try_fetch_chain_config(retry, commitment).await
    }

    async fn fetch_chain_config(
        &self,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        (**self).fetch_chain_config(commitment).await
    }

    async fn try_fetch_reward_merkle_tree_v2(
        &self,
        retry: usize,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        (**self)
            .try_fetch_reward_merkle_tree_v2(retry, height, view, reward_merkle_tree_root, accounts)
            .await
    }

    async fn fetch_reward_merkle_tree_v2(
        &self,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV2,
        accounts: Arc<Vec<RewardAccountV2>>,
    ) -> anyhow::Result<PermittedRewardMerkleTreeV2> {
        (**self)
            .fetch_reward_merkle_tree_v2(height, view, reward_merkle_tree_root, accounts)
            .await
    }

    async fn try_fetch_reward_accounts_v1(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: &[RewardAccountV1],
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        (**self)
            .try_fetch_reward_accounts_v1(
                retry,
                instance,
                height,
                view,
                reward_merkle_tree_root,
                accounts,
            )
            .await
    }

    async fn fetch_reward_accounts_v1(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitmentV1,
        accounts: Vec<RewardAccountV1>,
    ) -> anyhow::Result<Vec<RewardAccountProofV1>> {
        (**self)
            .fetch_reward_accounts_v1(instance, height, view, reward_merkle_tree_root, accounts)
            .await
    }

    async fn try_fetch_state_cert(
        &self,
        retry: usize,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        (**self).try_fetch_state_cert(retry, epoch).await
    }

    async fn fetch_state_cert(
        &self,
        epoch: u64,
    ) -> anyhow::Result<LightClientStateUpdateCertificateV2<SeqTypes>> {
        (**self).fetch_state_cert(epoch).await
    }

    fn backoff(&self) -> &BackoffParams {
        (**self).backoff()
    }

    fn name(&self) -> String {
        (**self).name()
    }

    fn is_local(&self) -> bool {
        (**self).is_local()
    }
}

#[async_trait]
pub trait PersistenceOptions: Clone + Send + Sync + Debug + 'static {
    type Persistence: SequencerPersistence + MembershipPersistence;

    fn set_view_retention(&mut self, view_retention: u64);
    async fn create(&mut self) -> anyhow::Result<Self::Persistence>;
    async fn reset(self) -> anyhow::Result<()>;
}

/// Determine the read state based on the queried block range.
// - If the persistence returned events up to the requested block, the read is complete.
/// - Otherwise, indicate that the read is up to the last processed block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventsPersistenceRead {
    Complete,
    UntilL1Block(u64),
}

/// Tuple type for stake table data: (validators, block_reward, stake_table_hash)
pub type StakeTuple = (
    AuthenticatedValidatorMap,
    Option<RewardAmount>,
    Option<StakeTableHash>,
);

#[async_trait]
/// Trait used by `Memberships` implementations to interact with persistence layer.
pub trait MembershipPersistence: Send + Sync + 'static {
    /// Load stake table for epoch from storage
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>>;

    /// Load stake tables for storage for latest `n` known epochs
    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>>;

    /// Store stake table at `epoch` in the persistence layer
    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()>;

    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()>;
    async fn load_events(
        &self,
        from_l1_block: u64,
        l1_finalized: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )>;

    /// Delete all stake table events, the L1 block tracker, and the epoch DRB and root data.
    async fn delete_stake_tables(&self) -> anyhow::Result<()>;

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: IndexMap<Address, RegisteredValidator<PubKey>>,
    ) -> anyhow::Result<()>;

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>>;
}

#[async_trait]
pub trait SequencerPersistence:
    Sized + Send + Sync + Clone + 'static + DhtPersistentStorage + MembershipPersistence
{
    async fn migrate_reward_merkle_tree_v2(&self) -> anyhow::Result<()>;

    /// Use this storage as a state catchup backend, if supported.
    fn into_catchup_provider(
        self,
        _backoff: BackoffParams,
    ) -> anyhow::Result<Arc<dyn StateCatchup>> {
        bail!("state catchup is not implemented for this persistence type");
    }

    /// Load the orchestrator config from storage.
    ///
    /// Returns `None` if no config exists (we are joining a network for the first time). Fails with
    /// `Err` if it could not be determined whether a config exists or not.
    async fn load_config(&self) -> anyhow::Result<Option<NetworkConfig>>;

    /// Save the orchestrator config to storage.
    async fn save_config(&self, cfg: &NetworkConfig) -> anyhow::Result<()>;

    /// Load the highest view saved with [`save_voted_view`](Self::save_voted_view).
    async fn load_latest_acted_view(&self) -> anyhow::Result<Option<ViewNumber>>;

    /// Load the view to restart from.
    async fn load_restart_view(&self) -> anyhow::Result<Option<ViewNumber>>;

    /// Load the proposals saved by consensus
    async fn load_quorum_proposals(
        &self,
    ) -> anyhow::Result<BTreeMap<ViewNumber, Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>>;

    async fn load_quorum_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>>;

    async fn load_vid_share(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, VidDisperseShare<SeqTypes>>>>;
    async fn load_da_proposal(
        &self,
        view: ViewNumber,
    ) -> anyhow::Result<Option<Proposal<SeqTypes, DaProposal2<SeqTypes>>>>;
    async fn load_upgrade_certificate(
        &self,
    ) -> anyhow::Result<Option<UpgradeCertificate<SeqTypes>>>;
    async fn load_start_epoch_info(&self) -> anyhow::Result<Vec<InitializerEpochInfo<SeqTypes>>>;
    async fn load_state_cert(
        &self,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>>;

    /// Get a state certificate for an epoch.
    async fn get_state_cert_by_epoch(
        &self,
        epoch: u64,
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificateV2<SeqTypes>>>;

    /// Insert a state certificate for a given epoch.
    async fn insert_state_cert(
        &self,
        epoch: u64,
        cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()>;

    /// Load the latest known consensus state.
    ///
    /// Returns an initializer to resume HotShot from the latest saved state (or start from genesis,
    /// if there is no saved state). Also returns the anchor view number, which can be used as a
    /// reference point to process any events which were not processed before a previous shutdown,
    /// if applicable,.
    async fn load_consensus_state(
        &self,
        state: NodeState,
        upgrade: Upgrade,
    ) -> anyhow::Result<(HotShotInitializer<SeqTypes>, Option<ViewNumber>)> {
        let genesis_validated_state = ValidatedState::genesis(&state).0;
        let highest_voted_view = match self
            .load_latest_acted_view()
            .await
            .context("loading last voted view")?
        {
            Some(view) => {
                tracing::info!(?view, "starting with last actioned view");
                view
            },
            None => {
                tracing::info!("no saved view, starting from genesis");
                ViewNumber::genesis()
            },
        };

        let restart_view = match self
            .load_restart_view()
            .await
            .context("loading restart view")?
        {
            Some(view) => {
                tracing::info!(?view, "starting from saved view");
                view
            },
            None => {
                tracing::info!("no saved view, starting from genesis");
                ViewNumber::genesis()
            },
        };
        let next_epoch_high_qc = self
            .load_next_epoch_quorum_certificate()
            .await
            .context("loading next epoch qc")?;
        let (leaf, mut high_qc, anchor_view) = match self
            .load_anchor_leaf()
            .await
            .context("loading anchor leaf")?
        {
            Some((leaf, high_qc)) => {
                tracing::info!(?leaf, ?high_qc, "starting from saved leaf");
                ensure!(
                    leaf.view_number() == high_qc.view_number,
                    format!(
                        "loaded anchor leaf from view {}, but high QC is from view {}",
                        leaf.view_number(),
                        high_qc.view_number
                    )
                );

                let anchor_view = leaf.view_number();
                (leaf, high_qc, Some(anchor_view))
            },
            None => {
                tracing::info!("no saved leaf, starting from genesis leaf");
                (
                    hotshot_types::data::Leaf2::genesis(
                        &genesis_validated_state,
                        &state,
                        upgrade.base,
                    )
                    .await,
                    QuorumCertificate2::genesis(&genesis_validated_state, &state, upgrade).await,
                    None,
                )
            },
        };

        if let Some((extended_high_qc, _)) = self.load_eqc().await
            && extended_high_qc.view_number() > high_qc.view_number()
        {
            high_qc = extended_high_qc
        }

        let validated_state = if leaf.block_header().height() == 0 {
            // If we are starting from genesis, we can provide the full state.
            genesis_validated_state
        } else {
            // Otherwise, we will have to construct a sparse state and fetch missing data during
            // catchup.
            ValidatedState::from_header(leaf.block_header())
        };

        // If we are not starting from genesis, we start from the view following the maximum view
        // between `highest_voted_view` and `leaf.view_number`. This prevents double votes from
        // starting in a view in which we had already voted before the restart, and prevents
        // unnecessary catchup from starting in a view earlier than the anchor leaf.
        let restart_view = max(restart_view, leaf.view_number());
        // TODO:
        let epoch = genesis_epoch_from_version(upgrade.base);

        let config = self.load_config().await.context("loading config")?;
        let epoch_height = config
            .as_ref()
            .map(|c| c.config.epoch_height)
            .unwrap_or_default();
        let epoch_start_block = config
            .as_ref()
            .map(|c| c.config.epoch_start_block)
            .unwrap_or_default();

        let saved_proposals = self
            .load_quorum_proposals()
            .await
            .context("loading saved proposals")?;

        let upgrade_certificate = self
            .load_upgrade_certificate()
            .await
            .context("loading upgrade certificate")?;

        let start_epoch_info = self
            .load_start_epoch_info()
            .await
            .context("loading start epoch info")?;

        let state_cert = self
            .load_state_cert()
            .await
            .context("loading light client state update certificate")?;

        tracing::warn!(
            ?leaf,
            ?restart_view,
            ?epoch,
            ?high_qc,
            ?validated_state,
            ?state_cert,
            "loaded consensus state"
        );

        Ok((
            HotShotInitializer {
                instance_state: state,
                epoch_height,
                epoch_start_block,
                anchor_leaf: leaf,
                anchor_state: Arc::new(validated_state),
                anchor_state_delta: None,
                start_view: restart_view,
                start_epoch: epoch,
                last_actioned_view: highest_voted_view,
                saved_proposals,
                high_qc,
                next_epoch_high_qc,
                decided_upgrade_certificate: upgrade_certificate,
                undecided_leaves: Default::default(),
                undecided_state: Default::default(),
                saved_vid_shares: Default::default(), // TODO: implement saved_vid_shares
                start_epoch_info,
                state_cert,
            },
            anchor_view,
        ))
    }

    /// Decode a consensus decide event and persist its leaves, for the consensus event loop. This
    /// is the persist-only half of a decide; query-service ingestion and GC are deferred to
    /// [`process_decided_events`](Self::process_decided_events). On a decide, returns a
    /// [`PendingDecide`] (carrying the in-memory decide data) to wake that background task;
    /// `None` otherwise. Tests wanting synchronous persist-then-process use
    /// [`append_decided_leaves`](Self::append_decided_leaves).
    async fn persist_event(
        &self,
        event: &CoordinatorEvent<SeqTypes>,
        consumer: &(impl EventConsumer + 'static),
    ) -> Option<PendingDecide> {
        match event {
            CoordinatorEvent::LegacyEvent(hotshot_event) => {
                let EventType::Decide {
                    leaf_chain,
                    committing_qc,
                    deciding_qc,
                    ..
                } = &hotshot_event.event
                else {
                    return None;
                };
                let LeafInfo { leaf, .. } = leaf_chain.first()?;
                let decided_view = leaf.view_number();

                let chain = leaf_chain.iter().zip(
                    std::iter::once((**committing_qc).clone()).chain(
                        leaf_chain
                            .iter()
                            .map(|leaf| CertificatePair::for_parent(&leaf.leaf)),
                    ),
                );

                if let Err(err) = self
                    .persist_decided_leaves(decided_view, chain, deciding_qc.clone(), consumer)
                    .await
                {
                    tracing::error!(
                        "failed to save decided leaves, chain may not be up to date: {err:#}"
                    );
                    return None;
                }
                Some(PendingDecide {
                    view: decided_view,
                    deciding_qc: deciding_qc.clone(),
                    data: Arc::new(DecideEventData::new(leaf_chain.iter(), None)),
                })
            },
            CoordinatorEvent::NewDecide {
                leaf_infos,
                cert1,
                cert2,
            } => {
                let first = leaf_infos.first()?;
                let decided_view = first.leaf.view_number();

                // `cert1` certifies the newest leaf; each newer leaf's justify_qc certifies the
                // next older leaf.
                let certifying_qcs = std::iter::once(cert1.clone())
                    .chain(leaf_infos.iter().map(|info| info.leaf.justify_qc()))
                    .take(leaf_infos.len())
                    .map(CertificatePair::non_epoch_change);

                if let Err(err) = self
                    .persist_decided_leaves(
                        decided_view,
                        leaf_infos.iter().zip(certifying_qcs),
                        None,
                        consumer,
                    )
                    .await
                {
                    tracing::error!(
                        "failed to save decided leaves from new protocol, chain may not be up to \
                         date: {err:#}"
                    );
                    return None;
                }
                Some(PendingDecide {
                    view: decided_view,
                    deciding_qc: None,
                    data: Arc::new(DecideEventData::new(
                        leaf_infos.iter(),
                        // `cert2` certifies the newest decided leaf.
                        cert2.clone().map(|cert2| (decided_view, cert2)),
                    )),
                })
            },
            _ => None,
        }
    }

    /// Append decided leaves to persistent storage and emit a corresponding event.
    ///
    /// `consumer` will be sent a `Decide` event containing all decided leaves in persistent storage
    /// up to and including `view`. If available in persistent storage, full block payloads and VID
    /// info will also be included for each leaf.
    ///
    /// Once the new decided leaves have been processed, old data up to `view` will be garbage
    /// collected The consumer's handling of this event is a prerequisite for the completion of
    /// garbage collection: if the consumer fails to process the event, no data is deleted. This
    /// ensures that, if called repeatedly, all decided leaves ever recorded in consensus storage
    /// will eventually be passed to the consumer.
    ///
    /// Note that the converse is not true: if garbage collection fails, it is not guaranteed that
    /// the consumer hasn't processed the decide event. Thus, in rare cases, some events may be
    /// processed twice, or the consumer may get two events which share a subset of their data.
    /// Thus, it is the consumer's responsibility to make sure its handling of each leaf is
    /// idempotent.
    ///
    /// If the consumer fails to handle the new decide event, it may be retried, or simply postponed
    /// until the next decide, at which point all persisted leaves from the failed GC run will be
    /// included in the event along with subsequently decided leaves.
    ///
    /// This functionality is useful for keeping a separate view of the blockchain in sync with the
    /// consensus storage. For example, the `consumer` could be used for moving data from consensus
    /// storage to long-term archival storage.
    ///
    /// Convenience combinator: [`persist_decided_leaves`](Self::persist_decided_leaves) then
    /// [`process_decided_events`](Self::process_decided_events). Production drives the two halves on
    /// separate tasks; tests and back-compat callers use this synchronous form.
    async fn append_decided_leaves(
        &self,
        decided_view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()> {
        self.persist_decided_leaves(decided_view, leaf_chain, deciding_qc.clone(), consumer)
            .await?;
        // Leaves are persisted; processing failures are non-fatal here and retried in production.
        // No in-memory event data is staged, so this form always exercises the storage path.
        if let Err(err) = self
            .process_decided_events(decided_view, deciding_qc, consumer)
            .await
        {
            tracing::warn!(?decided_view, "decide event processing failed: {err:#}");
        }
        Ok(())
    }

    /// Persist decided leaves only (the critical, must-not-lag half of a decide; also the
    /// anchor for restart recovery). Query-service ingestion and GC are deferred to
    /// [`process_decided_events`](Self::process_decided_events). Backends with no replayable storage
    /// (e.g. `NoStorage`) may instead forward decide events to `consumer` here.
    async fn persist_decided_leaves(
        &self,
        decided_view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, CertificatePair<SeqTypes>)> + Send,
        deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()>;

    /// Write the in-memory data captured from a decide event into the consensus staging stores,
    /// for views whose asynchronous coordinator writes haven't landed yet. The decide processor
    /// calls this before [`process_decided_events`](Self::process_decided_events), so event
    /// generation reads storage only and the captured data survives a restart.
    async fn stage_decide_data(&self, data: &DecideEventData) -> anyhow::Result<()> {
        for (view, (payload, payload_commitment)) in &data.payloads {
            if self.load_da_proposal(*view).await?.is_some() {
                continue;
            }
            let proposal = staged_proposal(DaProposal2 {
                encoded_transactions: payload.encode(),
                metadata: payload.ns_table().clone(),
                view_number: *view,
                // Not recoverable from the capture; staged rows are read back for their
                // payload bytes only.
                epoch: None,
                epoch_transition_indicator: EpochTransitionIndicator::NotInTransition,
            });
            self.append_da2(&proposal, *payload_commitment)
                .await
                .context("staging DA proposal from decide data")?;
        }
        for (view, share) in &data.vid_shares {
            if self.load_vid_share(*view).await?.is_some() {
                continue;
            }
            self.append_vid(&staged_proposal(share.clone()))
                .await
                .context("staging VID share from decide data")?;
        }
        if let Some((view, cert2)) = &data.cert2
            && self.load_cert2(*view).await?.is_none()
        {
            self.append_cert2(*view, cert2.clone())
                .await
                .context("staging cert2 from decide data")?;
        }
        Ok(())
    }

    /// Generate decide events for `consumer` from persisted leaves, then GC processed data.
    /// Cursor-driven (e.g. `last_processed_view`): advances only on success, so it may lag
    /// consensus without losing data.
    ///
    /// All event data is read from storage; the in-memory capture from the decide event is
    /// written to the staging stores up front via [`stage_decide_data`](Self::stage_decide_data),
    /// covering views whose asynchronous coordinator writes haven't landed yet.
    ///
    /// Events are never deferred for missing data: a leaf whose payload is not in storage is
    /// emitted without it and reported in the outcome, so the caller can heal it asynchronously
    /// via peer recovery.
    ///
    /// Returns the cursor (highest view processed, `None` if none) and the payload-less leaves.
    /// Errors propagate; the failed range is retried. The default reports `decided_view` with no
    /// missing payloads, for backends (e.g. `NoStorage`) that forward synchronously in
    /// `persist_decided_leaves`.
    async fn process_decided_events(
        &self,
        decided_view: ViewNumber,
        _deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
        _consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<DecideProcessingOutcome> {
        Ok(DecideProcessingOutcome {
            processed: Some(decided_view),
            missing_payload: vec![],
        })
    }

    async fn load_anchor_leaf(
        &self,
    ) -> anyhow::Result<Option<(Leaf2, QuorumCertificate2<SeqTypes>)>>;
    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()>;
    async fn append_da(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()>;
    async fn record_action(
        &self,
        view: ViewNumber,
        epoch: Option<EpochNumber>,
        action: HotShotAction,
    ) -> anyhow::Result<()>;

    async fn append_quorum_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()>;

    /// Persist cert2 for the given view.
    async fn append_cert2(
        &self,
        _view: ViewNumber,
        _cert2: Certificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// Load a persisted cert2 by view, if any.
    async fn load_cert2(
        &self,
        _view: ViewNumber,
    ) -> anyhow::Result<Option<Certificate2<SeqTypes>>> {
        Ok(None)
    }

    /// Update the current eQC in storage.
    async fn store_eqc(
        &self,
        _high_qc: QuorumCertificate2<SeqTypes>,
        _next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()>;

    /// Load the current eQC from storage.
    async fn load_eqc(
        &self,
    ) -> Option<(
        QuorumCertificate2<SeqTypes>,
        NextEpochQuorumCertificate2<SeqTypes>,
    )>;

    async fn store_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()>;

    async fn migrate_storage(&self) -> anyhow::Result<()> {
        tracing::warn!("migrating consensus data...");

        self.migrate_anchor_leaf().await?;
        self.migrate_da_proposals().await?;
        self.migrate_vid_shares().await?;
        self.migrate_quorum_proposals().await?;
        self.migrate_quorum_certificates().await?;
        self.migrate_reward_merkle_tree_v2()
            .await
            .context("failed to migrate reward merkle tree v2")?;
        self.migrate_x25519_keys().await?;
        tracing::warn!("consensus storage has been migrated to new types");

        Ok(())
    }

    async fn migrate_x25519_keys(&self) -> anyhow::Result<()>;

    async fn migrate_anchor_leaf(&self) -> anyhow::Result<()>;
    async fn migrate_da_proposals(&self) -> anyhow::Result<()>;
    async fn migrate_vid_shares(&self) -> anyhow::Result<()>;
    async fn migrate_quorum_proposals(&self) -> anyhow::Result<()>;
    async fn migrate_quorum_certificates(&self) -> anyhow::Result<()>;

    async fn load_anchor_view(&self) -> anyhow::Result<ViewNumber> {
        match self.load_anchor_leaf().await? {
            Some((leaf, _)) => Ok(leaf.view_number()),
            None => Ok(ViewNumber::genesis()),
        }
    }

    async fn store_next_epoch_quorum_certificate(
        &self,
        high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()>;

    async fn load_next_epoch_quorum_certificate(
        &self,
    ) -> anyhow::Result<Option<NextEpochQuorumCertificate2<SeqTypes>>>;

    async fn append_da2(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()>;

    async fn append_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        self.append_quorum_proposal2(proposal).await
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()>;
    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()>;
    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput>;
    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: <SeqTypes as NodeType>::BlockHeader,
    ) -> anyhow::Result<()>;
    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()>;

    fn enable_metrics(&mut self, metrics: &dyn Metrics);
}

#[async_trait]
pub trait EventConsumer: Debug + Send + Sync {
    async fn handle_event(&self, event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()>;
}

/// Outcome of a decide processing pass
/// ([`process_decided_events`](SequencerPersistence::process_decided_events)).
#[derive(Debug, Default)]
pub struct DecideProcessingOutcome {
    /// Highest view confirmed processed (the cursor), or `None` if nothing was processed.
    pub processed: Option<ViewNumber>,
    /// Leaves whose decide events were emitted without a block payload, in view order.
    /// Candidates for background payload recovery from peers.
    pub missing_payload: Vec<Leaf2>,
}

/// A block payload recovered for a decided leaf, plus the VID common recomputed from it (a
/// deterministic function of the payload), so one recovery heals both.
#[derive(Clone, Debug)]
pub struct RecoveredPayload {
    /// The recovered DA proposal (block payload), verified against the leaf's payload commitment.
    pub proposal: Proposal<SeqTypes, DaProposal2<SeqTypes>>,
    /// VID common recomputed from the recovered payload, consistent with that same commitment.
    pub vid_common: AvidmGf2Common,
}

/// Recover a block payload for a leaf decided without one, from peers (who retain DA proposals
/// for the retention window). Used by the background task that heals the gaps
/// [`process_decided_events`](SequencerPersistence::process_decided_events) reports.
#[async_trait]
pub trait DecidePayloadRecovery: Debug + Send + Sync {
    /// Try to fetch the DA proposal for `leaf`. Implementations MUST verify it against the leaf's
    /// payload commitment; a `Some` result (and its [`RecoveredPayload::vid_common`]) is trusted.
    /// `Ok(None)` means not recovered (may be retried later).
    async fn recover_payload(&self, leaf: &Leaf2) -> anyhow::Result<Option<RecoveredPayload>>;
}

/// Payload, VID, and cert2 data captured in memory from a decide event, keyed by view.
///
/// The new protocol writes DA proposals, VID shares, and cert2s to storage asynchronously, so a
/// view can be decided before its data lands on disk — but the decide event already carries it.
/// The decide processor writes this capture into the staging stores
/// ([`stage_decide_data`](SequencerPersistence::stage_decide_data)) before generating events, so
/// event generation reads storage only and the captured data survives a restart.
#[derive(Clone, Debug, Default)]
pub struct DecideEventData {
    /// Block payloads from the decided leaves, with the header's payload commitment.
    payloads: BTreeMap<ViewNumber, (Payload, VidCommitment)>,
    /// VID shares attached to the decide event.
    vid_shares: BTreeMap<ViewNumber, VidDisperseShare<SeqTypes>>,
    /// The cert2 certifying the newest decided leaf, keyed by the view it certifies.
    cert2: Option<(ViewNumber, Certificate2<SeqTypes>)>,
}

impl DecideEventData {
    /// Capture the in-memory data from a decide event's leaf chain. `cert2`, when present,
    /// is keyed by the view it certifies (the newest decided view).
    pub fn new<'a>(
        leaf_infos: impl IntoIterator<Item = &'a LeafInfo<SeqTypes>>,
        cert2: Option<(ViewNumber, Certificate2<SeqTypes>)>,
    ) -> Self {
        let mut payloads = BTreeMap::new();
        let mut vid_shares = BTreeMap::new();
        for info in leaf_infos {
            let view = info.leaf.view_number();
            if let Some(payload) = info.leaf.block_payload() {
                payloads.insert(
                    view,
                    (payload, info.leaf.block_header().payload_commitment()),
                );
            }
            if let Some(share) = &info.vid_share {
                vid_shares.insert(view, share.clone());
            }
        }
        Self {
            payloads,
            vid_shares,
            cert2,
        }
    }

    /// Whether the capture carries no data at all (e.g. a legacy decide, whose staging writes
    /// are synchronous), so staging can be skipped.
    pub fn is_empty(&self) -> bool {
        self.payloads.is_empty() && self.vid_shares.is_empty() && self.cert2.is_none()
    }
}

/// Wrap `data` in a [`Proposal`] envelope for the staging stores. The signature is vestigial:
/// staging rows are read back for their data only (decide-event fill, peer recovery) and
/// consumers re-verify against the header's payload commitment, never the signature — the
/// coordinator's own storage writer likewise signs staging rows over an empty message.
fn staged_proposal<D: HasViewNumber + simple_vote::HasEpoch + DeserializeOwned>(
    data: D,
) -> Proposal<SeqTypes, D> {
    let (_, privkey) = PubKey::generated_from_seed_indexed([0; 32], 0);
    Proposal {
        data,
        signature: PubKey::sign(&privkey, &[]).expect("signing an empty message cannot fail"),
        _pd: std::marker::PhantomData,
    }
}

/// A decide persisted by [`persist_event`](SequencerPersistence::persist_event) and pending
/// background processing.
#[derive(Clone, Debug)]
pub struct PendingDecide {
    /// The newest decided view.
    pub view: ViewNumber,
    /// The QC deciding `view` (legacy epoch decides only).
    pub deciding_qc: Option<Arc<CertificatePair<SeqTypes>>>,
    /// In-memory data from the decide event, for live query-service ingestion. Shared via
    /// `Arc` so cloning the signal (e.g. out of a `watch` channel) stays cheap.
    pub data: Arc<DecideEventData>,
}

#[async_trait]
impl<T> EventConsumer for Box<T>
where
    T: EventConsumer + ?Sized,
{
    async fn handle_event(&self, event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
        (**self).handle_event(event).await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NullEventConsumer;

#[async_trait]
impl EventConsumer for NullEventConsumer {
    async fn handle_event(&self, _event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl<P: SequencerPersistence> Storage<SeqTypes> for Arc<P> {
    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self).append_vid(proposal).await
    }

    async fn append_da(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        (**self).append_da(proposal, vid_commit).await
    }

    async fn append_da2(
        &self,
        proposal: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        vid_commit: VidCommitment,
    ) -> anyhow::Result<()> {
        (**self).append_da2(proposal, vid_commit).await
    }

    async fn record_action(
        &self,
        view: ViewNumber,
        epoch: Option<EpochNumber>,
        action: HotShotAction,
    ) -> anyhow::Result<()> {
        (**self).record_action(view, epoch, action).await
    }

    async fn update_high_qc(&self, _high_qc: QuorumCertificate<SeqTypes>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn append_proposal(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposal<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self)
            .append_quorum_proposal2(&convert_proposal(proposal.clone()))
            .await
    }

    async fn append_proposal2(
        &self,
        proposal: &Proposal<SeqTypes, QuorumProposal2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        let proposal_qp_wrapper: Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>> =
            convert_proposal(proposal.clone());
        (**self).append_quorum_proposal2(&proposal_qp_wrapper).await
    }

    async fn update_high_qc2(&self, _high_qc: QuorumCertificate2<SeqTypes>) -> anyhow::Result<()> {
        Ok(())
    }

    /// Update the current eQC in storage.
    async fn update_eqc(
        &self,
        high_qc: QuorumCertificate2<SeqTypes>,
        next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        if let Some((existing_high_qc, _)) = (**self).load_eqc().await
            && high_qc.view_number() < existing_high_qc.view_number()
        {
            return Ok(());
        }

        (**self).store_eqc(high_qc, next_epoch_high_qc).await
    }

    async fn update_next_epoch_high_qc2(
        &self,
        _next_epoch_high_qc: NextEpochQuorumCertificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn update_decided_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self)
            .store_upgrade_certificate(decided_upgrade_certificate)
            .await
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        (**self).store_drb_result(epoch, drb_result).await
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: <SeqTypes as NodeType>::BlockHeader,
    ) -> anyhow::Result<()> {
        (**self).store_epoch_root(epoch, block_header).await
    }

    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        (**self).store_drb_input(drb_input).await
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        (**self).load_drb_input(epoch).await
    }

    async fn update_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        (**self).add_state_cert(state_cert).await
    }
}

#[async_trait]
impl<P: SequencerPersistence> NewProtocolStorage<SeqTypes> for Arc<P> {
    async fn append_cert2(
        &self,
        view: ViewNumber,
        cert: Certificate2<SeqTypes>,
    ) -> anyhow::Result<()> {
        (**self).append_cert2(view, cert).await
    }
}

/// Data that can be deserialized from a subslice of namespace payload bytes.
///
/// Companion trait for [`NsPayloadBytesRange`], which specifies the subslice of
/// namespace payload bytes to read.
pub trait FromNsPayloadBytes<'a> {
    /// Deserialize `Self` from namespace payload bytes.
    fn from_payload_bytes(bytes: &'a [u8]) -> Self;
}

/// Specifies a subslice of namespace payload bytes to read.
///
/// Companion trait for [`FromNsPayloadBytes`], which holds data that can be
/// deserialized from that subslice of bytes.
pub trait NsPayloadBytesRange<'a> {
    type Output: FromNsPayloadBytes<'a>;

    /// Range relative to this ns payload
    fn ns_payload_range(&self) -> Range<usize>;
}

/// Types which can be deserialized from either integers or strings.
///
/// Some types can be represented as an integer or a string in human-readable formats like JSON or
/// TOML. For example, 1 GWEI might be represented by the integer `1000000000` or the string `"1
/// gwei"`. Such types can implement `FromStringOrInteger` and then use [`impl_string_or_integer`]
/// to derive this user-friendly serialization.
///
/// These types are assumed to have an efficient representation as an integral type in Rust --
/// [`Self::Binary`] -- and will be serialized to and from this type when using a non-human-readable
/// encoding. With human readable encodings, serialization is always to a string.
pub trait FromStringOrInteger: Sized {
    type Binary: Serialize + DeserializeOwned;
    type Integer: Serialize + DeserializeOwned;

    fn from_binary(b: Self::Binary) -> anyhow::Result<Self>;
    fn from_string(s: String) -> anyhow::Result<Self>;
    fn from_integer(i: Self::Integer) -> anyhow::Result<Self>;

    fn to_binary(&self) -> anyhow::Result<Self::Binary>;
    fn to_string(&self) -> anyhow::Result<String>;
}
