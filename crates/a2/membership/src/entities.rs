//! Membership as a store keeps it: per epoch, the L1 stake-table events, and DRB progress; and
//! [`Persistence`], which makes any such store the persistence the membership coordinator and the
//! stake table fetcher use.

use std::collections::BTreeSet;

use alloy::primitives::Address;
use anyhow::{Context, bail};
use async_trait::async_trait;
use espresso_types::{
    AuthenticatedValidatorMap, Header, PubKey, RegisteredValidatorMap, SeqTypes, StakeTableHash,
    traits::{EventsPersistenceRead, MembershipPersistence, StakeTuple},
    v0_3::{EventKey, IndexedStake, RegisteredValidator, RewardAmount, StakeTableEvent},
};
use hotshot_types::{
    data::{
        DaProposal2, EpochNumber, QuorumProposalWrapper, VidCommitment, VidDisperseShare,
        ViewNumber,
    },
    drb::{DrbInput, DrbResult},
    event::HotShotAction,
    message::Proposal,
    simple_certificate::{LightClientStateUpdateCertificateV2, UpgradeCertificate},
    traits::storage::Storage,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// What a store keeps of the membership of an epoch. Each part is absent until known.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EpochMembership {
    /// The validators that make up the epoch's stake table, the block reward, and the stake
    /// table's hash.
    pub stake: Option<StakeTuple>,

    /// Every validator registered on L1 then, active or not.
    pub validators: Option<RegisteredValidatorMap>,

    pub drb_result: Option<DrbResult>,

    /// The header of the epoch's root block.
    pub root: Option<Header>,
}

/// The L1 stake-table events a store holds in a range of L1 blocks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StakeEvents {
    /// The L1 block up to which the store holds every event, if it holds any.
    pub until: Option<u64>,

    /// The events of the range up to `until`, in L1 order.
    pub events: Vec<(EventKey, StakeTableEvent)>,
}

/// The membership a store keeps.
#[async_trait]
pub trait MembershipEntities: Clone + Send + Sync + 'static {
    /// What the store keeps of `epoch`.
    async fn epoch(&self, epoch: EpochNumber) -> anyhow::Result<EpochMembership>;

    /// The latest `limit` epochs the store keeps anything of, latest first.
    async fn epochs(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, EpochMembership)>>;

    /// The stake tables of the latest `limit` epochs the store keeps one of, latest first.
    async fn latest_stakes(&self, limit: usize) -> anyhow::Result<Vec<(EpochNumber, StakeTuple)>>;

    /// Stores the parts `update` has of `epoch`, keeping the others, in one commit. With a stake
    /// table, it forgets the epochs more than [`RETAINED_EPOCHS`](crate::RETAINED_EPOCHS) before
    /// `epoch`.
    async fn put_epoch(&self, epoch: EpochNumber, update: EpochMembership) -> anyhow::Result<()>;

    /// The events of the L1 blocks from `from` to `to`, both included, that the store holds.
    async fn events(&self, from: u64, to: u64) -> anyhow::Result<StakeEvents>;

    /// Stores `events`, and that every event up to `l1_finalized` is stored, unless the store
    /// holds events up to a later block already.
    async fn put_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()>;

    /// Forgets the events and every epoch's validators, DRB result and root, so that the stake
    /// tables are derived again from L1.
    async fn clear_stake_tables(&self) -> anyhow::Result<()>;

    /// The progress of the DRB computation of `epoch`.
    async fn drb_input(&self, epoch: u64) -> anyhow::Result<Option<DrbInput>>;

    /// Stores the progress of a DRB computation; an error if older than the progress stored.
    async fn put_drb_input(&self, input: DrbInput) -> anyhow::Result<()>;
}

/// The persistence of the membership coordinator and the stake table fetcher over the store `E`.
#[derive(Clone, Debug)]
pub struct Persistence<E>(pub E);

impl<E: MembershipEntities> Persistence<E> {
    async fn put(&self, epoch: EpochNumber, update: EpochMembership) -> anyhow::Result<()> {
        self.0.put_epoch(epoch, update).await
    }
}

#[async_trait]
impl<E: MembershipEntities> MembershipPersistence for Persistence<E> {
    async fn load_stake(&self, epoch: EpochNumber) -> anyhow::Result<Option<StakeTuple>> {
        Ok(self.0.epoch(epoch).await?.stake)
    }

    async fn load_latest_stake(&self, limit: u64) -> anyhow::Result<Option<Vec<IndexedStake>>> {
        let limit = usize::try_from(limit).unwrap_or(usize::MAX);
        let stakes = self.0.latest_stakes(limit).await?;
        Ok(Some(
            stakes
                .into_iter()
                .map(|(epoch, (validators, reward, hash))| (epoch, (validators, reward), hash))
                .collect(),
        ))
    }

    async fn stored_stake_epochs(&self) -> anyhow::Result<BTreeSet<EpochNumber>> {
        let stakes = self.0.latest_stakes(usize::MAX).await?;
        Ok(stakes.into_iter().map(|(epoch, _)| epoch).collect())
    }

    async fn load_drb_result(&self, epoch: EpochNumber) -> anyhow::Result<Option<DrbResult>> {
        Ok(self.0.epoch(epoch).await?.drb_result)
    }

