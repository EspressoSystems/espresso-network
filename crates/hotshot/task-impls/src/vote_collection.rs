// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap},
    fmt::Debug,
    future::Future,
    marker::PhantomData,
    sync::Arc,
};

use async_broadcast::Sender;
use async_trait::async_trait;
use either::Either::{Left, Right};
use hotshot_types::{
    epoch_membership::EpochMembership,
    message::UpgradeLock,
    simple_certificate::{
        DaCertificate2, EpochRootQuorumCertificate, NextEpochQuorumCertificate2, QuorumCertificate,
        QuorumCertificate2, TimeoutCertificate2, UpgradeCertificate, ViewSyncCommitCertificate2,
        ViewSyncFinalizeCertificate2, ViewSyncPreCommitCertificate2,
    },
    simple_vote::{
        DaVote2, EpochRootQuorumVote, NextEpochQuorumVote2, QuorumVote, QuorumVote2, TimeoutVote2,
        UpgradeVote, ViewSyncCommitVote2, ViewSyncFinalizeVote2, ViewSyncPreCommitVote2,
    },
    traits::node_implementation::{ConsensusTime, NodeType, Versions},
    utils::EpochTransitionIndicator,
    vote::{
        Certificate, HasViewNumber, LightClientStateUpdateVoteAccumulator, Vote, VoteAccumulator,
    },
};
use hotshot_utils::anytrace::*;

use crate::{events::HotShotEvent, helpers::broadcast_event};

/// Alias for a map of Vote Collectors
pub type VoteCollectorsMap<TYPES, VOTE, CERT, V> =
    BTreeMap<<TYPES as NodeType>::View, VoteCollectionTaskState<TYPES, VOTE, CERT, V>>;

/// Task state for collecting votes of one type and emitting a certificate
pub struct VoteCollectionTaskState<
    TYPES: NodeType,
    VOTE: Vote<TYPES>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment> + Debug,
    V: Versions,
> {
    /// Public key for this node.
    pub public_key: TYPES::SignatureKey,

    /// Membership for voting
    pub membership: EpochMembership<TYPES>,

    /// accumulator handles aggregating the votes
    pub accumulator: Option<VoteAccumulator<TYPES, VOTE, CERT, V>>,

    /// The view which we are collecting votes for
    pub view: TYPES::View,

    /// Node id
    pub id: u64,

    /// Whether we should check if we are the leader when handling a vote
    pub transition_indicator: EpochTransitionIndicator,
}

/// Describes the functions a vote must implement for it to be aggregatable by the generic vote collection task
pub trait AggregatableVote<
    TYPES: NodeType,
    VOTE: Vote<TYPES>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>,
>
{
    /// return the leader for this votes
    ///
    /// # Errors
    /// if the leader cannot be calculated
    fn leader(
        &self,
        membership: &EpochMembership<TYPES>,
    ) -> impl Future<Output = Result<TYPES::SignatureKey>>;

    /// return the Hotshot event for the completion of this CERT
    fn make_cert_event(certificate: CERT, key: &TYPES::SignatureKey) -> HotShotEvent<TYPES>;
}

