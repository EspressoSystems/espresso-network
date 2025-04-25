//! This module contains all the traits used for building the sequencer types.
//! It also includes some trait implementations that cannot be implemented in an external crate.
use std::{cmp::max, collections::BTreeMap, fmt::Debug, ops::Range, sync::Arc};

use alloy::primitives::U256;
use anyhow::{bail, ensure, Context};
use async_trait::async_trait;
use committable::Commitment;
use futures::{FutureExt, TryFutureExt};
use hotshot::{
    types::{BLSPubKey, EventType},
    HotShotInitializer, InitializerEpochInfo,
};
use hotshot_types::{
    data::{
        vid_disperse::{ADVZDisperseShare, VidDisperseShare2},
        DaProposal, DaProposal2, EpochNumber, QuorumProposal, QuorumProposal2,
        QuorumProposalWrapper, VidCommitment, VidDisperseShare, ViewNumber,
    },
    drb::DrbResult,
    event::{HotShotAction, LeafInfo},
    message::{convert_proposal, Proposal, UpgradeLock},
    simple_certificate::{
        LightClientStateUpdateCertificate, NextEpochQuorumCertificate2, QuorumCertificate,
        QuorumCertificate2, UpgradeCertificate,
    },
    traits::{
        node_implementation::{ConsensusTime, NodeType, Versions},
        storage::Storage,
        ValidatedState as HotShotState,
    },
    utils::{genesis_epoch_from_version, verify_leaf_chain},
    PeerConfig,
};
use indexmap::IndexMap;
use itertools::Itertools;
use serde::{de::DeserializeOwned, Serialize};

use super::{
    impls::NodeState,
    utils::BackoffParams,
    v0_1::{RewardAccount, RewardAccountProof, RewardMerkleCommitment, RewardMerkleTree},
    v0_3::{EventKey, IndexedStake, StakeTableEvent, Validator},
    EpochVersion, SequencerVersions,
};
use crate::{
    v0::impls::ValidatedState, v0_99::ChainConfig, BlockMerkleTree, Event, FeeAccount,
    FeeAccountProof, FeeMerkleCommitment, FeeMerkleTree, Leaf2, NetworkConfig, SeqTypes,
};

#[async_trait]
pub trait StateCatchup: Send + Sync {
    async fn try_fetch_leaves(&self, retry: usize, height: u64) -> anyhow::Result<Vec<Leaf2>>;

    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
        success_threshold: U256,
    ) -> anyhow::Result<Leaf2> {
        self.backoff().retry(
            self, |provider, retry| {
        let stake_table_clone = stake_table.clone();
        async move {
                    let mut chain = provider.try_fetch_leaves(retry, height).await?;
                    chain.sort_by_key(|l| l.view_number());
                    let leaf_chain = chain.into_iter().rev().collect();
                    verify_leaf_chain(
                        leaf_chain,
                        stake_table_clone.clone(),
                        success_threshold,
                        height,
                        &UpgradeLock::<SeqTypes, SequencerVersions<EpochVersion, EpochVersion>>::new()).await
                }.boxed()
            }).await
    }

    /// Try to fetch the given accounts state, failing without retrying if unable.
    async fn try_fetch_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        account: &[FeeAccount],
    ) -> anyhow::Result<FeeMerkleTree>;

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
                    let tree = provider
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
                                "fetching accounts {accounts:?}, height {height}, view {view:?}"
                            ))
                        })?;
                    accounts
                        .iter()
                        .map(|account| {
                            FeeAccountProof::prove(&tree, (*account).into())
                                .context(format!("missing account {account}"))
                                .map(|(proof, _)| proof)
                        })
                        .collect::<anyhow::Result<Vec<FeeAccountProof>>>()
                }
                .boxed()
            })
            .await
    }

    /// Try to fetch and remember the blocks frontier, failing without retrying if unable.
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
                    .map_err(|err| err.context("fetching frontier"))
                    .boxed()
            })
            .await
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig>;

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

    /// Try to fetch the given accounts state, failing without retrying if unable.
    async fn try_fetch_reward_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        account: &[RewardAccount],
    ) -> anyhow::Result<RewardMerkleTree>;

    /// Fetch the given list of accounts, retrying on transient errors.
    async fn fetch_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: Vec<RewardAccount>,
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        self.backoff()
            .retry(self, |provider, retry| {
                let accounts = &accounts;
                async move {
                    let tree = provider
                        .try_fetch_reward_accounts(
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
                                "fetching reward accounts {accounts:?}, height {height}, view {view:?}"
                            ))
                        })?;
                    accounts
                        .iter()
                        .map(|account| {
                            RewardAccountProof::prove(&tree, (*account).into())
                                .context(format!("missing reward account {account}"))
                                .map(|(proof, _)| proof)
                        })
                        .collect::<anyhow::Result<Vec<RewardAccountProof>>>()
                }
                .boxed()
            })
            .await
    }

    fn backoff(&self) -> &BackoffParams;
    fn name(&self) -> String;
}