    async fn load_epoch_root(&self, epoch: EpochNumber) -> anyhow::Result<Option<Header>> {
        Ok(self.0.epoch(epoch).await?.root)
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        let update = EpochMembership {
            root: Some(block_header),
            ..Default::default()
        };
        self.put(epoch, update).await
    }

    /// Stores the stake table of `epoch` and forgets the epochs more than
    /// [`RETAINED_EPOCHS`](crate::RETAINED_EPOCHS) before it.
    async fn store_stake(
        &self,
        epoch: EpochNumber,
        stake: AuthenticatedValidatorMap,
        block_reward: Option<RewardAmount>,
        stake_table_hash: Option<StakeTableHash>,
    ) -> anyhow::Result<()> {
        let update = EpochMembership {
            stake: Some((stake, block_reward, stake_table_hash)),
            ..Default::default()
        };
        self.put(epoch, update).await
    }

    async fn store_events(
        &self,
        l1_finalized: u64,
        events: Vec<(EventKey, StakeTableEvent)>,
    ) -> anyhow::Result<()> {
        self.0.put_events(l1_finalized, events).await
    }

    /// The events from `from_l1_block` to `to_l1_block`, and whether the store holds every event
    /// of the range or only up to an earlier block.
    async fn load_events(
        &self,
        from_l1_block: u64,
        to_l1_block: u64,
    ) -> anyhow::Result<(
        Option<EventsPersistenceRead>,
        Vec<(EventKey, StakeTableEvent)>,
    )> {
        let read = self.0.events(from_l1_block, to_l1_block).await?;
        let Some(until) = read.until else {
            return Ok((None, Vec::new()));
        };
        let last = until.min(to_l1_block);
        let complete = match last == to_l1_block {
            true => EventsPersistenceRead::Complete,
            false => EventsPersistenceRead::UntilL1Block(last),
        };
        Ok((Some(complete), read.events))
    }

    async fn delete_stake_tables(&self) -> anyhow::Result<()> {
        self.0.clear_stake_tables().await
    }

    async fn store_all_validators(
        &self,
        epoch: EpochNumber,
        all_validators: IndexMap<Address, RegisteredValidator<PubKey>>,
    ) -> anyhow::Result<()> {
        let update = EpochMembership {
            validators: Some(all_validators),
            ..Default::default()
        };
        self.put(epoch, update).await
    }

    async fn load_all_validators(
        &self,
        epoch: EpochNumber,
        offset: u64,
        limit: u64,
    ) -> anyhow::Result<Vec<RegisteredValidator<PubKey>>> {
        let Some(map) = self.0.epoch(epoch).await?.validators else {
            bail!("no validators stored for epoch {epoch}");
        };
        let mut validators: Vec<_> = map.into_values().collect();
        validators.sort_by_key(|v| v.account);
        Ok(validators
            .into_iter()
            .skip(usize::try_from(offset).unwrap_or(usize::MAX))
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
    }
}

/// The DRB storage of the membership coordinator. Consensus keeps everything else this trait
/// covers in its own tables, so those methods fail.
#[async_trait]
impl<E: MembershipEntities> Storage<SeqTypes> for Persistence<E> {
    async fn append_vid(
        &self,
        _: &Proposal<SeqTypes, VidDisperseShare<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps VID shares in its own tables")
    }

    async fn append_da2(
        &self,
        _: &Proposal<SeqTypes, DaProposal2<SeqTypes>>,
        _: VidCommitment,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps payloads in its own tables")
    }

    async fn append_proposal_wrapper(
        &self,
        _: &Proposal<SeqTypes, QuorumProposalWrapper<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps proposals in its own tables")
    }

    async fn record_action(&self, _: ViewNumber, _: HotShotAction) -> anyhow::Result<()> {
        bail!("consensus keeps its actions in its own tables")
    }

    async fn update_state_cert(
        &self,
        _: LightClientStateUpdateCertificateV2<SeqTypes>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps state certificates in its own tables")
    }

    async fn update_decided_upgrade_certificate(
        &self,
        _: Option<UpgradeCertificate<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!("consensus keeps the upgrade certificate in its own tables")
    }

    async fn store_drb_result(
        &self,
        epoch: EpochNumber,
        drb_result: DrbResult,
    ) -> anyhow::Result<()> {
        let update = EpochMembership {
            drb_result: Some(drb_result),
            ..Default::default()
        };
        self.put(epoch, update).await
    }

    async fn store_epoch_root(
        &self,
        epoch: EpochNumber,
        block_header: Header,
    ) -> anyhow::Result<()> {
        MembershipPersistence::store_epoch_root(self, epoch, block_header).await
    }

    /// Refuses to replace the progress of the same computation with older progress.
    async fn store_drb_input(&self, drb_input: DrbInput) -> anyhow::Result<()> {
        self.0.put_drb_input(drb_input).await
    }

    async fn load_drb_input(&self, epoch: u64) -> anyhow::Result<DrbInput> {
        self.0
            .drb_input(epoch)
            .await?
            .with_context(|| format!("no DRB progress stored for epoch {epoch}"))
    }
}