impl<
        TYPES: NodeType,
        VOTE: Vote<TYPES> + AggregatableVote<TYPES, VOTE, CERT>,
        CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment> + Clone + Debug,
        V: Versions,
    > VoteCollectionTaskState<TYPES, VOTE, CERT, V>
{
    /// Take one vote and accumulate it. Returns either the cert or the updated state
    /// after the vote is accumulated
    ///
    /// # Errors
    /// If are unable to accumulate the vote
    #[allow(clippy::question_mark)]
    pub async fn accumulate_vote(
        &mut self,
        vote: &VOTE,
        event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<CERT>> {
        // TODO create this only once
        ensure!(
            matches!(
                self.transition_indicator,
                EpochTransitionIndicator::InTransition
            ) || vote.leader(&self.membership).await? == self.public_key,
            info!("Received vote for a view in which we were not the leader.")
        );

        ensure!(
            vote.view_number() == self.view,
            error!(
                "Vote view does not match! vote view is {} current view is {}. This vote should not have been passed to this accumulator.",
                *vote.view_number(),
                *self.view
            )
        );

        let accumulator = self.accumulator.as_mut().context(warn!(
            "No accumulator to handle vote with. This shouldn't happen."
        ))?;

        match accumulator.accumulate(vote, self.membership.clone()).await {
            None => Ok(None),
            Some(cert) => {
                tracing::debug!("Certificate Formed! {cert:?}");

                broadcast_event(
                    Arc::new(VOTE::make_cert_event(cert.clone(), &self.public_key)),
                    event_stream,
                )
                .await;
                self.accumulator = None;

                Ok(Some(cert))
            },
        }
    }
}

/// Trait for types which will handle a vote event.
#[async_trait]
pub trait HandleVoteEvent<TYPES, VOTE, CERT>
where
    TYPES: NodeType,
    VOTE: Vote<TYPES> + AggregatableVote<TYPES, VOTE, CERT>,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment> + Debug,
{
    /// Handle a vote event
    ///
    /// # Errors
    /// Returns an error if we fail to handle the vote
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<CERT>>;

    /// Event filter to use for this event
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool;
}

/// Info needed to create a vote accumulator task
pub struct AccumulatorInfo<TYPES: NodeType> {
    /// This nodes Pub Key
    pub public_key: TYPES::SignatureKey,

    /// Membership we are accumulation votes for
    pub membership: EpochMembership<TYPES>,

    /// View of the votes we are collecting
    pub view: TYPES::View,

    /// This nodes id
    pub id: u64,
}

/// Generic function for spawning a vote task.  Returns the event stream id of the spawned task if created
///
/// # Errors
/// If we failed to create the accumulator
///
/// # Panics
/// Calls unwrap but should never panic.
pub async fn create_vote_accumulator<TYPES, VOTE, CERT, V>(
    info: &AccumulatorInfo<TYPES>,
    event: Arc<HotShotEvent<TYPES>>,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    upgrade_lock: UpgradeLock<TYPES, V>,
    transition_indicator: EpochTransitionIndicator,
) -> Result<VoteCollectionTaskState<TYPES, VOTE, CERT, V>>
where
    TYPES: NodeType,
    VOTE: Vote<TYPES>
        + AggregatableVote<TYPES, VOTE, CERT>
        + std::marker::Send
        + std::marker::Sync
        + 'static,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>
        + Debug
        + std::marker::Send
        + std::marker::Sync
        + 'static,
    V: Versions,
    VoteCollectionTaskState<TYPES, VOTE, CERT, V>: HandleVoteEvent<TYPES, VOTE, CERT>,
{
    let new_accumulator = VoteAccumulator {
        vote_outcomes: HashMap::new(),
        signers: HashMap::new(),
        phantom: PhantomData,
        upgrade_lock,
    };

    let mut state = VoteCollectionTaskState::<TYPES, VOTE, CERT, V> {
        membership: info.membership.clone(),
        public_key: info.public_key.clone(),
        accumulator: Some(new_accumulator),
        view: info.view,
        id: info.id,
        transition_indicator,
    };

    state.handle_vote_event(Arc::clone(&event), sender).await?;

    Ok(state)
}

/// A helper function that handles a vote regardless whether it's the first vote in the view or not.
///
/// # Errors
/// If we fail to handle the vote
#[allow(clippy::too_many_arguments)]
pub async fn handle_vote<
    TYPES: NodeType,
    VOTE: Vote<TYPES> + AggregatableVote<TYPES, VOTE, CERT> + Send + Sync + 'static,
    CERT: Certificate<TYPES, VOTE::Commitment, Voteable = VOTE::Commitment>
        + Debug
        + Send
        + Sync
        + 'static,
    V: Versions,
>(
    collectors: &mut VoteCollectorsMap<TYPES, VOTE, CERT, V>,
    vote: &VOTE,
    public_key: TYPES::SignatureKey,
    membership: &EpochMembership<TYPES>,
    id: u64,
    event: &Arc<HotShotEvent<TYPES>>,
    event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
    upgrade_lock: &UpgradeLock<TYPES, V>,
    transition_indicator: EpochTransitionIndicator,
) -> Result<()>
where
    VoteCollectionTaskState<TYPES, VOTE, CERT, V>: HandleVoteEvent<TYPES, VOTE, CERT>,
{
    match collectors.entry(vote.view_number()) {
        Entry::Vacant(entry) => {
            tracing::debug!("Starting vote handle for view {:?}", vote.view_number());
            let info = AccumulatorInfo {
                public_key,
                membership: membership.clone(),
                view: vote.view_number(),
                id,
            };
            let collector = create_vote_accumulator(
                &info,
                Arc::clone(event),
                event_stream,
                upgrade_lock.clone(),
                transition_indicator,
            )
            .await?;

            entry.insert(collector);

            Ok(())
        },
        Entry::Occupied(mut entry) => {
            // handle the vote, and garbage collect if the vote collector is finished
            if entry
                .get_mut()
                .handle_vote_event(Arc::clone(event), event_stream)
                .await?
                .is_some()
            {
                entry.remove();
                *collectors = collectors.split_off(&vote.view_number());
            }

            Ok(())
        },
    }
}

/// Alias for Quorum vote accumulator
type QuorumVoteState<TYPES, V> =
    VoteCollectionTaskState<TYPES, QuorumVote2<TYPES>, QuorumCertificate2<TYPES>, V>;
/// Alias for Quorum vote accumulator
type NextEpochQuorumVoteState<TYPES, V> = VoteCollectionTaskState<
    TYPES,
    NextEpochQuorumVote2<TYPES>,
    NextEpochQuorumCertificate2<TYPES>,
    V,
>;
/// Alias for DA vote accumulator
type DaVoteState<TYPES, V> =
    VoteCollectionTaskState<TYPES, DaVote2<TYPES>, DaCertificate2<TYPES>, V>;
/// Alias for Timeout vote accumulator
type TimeoutVoteState<TYPES, V> =
    VoteCollectionTaskState<TYPES, TimeoutVote2<TYPES>, TimeoutCertificate2<TYPES>, V>;
/// Alias for upgrade vote accumulator
type UpgradeVoteState<TYPES, V> =
    VoteCollectionTaskState<TYPES, UpgradeVote<TYPES>, UpgradeCertificate<TYPES>, V>;
/// Alias for View Sync Pre Commit vote accumulator
type ViewSyncPreCommitState<TYPES, V> = VoteCollectionTaskState<
    TYPES,
    ViewSyncPreCommitVote2<TYPES>,
    ViewSyncPreCommitCertificate2<TYPES>,
    V,
>;
/// Alias for View Sync Commit vote accumulator
type ViewSyncCommitVoteState<TYPES, V> = VoteCollectionTaskState<
    TYPES,
    ViewSyncCommitVote2<TYPES>,
    ViewSyncCommitCertificate2<TYPES>,
    V,
>;
/// Alias for View Sync Finalize vote accumulator
type ViewSyncFinalizeVoteState<TYPES, V> = VoteCollectionTaskState<
    TYPES,
    ViewSyncFinalizeVote2<TYPES>,
    ViewSyncFinalizeCertificate2<TYPES>,
    V,
>;

impl<TYPES: NodeType> AggregatableVote<TYPES, QuorumVote<TYPES>, QuorumCertificate<TYPES>>
    for QuorumVote<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership.leader(self.view_number() + 1).await
    }
    fn make_cert_event(
        certificate: QuorumCertificate<TYPES>,
        _key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::QcFormed(Left(certificate))
    }
}