#[async_trait]
impl<T: StateCatchup + ?Sized> StateCatchup for Box<T> {
    async fn try_fetch_leaves(&self, retry: usize, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        (**self).try_fetch_leaves(retry, height).await
    }

    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
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
    ) -> anyhow::Result<FeeMerkleTree> {
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

    async fn try_fetch_reward_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<RewardMerkleTree> {
        (**self)
            .try_fetch_reward_accounts(
                retry,
                instance,
                height,
                view,
                reward_merkle_tree_root,
                accounts,
            )
            .await
    }

    async fn fetch_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: Vec<RewardAccount>,
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        (**self)
            .fetch_reward_accounts(instance, height, view, reward_merkle_tree_root, accounts)
            .await
    }

    fn backoff(&self) -> &BackoffParams {
        (**self).backoff()
    }

    fn name(&self) -> String {
        (**self).name()
    }
}

#[async_trait]
impl<T: StateCatchup + ?Sized> StateCatchup for Arc<T> {
    async fn try_fetch_leaves(&self, retry: usize, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        (**self).try_fetch_leaves(retry, height).await
    }

    async fn fetch_leaf(
        &self,
        height: u64,
        stake_table: Vec<PeerConfig<SeqTypes>>,
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
    ) -> anyhow::Result<FeeMerkleTree> {
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

    async fn try_fetch_reward_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<RewardMerkleTree> {
        (**self)
            .try_fetch_reward_accounts(
                retry,
                instance,
                height,
                view,
                reward_merkle_tree_root,
                accounts,
            )
            .await
    }

    async fn fetch_reward_accounts(
        &self,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: Vec<RewardAccount>,
    ) -> anyhow::Result<Vec<RewardAccountProof>> {
        (**self)
            .fetch_reward_accounts(instance, height, view, reward_merkle_tree_root, accounts)
            .await
    }

    fn backoff(&self) -> &BackoffParams {
        (**self).backoff()
    }

    fn name(&self) -> String {
        (**self).name()
    }
}

/// Catchup from multiple providers tries each provider in a round robin fashion until it succeeds.
#[async_trait]
impl<T: StateCatchup> StateCatchup for Vec<T> {
    async fn try_fetch_leaves(&self, retry: usize, height: u64) -> anyhow::Result<Vec<Leaf2>> {
        for provider in self {
            match provider.try_fetch_leaves(retry, height).await {
                Ok(leaves) => return Ok(leaves),
                Err(err) => {
                    tracing::info!(
                        provider = provider.name(),
                        "failed to fetch leaves: {err:#}"
                    );
                },
            }
        }

        bail!("could not fetch leaf chain for height={height:?} from any provider");
    }
    #[tracing::instrument(skip(self, instance))]
    async fn try_fetch_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        fee_merkle_tree_root: FeeMerkleCommitment,
        accounts: &[FeeAccount],
    ) -> anyhow::Result<FeeMerkleTree> {
        for provider in self {
            match provider
                .try_fetch_accounts(
                    retry,
                    instance,
                    height,
                    view,
                    fee_merkle_tree_root,
                    accounts,
                )
                .await
            {
                Ok(tree) => return Ok(tree),
                Err(err) => {
                    tracing::info!(
                        ?accounts,
                        provider = provider.name(),
                        "failed to fetch accounts: {err:#}"
                    );
                },
            }
        }

        bail!("could not fetch account from any provider");
    }

    #[tracing::instrument(skip(self, instance, mt))]
    async fn try_remember_blocks_merkle_tree(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        mt: &mut BlockMerkleTree,
    ) -> anyhow::Result<()> {
        for provider in self {
            match provider
                .try_remember_blocks_merkle_tree(retry, instance, height, view, mt)
                .await
            {
                Ok(()) => return Ok(()),
                Err(err) => {
                    tracing::info!(
                        provider = provider.name(),
                        "failed to fetch frontier: {err:#}"
                    );
                },
            }
        }

        bail!("could not fetch account from any provider");
    }