impl<TYPES: NodeType> AggregatableVote<TYPES, QuorumVote2<TYPES>, QuorumCertificate2<TYPES>>
    for QuorumVote2<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership.leader(self.view_number() + 1).await
    }
    fn make_cert_event(
        certificate: QuorumCertificate2<TYPES>,
        _key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::Qc2Formed(Left(certificate))
    }
}

impl<TYPES: NodeType>
    AggregatableVote<TYPES, NextEpochQuorumVote2<TYPES>, NextEpochQuorumCertificate2<TYPES>>
    for NextEpochQuorumVote2<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        let epoch = membership
            .epoch
            .map(|e| TYPES::Epoch::new(e.saturating_sub(1)));
        membership
            .get_new_epoch(epoch)
            .await?
            .leader(self.view_number() + 1)
            .await
    }
    fn make_cert_event(
        certificate: NextEpochQuorumCertificate2<TYPES>,
        _key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::NextEpochQc2Formed(Left(certificate))
    }
}

impl<TYPES: NodeType> AggregatableVote<TYPES, UpgradeVote<TYPES>, UpgradeCertificate<TYPES>>
    for UpgradeVote<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership.leader(self.view_number()).await
    }
    fn make_cert_event(
        certificate: UpgradeCertificate<TYPES>,
        _key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::UpgradeCertificateFormed(certificate)
    }
}

impl<TYPES: NodeType> AggregatableVote<TYPES, DaVote2<TYPES>, DaCertificate2<TYPES>>
    for DaVote2<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership.leader(self.view_number()).await
    }
    fn make_cert_event(
        certificate: DaCertificate2<TYPES>,
        key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::DacSend(certificate, key.clone())
    }
}

impl<TYPES: NodeType> AggregatableVote<TYPES, TimeoutVote2<TYPES>, TimeoutCertificate2<TYPES>>
    for TimeoutVote2<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership.leader(self.view_number() + 1).await
    }
    fn make_cert_event(
        certificate: TimeoutCertificate2<TYPES>,
        _key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::Qc2Formed(Right(certificate))
    }
}

impl<TYPES: NodeType>
    AggregatableVote<TYPES, ViewSyncCommitVote2<TYPES>, ViewSyncCommitCertificate2<TYPES>>
    for ViewSyncCommitVote2<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership
            .leader(self.date().round + self.date().relay)
            .await
    }
    fn make_cert_event(
        certificate: ViewSyncCommitCertificate2<TYPES>,
        key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::ViewSyncCommitCertificateSend(certificate, key.clone())
    }
}

impl<TYPES: NodeType>
    AggregatableVote<TYPES, ViewSyncPreCommitVote2<TYPES>, ViewSyncPreCommitCertificate2<TYPES>>
    for ViewSyncPreCommitVote2<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership
            .leader(self.date().round + self.date().relay)
            .await
    }
    fn make_cert_event(
        certificate: ViewSyncPreCommitCertificate2<TYPES>,
        key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::ViewSyncPreCommitCertificateSend(certificate, key.clone())
    }
}

impl<TYPES: NodeType>
    AggregatableVote<TYPES, ViewSyncFinalizeVote2<TYPES>, ViewSyncFinalizeCertificate2<TYPES>>
    for ViewSyncFinalizeVote2<TYPES>
{
    async fn leader(&self, membership: &EpochMembership<TYPES>) -> Result<TYPES::SignatureKey> {
        membership
            .leader(self.date().round + self.date().relay)
            .await
    }
    fn make_cert_event(
        certificate: ViewSyncFinalizeCertificate2<TYPES>,
        key: &TYPES::SignatureKey,
    ) -> HotShotEvent<TYPES> {
        HotShotEvent::ViewSyncFinalizeCertificateSend(certificate, key.clone())
    }
}

// Handlers for all vote accumulators
#[async_trait]
impl<TYPES: NodeType, V: Versions>
    HandleVoteEvent<TYPES, QuorumVote2<TYPES>, QuorumCertificate2<TYPES>>
    for QuorumVoteState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<QuorumCertificate2<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::QuorumVoteRecv(vote) => self.accumulate_vote(vote, sender).await,
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::QuorumVoteRecv(_))
    }
}

// Handlers for all vote accumulators
#[async_trait]
impl<TYPES: NodeType, V: Versions>
    HandleVoteEvent<TYPES, NextEpochQuorumVote2<TYPES>, NextEpochQuorumCertificate2<TYPES>>
    for NextEpochQuorumVoteState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<NextEpochQuorumCertificate2<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::QuorumVoteRecv(vote) => {
                // #3967 REVIEW NOTE: Should we error if self.epoch is None?
                self.accumulate_vote(&vote.clone().into(), sender).await
            },
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::QuorumVoteRecv(_))
    }
}

// Handlers for all vote accumulators
#[async_trait]
impl<TYPES: NodeType, V: Versions>
    HandleVoteEvent<TYPES, UpgradeVote<TYPES>, UpgradeCertificate<TYPES>>
    for UpgradeVoteState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<UpgradeCertificate<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::UpgradeVoteRecv(vote) => self.accumulate_vote(vote, sender).await,
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::UpgradeVoteRecv(_))
    }
}