    async fn try_fetch_chain_config(
        &self,
        retry: usize,
        commitment: Commitment<ChainConfig>,
    ) -> anyhow::Result<ChainConfig> {
        for provider in self {
            match provider.try_fetch_chain_config(retry, commitment).await {
                Ok(cf) => return Ok(cf),
                Err(err) => {
                    tracing::info!(
                        provider = provider.name(),
                        "failed to fetch chain config: {err:#}"
                    );
                },
            }
        }

        bail!("could not fetch chain config from any provider");
    }

    #[tracing::instrument(skip(self, instance))]
    async fn try_fetch_reward_accounts(
        &self,
        retry: usize,
        instance: &NodeState,
        height: u64,
        view: ViewNumber,
        reward_merkle_tree_root: RewardMerkleCommitment,
        accounts: &[RewardAccount],
    ) -> anyhow::Result<RewardMerkleTree> {
        for provider in self {
            match provider
                .try_fetch_reward_accounts(
                    retry,
                    instance,
                    height,
                    view,
                    reward_merkle_tree_root,
                    accounts,
                )
                .await
            {
                Ok(tree) => return Ok(tree),
                Err(err) => {
                    tracing::info!(
                        ?accounts,
                        provider = provider.name(),
                        "failed to fetch reward accounts: {err:#}"
                    );
                },
            }
        }

        bail!("could not fetch account from any provider");
    }

    fn backoff(&self) -> &BackoffParams {
        // Use whichever provider's backoff is most conservative.
        self.iter()
            .map(|p| p.backoff())
            .max()
            .expect("provider list not empty")
    }

    fn name(&self) -> String {
        format!("[{}]", self.iter().map(StateCatchup::name).join(","))
    }
}

#[async_trait]
pub trait PersistenceOptions: Clone + Send + Sync + Debug + 'static {
    type Persistence: SequencerPersistence + MembershipPersistence;

    fn set_view_retention(&mut self, view_retention: u64);
    async fn create(&mut self) -> anyhow::Result<Self::Persistence>;
    async fn reset(self) -> anyhow::Result<()>;
}

#[async_trait]
/// Trait used by `Memberships` implementations to interact with persistence layer.
pub trait MembershipPersistence: Send + Sync + 'static {
    /// Load stake table for epoch from storage
    async fn load_stake(
        &self,
        epoch: EpochNumber,
    ) -> anyhow::Result<Option<IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>>>;

    /// Load stake tables for storage for latest `n` known epochs
    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>>;

    /// Store stake table at `epoch` in the persistence layer
    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: IndexMap<alloy::primitives::Address, Validator<BLSPubKey>>,
    ) -> anyhow::Result<()>;

    async fn store_events(
        &self,
        l1_block: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()>;
    async fn load_events(&self) -> anyhow::Result<Option<(u64, Vec<(EventKey, StakeTableEvent)>)>>;
}