#[async_trait]
impl<TYPES: NodeType, V: Versions> HandleVoteEvent<TYPES, DaVote2<TYPES>, DaCertificate2<TYPES>>
    for DaVoteState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<DaCertificate2<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::DaVoteRecv(vote) => self.accumulate_vote(vote, sender).await,
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::DaVoteRecv(_))
    }
}

#[async_trait]
impl<TYPES: NodeType, V: Versions>
    HandleVoteEvent<TYPES, TimeoutVote2<TYPES>, TimeoutCertificate2<TYPES>>
    for TimeoutVoteState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<TimeoutCertificate2<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::TimeoutVoteRecv(vote) => self.accumulate_vote(vote, sender).await,
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::TimeoutVoteRecv(_))
    }
}

#[async_trait]
impl<TYPES: NodeType, V: Versions>
    HandleVoteEvent<TYPES, ViewSyncPreCommitVote2<TYPES>, ViewSyncPreCommitCertificate2<TYPES>>
    for ViewSyncPreCommitState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<ViewSyncPreCommitCertificate2<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::ViewSyncPreCommitVoteRecv(vote) => {
                self.accumulate_vote(vote, sender).await
            },
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::ViewSyncPreCommitVoteRecv(_))
    }
}

#[async_trait]
impl<TYPES: NodeType, V: Versions>
    HandleVoteEvent<TYPES, ViewSyncCommitVote2<TYPES>, ViewSyncCommitCertificate2<TYPES>>
    for ViewSyncCommitVoteState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<ViewSyncCommitCertificate2<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::ViewSyncCommitVoteRecv(vote) => self.accumulate_vote(vote, sender).await,
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::ViewSyncCommitVoteRecv(_))
    }
}

#[async_trait]
impl<TYPES: NodeType, V: Versions>
    HandleVoteEvent<TYPES, ViewSyncFinalizeVote2<TYPES>, ViewSyncFinalizeCertificate2<TYPES>>
    for ViewSyncFinalizeVoteState<TYPES, V>
{
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<ViewSyncFinalizeCertificate2<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::ViewSyncFinalizeVoteRecv(vote) => {
                self.accumulate_vote(vote, sender).await
            },
            _ => Ok(None),
        }
    }
    fn filter(event: Arc<HotShotEvent<TYPES>>) -> bool {
        matches!(event.as_ref(), HotShotEvent::ViewSyncFinalizeVoteRecv(_))
    }
}

/// A map for extended quorum vote collectors
pub type EpochRootVoteCollectorsMap<TYPES, V> =
    BTreeMap<<TYPES as NodeType>::View, EpochRootVoteCollectionTaskState<TYPES, V>>;

pub struct EpochRootVoteCollectionTaskState<TYPES: NodeType, V: Versions> {
    /// Public key for this node.
    pub public_key: TYPES::SignatureKey,

    /// Membership for voting
    pub membership: EpochMembership<TYPES>,

    /// accumulator for quorum votes
    pub accumulator:
        Option<VoteAccumulator<TYPES, QuorumVote2<TYPES>, QuorumCertificate2<TYPES>, V>>,

    /// accumulator for light client state update votes
    pub state_vote_accumulator: Option<LightClientStateUpdateVoteAccumulator<TYPES>>,

    /// The view which we are collecting votes for
    pub view: TYPES::View,

    /// The epoch which we are collecting votes for
    pub epoch: Option<TYPES::Epoch>,

    /// Node id
    pub id: u64,
}