#[async_trait]
pub trait SequencerPersistence: Sized + Send + Sync + Clone + 'static {
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
    ) -> anyhow::Result<Option<LightClientStateUpdateCertificate<SeqTypes>>>;

    /// Load the latest known consensus state.
    ///
    /// Returns an initializer to resume HotShot from the latest saved state (or start from genesis,
    /// if there is no saved state). Also returns the anchor view number, which can be used as a
    /// reference point to process any events which were not processed before a previous shutdown,
    /// if applicable,.
    async fn load_consensus_state<V: Versions>(
        &self,
        state: NodeState,
    ) -> anyhow::Result<(HotShotInitializer<SeqTypes>, Option<ViewNumber>)> {
        let genesis_validated_state = ValidatedState::genesis(&state).0;
        let highest_voted_view = match self
            .load_latest_acted_view()
            .await
            .context("loading last voted view")?
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
        let (leaf, high_qc, anchor_view) = match self
            .load_anchor_leaf()
            .await
            .context("loading anchor leaf")?
        {
            Some((leaf, high_qc)) => {
                tracing::info!(?leaf, ?high_qc, "starting from saved leaf");
                ensure!(
                    leaf.view_number() == high_qc.view_number,
                    format!(
                        "loaded anchor leaf from view {:?}, but high QC is from view {:?}",
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
                    hotshot_types::data::Leaf2::genesis::<V>(&genesis_validated_state, &state)
                        .await,
                    QuorumCertificate2::genesis::<V>(&genesis_validated_state, &state).await,
                    None,
                )
            },
        };
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
        let view = max(highest_voted_view, leaf.view_number());
        // TODO:
        let epoch = genesis_epoch_from_version::<V, SeqTypes>();

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

        tracing::info!(
            ?leaf,
            ?view,
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
                start_view: view,
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

    /// Update storage based on an event from consensus.
    async fn handle_event(&self, event: &Event, consumer: &(impl EventConsumer + 'static)) {
        if let EventType::Decide { leaf_chain, qc, .. } = &event.event {
            let Some(LeafInfo { leaf, .. }) = leaf_chain.first() else {
                // No new leaves.
                return;
            };

            // Associate each decided leaf with a QC.
            let chain = leaf_chain.iter().zip(
                // The first (most recent) leaf corresponds to the QC triggering the decide event.
                std::iter::once((**qc).clone())
                    // Moving backwards in the chain, each leaf corresponds with the subsequent
                    // leaf's justify QC.
                    .chain(leaf_chain.iter().map(|leaf| leaf.leaf.justify_qc())),
            );

            if let Err(err) = self
                .append_decided_leaves(leaf.view_number(), chain, consumer)
                .await
            {
                tracing::error!(
                    "failed to save decided leaves, chain may not be up to date: {err:#}"
                );
                return;
            }
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
    async fn append_decided_leaves(
        &self,
        decided_view: ViewNumber,
        leaf_chain: impl IntoIterator<Item = (&LeafInfo<SeqTypes>, QuorumCertificate2<SeqTypes>)> + Send,
        consumer: &(impl EventConsumer + 'static),
    ) -> anyhow::Result<()>;

    async fn load_anchor_leaf(
        &self,
    ) -> anyhow::Result<Option<(Leaf2, QuorumCertificate2<SeqTypes>)>>;
    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()>;
    // TODO: merge these two `append_vid`s
    async fn append_vid2(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare2<SeqTypes>>,
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
    async fn store_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()>;
    async fn migrate_consensus(&self) -> anyhow::Result<()> {
        tracing::warn!("migrating consensus data...");

        self.migrate_anchor_leaf().await?;
        self.migrate_da_proposals().await?;
        self.migrate_vid_shares().await?;
        self.migrate_quorum_proposals().await?;
        self.migrate_quorum_certificates().await?;

        tracing::warn!("consensus storage has been migrated to new types");

        Ok(())
    }

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

    async fn add_drb_result(
        &self,
        epoch: <SeqTypes as NodeType>::Epoch,
        drb_result: DrbResult,
    ) -> anyhow::Result<()>;
    async fn add_epoch_root(
        &self,
        epoch: <SeqTypes as NodeType>::Epoch,
        block_header: <SeqTypes as NodeType>::BlockHeader,
    ) -> anyhow::Result<()>;
    async fn add_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificate<SeqTypes>,
    ) -> anyhow::Result<()>;
}

#[async_trait]
pub trait EventConsumer: Debug + Send + Sync {
    async fn handle_event(&self, event: &Event) -> anyhow::Result<()>;
}

#[async_trait]
impl<T> EventConsumer for Box<T>
where
    T: EventConsumer + ?Sized,
{
    async fn handle_event(&self, event: &Event) -> anyhow::Result<()> {
        (**self).handle_event(event).await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NullEventConsumer;

#[async_trait]
impl EventConsumer for NullEventConsumer {
    async fn handle_event(&self, _event: &Event) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl<P: SequencerPersistence> Storage<SeqTypes> for Arc<P> {
    async fn append_vid(
        &self,
        proposal: &Proposal<SeqTypes, ADVZDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self).append_vid(proposal).await
    }

    async fn append_vid2(
        &self,
        proposal: &Proposal<SeqTypes, VidDisperseShare2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self).append_vid2(proposal).await
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

    async fn update_decided_upgrade_certificate(
        &self,
        decided_upgrade_certificate: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self)
            .store_upgrade_certificate(decided_upgrade_certificate)
            .await
    }

    async fn add_drb_result(
        &self,
        epoch: <SeqTypes as NodeType>::Epoch,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        (**self).add_drb_result(epoch, drb_result).await
    }

    async fn add_epoch_root(
        &self,
        epoch: <SeqTypes as NodeType>::Epoch,
        block_header: <SeqTypes as NodeType>::BlockHeader,
    ) -> anyhow::Result<()> {
        (**self).add_epoch_root(epoch, block_header).await
    }

    async fn update_state_cert(
        &self,
        state_cert: LightClientStateUpdateCertificate<SeqTypes>,
    ) -> anyhow::Result<()> {
        (**self).add_state_cert(state_cert).await
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