// Handlers for extended quorum vote accumulators
impl<TYPES: NodeType, V: Versions> EpochRootVoteCollectionTaskState<TYPES, V> {
    /// Take one vote and accumulate it. Returns the certs once formed.
    async fn handle_vote_event(
        &mut self,
        event: Arc<HotShotEvent<TYPES>>,
        sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<EpochRootQuorumCertificate<TYPES>>> {
        match event.as_ref() {
            HotShotEvent::EpochRootQuorumVoteRecv(vote) => self.accumulate_vote(vote, sender).await,
            _ => Ok(None),
        }
    }

    /// Accumulate a vote and return the certificates if formed
    async fn accumulate_vote(
        &mut self,
        vote: &EpochRootQuorumVote<TYPES>,
        event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
    ) -> Result<Option<EpochRootQuorumCertificate<TYPES>>> {
        let EpochRootQuorumVote { vote, state_vote } = vote;
        ensure!(
            vote.view_number() == self.view,
            error!(
                "Vote view does not match! vote view is {} current view is {}. This vote should not have been passed to this accumulator.",
                *vote.view_number(),
                *self.view
            )
        );

        let accumulator = self.accumulator.as_mut().context(warn!(
            "No accumulator to handle extended quorum vote with. This shouldn't happen."
        ))?;

        let state_vote_accumulator = self.state_vote_accumulator.as_mut().context(warn!(
            "No accumulator to handle light client state update vote with. This shouldn't happen."
        ))?;

        match (
            accumulator.accumulate(vote, self.membership.clone()).await,
            state_vote_accumulator
                .accumulate(&vote.signing_key(), state_vote, &self.membership)
                .await,
        ) {
            (None, None) => Ok(None),
            (Some(cert), Some(state_cert)) => {
                let root_qc = EpochRootQuorumCertificate {
                    qc: cert,
                    state_cert,
                };

                tracing::debug!("Certificate Formed! {:?}", root_qc);

                broadcast_event(
                    Arc::new(HotShotEvent::EpochRootQcFormed(root_qc.clone())),
                    event_stream,
                )
                .await;
                self.accumulator = None;

                Ok(Some(root_qc))
            },
            _ => Err(error!(
                "Only one certificate formed for the epoch root, this should not happen."
            )),
        }
    }
}

async fn create_epoch_root_vote_collection_task_state<TYPES: NodeType, V: Versions>(
    info: &AccumulatorInfo<TYPES>,
    event: Arc<HotShotEvent<TYPES>>,
    sender: &Sender<Arc<HotShotEvent<TYPES>>>,
    upgrade_lock: UpgradeLock<TYPES, V>,
) -> Result<EpochRootVoteCollectionTaskState<TYPES, V>> {
    let new_accumulator =
        VoteAccumulator::<TYPES, QuorumVote2<TYPES>, QuorumCertificate2<TYPES>, V> {
            vote_outcomes: HashMap::new(),
            signers: HashMap::new(),
            phantom: PhantomData,
            upgrade_lock,
        };
    let state_vote_accumulator = LightClientStateUpdateVoteAccumulator {
        vote_outcomes: HashMap::new(),
    };

    let mut state = EpochRootVoteCollectionTaskState::<TYPES, V> {
        membership: info.membership.clone(),
        public_key: info.public_key.clone(),
        accumulator: Some(new_accumulator),
        state_vote_accumulator: Some(state_vote_accumulator),
        view: info.view,
        epoch: info.membership.epoch,
        id: info.id,
    };

    state.handle_vote_event(Arc::clone(&event), sender).await?;

    Ok(state)
}

/// A helper function that handles quorum vote collection for epoch root
///
/// # Errors
/// If we fail to handle the vote
#[allow(clippy::too_many_arguments)]
pub async fn handle_epoch_root_vote<TYPES: NodeType, V: Versions>(
    collectors: &mut EpochRootVoteCollectorsMap<TYPES, V>,
    vote: &EpochRootQuorumVote<TYPES>,
    public_key: TYPES::SignatureKey,
    membership: &EpochMembership<TYPES>,
    id: u64,
    event: &Arc<HotShotEvent<TYPES>>,
    event_stream: &Sender<Arc<HotShotEvent<TYPES>>>,
    upgrade_lock: &UpgradeLock<TYPES, V>,
) -> Result<()> {
    match collectors.entry(vote.view_number()) {
        Entry::Vacant(entry) => {
            tracing::debug!(
                "Starting epoch root quorum vote handle for view {:?}",
                vote.view_number()
            );
            let info = AccumulatorInfo {
                public_key,
                membership: membership.clone(),
                view: vote.view_number(),
                id,
            };
            let collector = create_epoch_root_vote_collection_task_state(
                &info,
                Arc::clone(event),
                event_stream,
                upgrade_lock.clone(),
            )
            .await?;

            entry.insert(collector);

            Ok(())
        },
        Entry::Occupied(mut entry) => {
            // handle the vote, and garbage collect if the vote collector is finished
            if entry
                .get_mut()
                .handle_vote_event(Arc::clone(event), event_stream)
                .await?
                .is_some()
            {
                entry.remove();
                *collectors = collectors.split_off(&vote.view_number());
            }

            Ok(())
        },
    }
}
